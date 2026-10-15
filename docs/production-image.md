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

Commit the resulting `.sqlx` files with query or migration changes. The image
build uses this metadata offline. Refresh and check it against PostgreSQL manually
when queries or migrations change. CI does not start a database or install SQLx.
No database URL or credentials enter the Docker build.

## Releases from main

A push to `main` that changes Rust sources or shared build, migration, query
metadata, Docker, or release inputs builds and publishes a `linux/amd64` image.
A pull request with those changes builds a local `exposed:pr` image and runs its
help command. Pull requests do not publish images or releases. CI uses Docker,
`gh`, and `jq`, with no database or host Rust installation.

Each push runs independently without cancelling earlier pushes. Before building,
the job atomically creates a Git tag at the exact source SHA using the current
UTC minute in `yyyy.mm.dd.hh.mm` format. If that tag already occupies the minute,
the job waits for the next actual minute. If tag creation fails without an
existing matching tag, the job fails.

The build publishes directly to
`ghcr.io/<repository-owner>/<repository-name>:<UTC-tag>`. The job pulls that image
and runs its help command, then uses `softprops/action-gh-release` to create the
GitHub release. Each release contains the source link, tagged image, a
digest-pinned run command, and an `image.txt` asset with the immutable pull
reference. Use that digest reference for deployment.

An interrupted or failed attempt can leave a reserved Git tag or published image
without a release. A retry allocates a new unused UTC minute.

The job's Actions token needs repository content and package write permissions.
GHCR package visibility follows the package settings. Set its visibility to public
if unauthenticated pulls are required.
