FROM rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e AS chef
RUN cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY exposed ./exposed
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG CARGO_BUILD_JOBS=2
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY exposed ./exposed
COPY .sqlx ./.sqlx
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked --package exposed --bin exposed
RUN ldd target/release/exposed

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS runtime
RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/exposed /usr/local/bin/exposed
USER 10001:10001

FROM node:24-bookworm-slim@sha256:a5f4366e834695d5f363b920c104953a535d3a9864d1bdf74566a0ebe01daee5 AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build && npm prune --omit=dev

FROM runtime AS frontend-server
USER root
RUN apt-get update \
    && apt-get install --no-install-recommends -y tini bash libstdc++6 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=frontend-builder /usr/local/bin/node /usr/local/bin/node
COPY --chmod=755 docker/frontend-server.sh /usr/local/bin/frontend-server
COPY --from=frontend-builder /app/frontend/build /app/frontend/build
COPY --from=frontend-builder /app/frontend/package.json /app/frontend/package.json
COPY --from=frontend-builder /app/frontend/node_modules /app/frontend/node_modules
ENV PORT=8080 HOST=0.0.0.0 EXPOSED_API_ORIGIN=http://127.0.0.1:6999
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/frontend-server"]
CMD ["exposed", "server"]

FROM runtime AS migrate
COPY db/migrations/ /app/db/migrations/
ENTRYPOINT ["exposed", "migrate"]
CMD ["run", "--source", "/app/db/migrations"]

FROM runtime AS data
ENTRYPOINT ["exposed", "data"]
CMD ["--help"]

FROM runtime AS binary
EXPOSE 6999
ENTRYPOINT ["exposed"]
CMD ["server"]
