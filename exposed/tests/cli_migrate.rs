use std::process::{Command, Output};

use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool};
use tempfile::TempDir;

fn command(directory: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_exposed"));
    command.current_dir(directory).arg("migrate");
    command
}

fn failure(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let response: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(response["error"]["message"].as_str().is_some());
    response
}

#[test]
fn migration_help_and_argument_errors_do_not_need_configuration() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    let output = command(temporary.path())
        .env_remove("DATABASE_URL")
        .args(["run", "--help"])
        .output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let help = String::from_utf8(output.stdout)?;
    assert!(help.contains("db/migrations"));
    assert!(help.contains("Success JSON schema:"));
    assert!(help.contains("Failure JSON schema:"));
    assert!(help.contains("\"source\""));
    for args in [vec![], vec!["revert"], vec!["run", "--source"]] {
        let output = command(temporary.path()).args(args).output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    assert_eq!(std::fs::read_dir(temporary.path())?.count(), 0);
    Ok(())
}

#[test]
fn missing_environment_and_source_use_json_failures() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    let output = command(temporary.path())
        .env_remove("DATABASE_URL")
        .arg("run")
        .output()?;
    assert!(
        failure(&output)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("DATABASE_URL must be set")
    );
    let output = command(temporary.path())
        .env("DATABASE_URL", "invalid-url")
        .args(["run", "--source", "missing"])
        .output()?;
    failure(&output);
    Ok(())
}

#[sqlx::test]
async fn runtime_source_runs_up_once_and_keeps_public_ledger(pool: PgPool) -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    let source = temporary.path().join("custom migrations");
    std::fs::create_dir(&source)?;
    std::fs::write(
        source.join("1_probe.up.sql"),
        "CREATE TABLE public.cli_migrate_probe (id INTEGER PRIMARY KEY);",
    )?;
    std::fs::write(
        source.join("1_probe.down.sql"),
        "DROP TABLE public.cli_migrate_probe;",
    )?;
    sqlx::query!("CREATE SCHEMA migration_test")
        .execute(&pool)
        .await?;
    let url = pool
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", "migration_test")])
        .to_url_lossy();
    for _ in 0..2 {
        let output = command(temporary.path())
            .env("DATABASE_URL", url.as_str())
            .args(["run", "--source"])
            .arg(&source)
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout)?,
            json!({"source": source})
        );
    }
    let objects = sqlx::query!(
        "SELECT to_regclass('public.cli_migrate_probe')::text AS probe,
                to_regclass('public._sqlx_migrations')::text AS ledger,
                to_regclass('migration_test._sqlx_migrations')::text AS other_ledger"
    )
    .fetch_one(&pool)
    .await?;
    assert!(objects.probe.is_some());
    assert!(objects.ledger.is_some());
    assert!(objects.other_ledger.is_none());
    std::fs::write(source.join("1_probe.up.sql"), "SELECT 1;")?;
    let output = command(temporary.path())
        .env("DATABASE_URL", url.as_str())
        .args(["run", "--source"])
        .arg(&source)
        .output()?;
    assert!(
        failure(&output)["error"]["message"]
            .as_str()
            .unwrap()
            .contains("previously applied")
    );
    Ok(())
}
