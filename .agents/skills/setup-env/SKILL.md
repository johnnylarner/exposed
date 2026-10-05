---
name: setup-env
description: How to setup a new testable dev environment when working on a feature
---

# Setup Env

This skill explains how setup the development environment for the `exposed` project.

## Available scripts

- **`scripts/setup-compose.py — Sets up up the local development environment

## Workflow

1. Create a git worktree with a name related to the task. For example, if your task is to improve the search algorithm, then "feat/improve-search-algo" would be an appropriate name. The worktree should be located in the parent folder of the **main** worktree.

2. Create a docker compose deployment for your worktree. To do this you need to:
    - Run `setup-compose.py` script and initialise the compose stack

3. Setup the application develpment database. We use `sqlx` to manage migrations.
    - Run the sqxl migration commands: `sqlx migrate run`

4. Validate application empty state
    - Use `dbvr` to connect  using the connection string via the `-con` arg. Use the postgres-jdbc driver
    - Assert no members exist: `"SELECT COUNT(*) = 0 FROM exposed.members;"` should be TRUE
    - Assert no declarations exist: `"SELECT COUNT(*) = 0 FROM exposed.declarations;"` should be TRUE

5. Populate members table
    - Use the application CLI for this: `cargo run -- data members fetch`
    - Then ingest the data: `cargo run -- data members load`

6. Assert application state is no longer empty
    -
    - Assert no members exist: `"SELECT COUNT(*) = 649 FROM exposed.members;"` should be TRUE

7. Fetch the declarations data
    - Store an ingestion key: INGESTION_KEY=$(python3 -c "import uuid; print(uuid.uuid7())")
    - Use the application CLI for this: `cargo run -- data declarations fetch --ingestion-key $INGESTION_KEY`
    - 

8. Assert data is there 
    - Retrieve the absolute path of the stored parquet files: `~/../<worktree>/exposed/data/<$INGESTION_KEY>/raw/declarations/*.parquet`;
    - Use `dbvr` with the duckdb in-memory driver: `"SELECT COUNT(*) > 10000 FROM <abosulte_path>"` should return TRUE
