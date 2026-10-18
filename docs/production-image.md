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

The `binary`, `migrate`, and `data` targets share the release binary and Debian
runtime. The independent `frontend-server` target contains Node and the SvelteKit
build. The default target is `binary`, which runs `exposed server` on port 6999
as UID and GID 10001.
Build the other targets with `--target frontend-server`, `--target migrate`, or
`--target data`. Use distinct local tags for each target.

```sh
docker buildx build --load -f docker/Dockerfile --target migrate -t exposed:migrate .
docker buildx build --load -f docker/Dockerfile --target frontend-server -t exposed:frontend-server .
docker buildx build --load -f docker/Dockerfile --target data -t exposed:data .
export DATABASE_URL='postgresql://user:password@database:5432/exposed?options=-csearch_path%3Dexposed,public'
docker run --rm --env DATABASE_URL exposed:migrate
docker run --rm --env EXPOSED_API_ORIGIN=http://backend:6999 -p 8080:8080 exposed:frontend-server
docker run --rm --env DATABASE_URL -p 6999:6999 exposed:local
```

Include `exposed` and `public` in the database connection search path, as shown
above, so the server can resolve the `pg_trgm` functions in the `exposed` schema.

The migration image runs `exposed migrate run --source /app/db/migrations` and
contains the versioned SQL assets. Run it explicitly before starting the Rust
server. See [database migrations](../db/README.md) for existing database upgrades.

The frontend image runs the locked SvelteKit build directly through Node on port
8080 as UID and GID 10001. It contains no Rust binary and needs no `DATABASE_URL`.
Deploy the Rust server separately and set the frontend's private
`EXPOSED_API_ORIGIN` environment variable to the backend's reachable origin, such
as `http://backend:6999`. The example above requires `backend` to resolve on a
container network shared by both deployments. Server page loaders call this
origin; browser navigation uses SvelteKit page data. The backend does not need a
public route or an `/api/` ingress rule.

The data image runs `exposed data --help` by default. Pass a data subcommand
directly, including `util` commands. Mount configuration and writable data
directories for ingestion.

```sh
docker run --rm --mount type=bind,src="$PWD/config.yaml",dst=/config.yaml,readonly \
    --mount type=bind,src="$PWD/data",dst=/data \
    exposed:data util latest --config /config.yaml
```

Use `data_dir: /data` in the mounted configuration and give UID 10001 access to
the mounted data directory. Pass `--env DATABASE_URL` for data commands that
access PostgreSQL.

## Run on macOS

New releases publish all four variants for Docker Desktop on Intel and Apple Silicon Macs.
The containers run inside Docker Desktop's Linux VM. Docker selects `linux/amd64`
on Intel Macs and `linux/arm64` on Apple Silicon Macs, including when you use the
pinned digest references from `images.json` or `image.txt`.

Earlier amd64-only releases remain unchanged. After an ARM64 pull fails, update
the image reference to a new release and rerun the migration before the server.

Use the release references with the `docker run` commands above. You do not need
to set `--platform`. Local builds also use the Docker builder's default platform.

If PostgreSQL runs on your Mac, use `host.docker.internal` as the host in
`DATABASE_URL`. PostgreSQL must accept connections from Docker Desktop.

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

The release workflow publishes on pushes to `main` that change Rust, frontend,
migrations, SQLx metadata, Docker, or release inputs. It builds and publishes all
four image variants for both `linux/amd64` and `linux/arm64`.
CI uses Docker, `gh`, and `jq`, without host Rust or SQLx installations.

Each push runs independently without cancelling earlier pushes. Before building,
the job atomically creates a Git tag at the exact source SHA using the current
UTC minute in `yyyy.mm.dd.hh.mm` format. If that tag already occupies the minute,
the job waits for the next actual minute. If tag creation fails without an
existing matching tag, the job fails.

All variants use the same allocated version and GHCR repository.

| Target | Published tag | Default command |
| --- | --- | --- |
| `binary` | `server-VERSION` | `exposed server` |
| `frontend-server` | `app-VERSION` | `node build` |
| `migrate` | `migrate-VERSION` | `exposed migrate run --source /app/db/migrations` |
| `data` | `cli-VERSION` | `exposed data --help` |

The repository is `ghcr.io/<repository-owner>/<repository-name>`. CI pushes each
architecture's images by digest with `mode=max` provenance. Separate native
`ubuntu-24.04` and `ubuntu-24.04-arm` runners build amd64 and arm64 in parallel.
Each runner uses one BuildKit builder for all four variants, so backend targets
reuse Rust compilation. Each architecture writes its own GitHub Actions cache,
`exposed-linux-amd64` or `exposed-linux-arm64`. Both also read the previous
`exposed-linux-multiarch` cache as a fallback.

After both builds succeed, the release job combines their digests into a
multi-platform index for each version tag. It updates the `server` alias only
after all four indexes exist. The indexes retain both architectures' provenance.

Manual `workflow_dispatch` runs build all four variants on the same native
runners without publishing images, reserving Git tags, or creating releases.
They populate the architecture caches and can validate workflow changes on a
branch before release.
Set the manual run's `cold` input to rebuild the Rust toolchain and dependencies
without cached compiler layers. Later backend targets still reuse that build.

The GitHub release contains `images.json` with `version`, source commit `source`,
and an `images` object with `server`, `app`, `migrate`, and `cli` keys. Each key
maps to the corresponding variant's immutable GHCR pull reference. Each reference
pins an OCI image index containing both architectures. Docker selects the matching
image when you pull or run the reference.
The `image.txt` asset keeps the server image reference for existing deployments.
Use the digest references for deployment.

An interrupted or failed attempt can leave a reserved Git tag or some version
tags without a release. A retry allocates a new unused UTC minute.

The prepare job needs repository content write permission to reserve the tag.
Build jobs need package write permission, and the release job needs both.
GHCR package visibility follows the package settings. Set its visibility to public
if unauthenticated pulls are required.
