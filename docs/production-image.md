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

All targets share the same release binary and Debian runtime. The default target
is `binary`, which runs `exposed server` on port 6999 as UID and GID 10001.
Build the other targets with `--target frontend-server`, `--target migrate`, or
`--target data`. Use distinct local tags for each target.

```sh
docker buildx build --load -f docker/Dockerfile --target migrate -t exposed:migrate .
docker buildx build --load -f docker/Dockerfile --target frontend-server -t exposed:frontend-server .
docker buildx build --load -f docker/Dockerfile --target data -t exposed:data .
export DATABASE_URL='postgresql://user:password@database:5432/exposed?options=-csearch_path%3Dexposed,public'
docker run --rm --env DATABASE_URL exposed:migrate
docker run --rm --env DATABASE_URL -p 8080:8080 exposed:frontend-server
docker run --rm --env DATABASE_URL -p 6999:6999 exposed:local
```

Include `exposed` and `public` in the database connection search path, as shown
above, so the server can resolve the `pg_trgm` functions in the `exposed` schema.

The migration image runs `exposed migrate run --source /app/db/migrations` and
contains the versioned SQL assets. Run it explicitly before starting either
server. See [database migrations](../db/README.md) for existing database upgrades.

The combined image serves the locked frontend build through nginx on port 8080.
It forwards `/api/` requests to the Rust server after removing `/api` and falls
back to `index.html` for frontend routes. Missing assets return 404. Both services
run as UID and GID 10001. Tini reaps orphan processes, and a Bash supervisor stops
both services on termination or when either service exits. An unexpected service
exit fails the container, including a service that exits with status zero.

The data image runs `exposed data --help` by default. Pass a data subcommand
directly. Mount configuration and writable data directories for ingestion.

```sh
docker run --rm --mount type=bind,src="$PWD/config.yaml",dst=/config.yaml,readonly \
    --mount type=bind,src="$PWD/data",dst=/data \
    exposed:data latest --config /config.yaml
```

Use `data_dir: /data` in the mounted configuration and give UID 10001 access to
the mounted data directory. Pass `--env DATABASE_URL` for data commands that
access PostgreSQL.

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
when queries or migrations change.
No database URL or credentials enter the Docker build.

## Releases from main

The release workflow runs only on pushes to `main` that change Rust, frontend,
migrations, SQLx metadata, Docker, or release inputs. It builds and publishes all
four `linux/amd64` images.
CI uses Docker, `gh`, and `jq`, without host Rust or SQLx installations.

Each push runs independently without cancelling earlier pushes. Before building,
the job atomically creates a Git tag at the exact source SHA using the current
UTC minute in `yyyy.mm.dd.hh.mm` format. If that tag already occupies the minute,
the job waits for the next actual minute. If tag creation fails without an
existing matching tag, the job fails.

All variants use the same allocated version and GHCR repository.

| Target | Published tag | Default command |
| --- | --- | --- |
| `binary` | `VERSION` | `exposed server` |
| `frontend-server` | `VERSION-frontend-server` | Supervisor for nginx and `exposed server` |
| `migrate` | `VERSION-migrate` | `exposed migrate run --source /app/db/migrations` |
| `data` | `VERSION-data` | `exposed data --help` |

The repository is `ghcr.io/<repository-owner>/<repository-name>`. CI pushes each
image directly to its version tag with `mode=max` provenance. The shared BuildKit
builder reuses Rust compilation across all targets.

The GitHub release contains `images.json` with `version`, source commit `source`,
and an `images` object mapping each target to its immutable GHCR pull reference.
The `image.txt` asset keeps the binary image reference for existing deployments.
Use the digest references for deployment.

An interrupted or failed attempt can leave a reserved Git tag or some version
tags without a release. A retry allocates a new unused UTC minute.

The job's Actions token needs repository content and package write permissions.
GHCR package visibility follows the package settings. Set its visibility to public
if unauthenticated pulls are required.
