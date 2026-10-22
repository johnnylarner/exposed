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
runtime in `docker/Dockerfile`. Its default target is `binary`, which runs
`exposed server` on port 6999 as UID and GID 10001. Build the other backend targets
with `--target migrate` or `--target data`. The independent
`docker/Dockerfile.frontend` contains Node and the SvelteKit build. Its default
target is `frontend-server`. Use distinct local tags for each target.

```sh
docker buildx build --load -f docker/Dockerfile --target migrate -t exposed:migrate .
docker buildx build --load -f docker/Dockerfile.frontend --target frontend-server -t exposed:frontend-server .
docker buildx build --load -f docker/Dockerfile --target data -t exposed:data .
export DATABASE_URL='postgresql://user:password@database:5432/exposed'
docker run --rm --env DATABASE_URL exposed:migrate
docker run --rm --env EXPOSED_API_ORIGIN=http://backend:6999 -p 8080:8080 exposed:frontend-server
docker run --rm --env DATABASE_URL -p 6999:6999 exposed:local
```

The search queries qualify `pg_trgm` functions and operators with the `exposed`
schema, so the connection does not need a custom search path.

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

Automatic releases support `linux/amd64`, including Docker Desktop on Intel
Macs. On Apple Silicon, use the manual ARM test workflow described below or build
locally. Local builds use the Docker builder's default platform.

The containers run inside Docker Desktop's Linux VM. Use the matching immutable
references with the `docker run` commands above. You do not need `--platform` for
native images. Historical multi-platform digest references remain valid and
Docker still selects their matching architecture.

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
migrations, SQLx metadata, Docker, or release inputs, including the shared
`.github/actions/build-images` action. It builds all four variants on a native
`ubuntu-24.04` runner for `linux/amd64`. CI uses Docker, `gh`, and `jq`, without
host Rust or SQLx installations.

Each push runs independently without cancelling earlier pushes. Before building,
the prepare job atomically creates a Git tag at the exact source SHA using the
current UTC minute in `yyyy.mm.dd.hh.mm` format. If that tag already occupies the
minute, the job waits for the next actual minute. If tag creation fails without
an existing matching tag, the job fails.

All variants use the same allocated version and GHCR repository.

| Target | Release tag | Default command |
| --- | --- | --- |
| `binary` | `server-VERSION` | `exposed server` |
| `frontend-server` | `app-VERSION` | `node build` |
| `migrate` | `migrate-VERSION` | `exposed migrate run --source /app/db/migrations` |
| `data` | `cli-VERSION` | `exposed data --help` |

The repository is `ghcr.io/<repository-owner>/<repository-name>`. The shared build
action pushes original image indexes by digest with `mode=max` provenance and
labels the images with the source commit. One BuildKit builder handles all four
variants so backend targets reuse Rust compilation. Builds write their own
`exposed-linux-amd64` or `exposed-linux-arm64` GitHub Actions cache and read the
previous `exposed-linux-multiarch` cache as a fallback.

After the AMD64 build succeeds, the release job copies each original attested
index to its version tag. It updates the `server` alias only after all four
version tags succeed. It creates one GitHub release named `VERSION` and marks it
Latest. ARM builds do not participate in automatic releases.

Each release includes `images.json` with `version`, source commit `source`, and
an `images` object with `server`, `app`, `migrate`, and `cli` keys. Each key maps
to that variant's immutable GHCR pull reference. The `image.txt` asset contains
the server reference for existing deployments. Use these digest references for
deployment.

Manual runs of the release workflow build all four AMD64 variants without
publishing images, reserving Git tags, or creating releases. They populate the
AMD64 cache. Set `cold` to rebuild the Rust toolchain and dependencies without
cached compiler layers. Later backend targets still reuse that build.

An interrupted or failed automatic release can leave a reserved Git tag or some
version tags without a release. A new run allocates a new unused UTC minute.
The prepare and release jobs need repository content write permission. The build
job needs content read and package write permissions. GHCR package visibility
follows the package settings. Set it to public if unauthenticated pulls are
required.

## Build ARM images for local testing

Run **Build ARM test images** from the Actions tab. Select the workflow revision
to use and optionally set `source_ref` to the source branch, tag, or commit to
build. If omitted, it builds the selected workflow commit. Set `cold` to rebuild
Rust dependencies. You can also dispatch it with the GitHub CLI.

```sh
gh workflow run arm-test.yml -f source_ref=main
```

To rebuild a specific commit without cached Rust compiler layers, set `cold=true`.

```sh
gh workflow run arm-test.yml -f source_ref=COMMIT_SHA -f cold=true
```

The workflow runs only when dispatched. A native `ubuntu-24.04-arm` runner uses
the shared action from the selected workflow revision and builds the requested
source in a separate checkout. OCI revision labels and the manifest use the
resolved source commit, even when it differs from the workflow revision.

It publishes four unique test tags in this form.

```text
VARIANT-test-arm64-FULL_SOURCE_SHA-RUN_ID-RUN_ATTEMPT
```

It creates no Git tag or GitHub release and never changes the `server` alias.
The run summary contains the actual source SHA and copyable `docker pull`
commands for all four immutable references. Download the `arm64-test-images`
artifact to obtain `images.json` and `image.txt`. The JSON contains `source`,
`architecture: "arm64"`, and `images` and `tags` objects with the four variant
keys. `image.txt` contains the immutable server reference.

On an Apple Silicon Mac with Docker Desktop, pull a reference from the summary
and use it with the earlier run commands. After you extract the artifact into
the current directory, run:

```sh
docker pull "$(jq -r '.images.server' images.json)"
docker run --rm --env DATABASE_URL -p 6999:6999 "$(cat image.txt)"
```

These test images contain only `linux/arm64`. The newest automatic release
continues to contain only `linux/amd64`.
