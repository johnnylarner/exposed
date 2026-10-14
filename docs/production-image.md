# Build and run the production image

Build from the repository root with Docker Buildx:

```sh
docker buildx build --load -f docker/Dockerfile -t exposed:local .
docker run --rm exposed:local --help
```

The build requires no database connection. It uses committed SQLx query metadata,
Rust 1.98.1, and cargo-chef 0.1.78. Dependency compilation uses ordinary Docker
layers so the CI GitHub Actions cache includes compiled dependencies. The default
build parallelism is two jobs. Set `--build-arg CARGO_BUILD_JOBS=4` to change it.

Apply the [database migrations](../db/README.md) before starting the server.
Provide a database URL that the container can reach:

```sh
export DATABASE_URL='postgresql://user:password@database:5432/exposed'
docker run --rm --env DATABASE_URL -p 6999:6999 exposed:local
```

The image runs `exposed server` as UID and GID 10001. It contains the Rust
application, Debian runtime libraries, and CA certificates. It does not include
the frontend or run migrations. Replace the default command to run a data CLI
command. Mount configuration and writable data directories for ingestion.

## Refresh SQLx query metadata

Use an isolated PostgreSQL 18 database. Set `DATABASE_URL` to that database, then
run these commands from the repository root:

```sh
cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features postgres,rustls,sqlx-toml
sqlx migrate run --config sqlx.toml --source db/migrations
cargo sqlx prepare --workspace -- --bin exposed
cargo sqlx prepare --check --workspace -- --bin exposed
```

Commit the resulting `.sqlx` files with query or migration changes. CI checks
these files against a freshly migrated PostgreSQL service before building the
image. No database URL or credentials enter the Docker build.

## Releases from main

A push to `main` that changes Rust sources or shared build, migration, query
metadata, Docker, or release inputs builds and publishes a `linux/amd64` image.
A pull request with those changes validates metadata, builds the image, and runs
its help command and an HTTP search against the empty database.
Pull requests do not publish images or releases.

Each push event has one release identity, its Actions run ID. The publication
script binds that ID to the exact source SHA, a UTC tag in `yyyy.mm.dd.hh.mm`
format, and the image manifest digest. An annotated Git tag stores the binding.
Retries of the same Actions run recover that binding and finish publication.
Different pushes are never cancelled by workflow concurrency. If a minute is
already reserved, publication waits for the next actual UTC minute.

Images use `ghcr.io/<repository-owner>/<repository-name>:<UTC-tag>`. Each GitHub
release contains the source link, a digest-pinned run command, and an `image.txt`
asset with the immutable pull reference. Use that digest reference for deployment.
The `run-<Actions-run-ID>` image tag is temporary build staging, not a deployment
reference. Publication rejects an existing timestamp image with another digest.
A crash after reservation leaves the annotation for a retry. A draft GitHub
release becomes public only after its image reference asset exists.

The repository's Actions token needs package write permission and release write
permission. The publish job alone has repository content write permission.
GHCR package visibility follows the package settings. Set its visibility to public
if unauthenticated pulls are required.

Run the release helper tests with `python3 scripts/test-publish-release.py`.
