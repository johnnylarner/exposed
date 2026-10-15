#[path = "common/response_schema.rs"]
mod response_schema;

use clap::CommandFactory;
use exposed::inbound::cli::{DataArgs, ExposedCli, UtilArgs};
use serde_json::json;
use std::process::Command;

#[test]
fn every_command_documents_responses_without_configuration_or_side_effects() {
    for command in DataArgs::command()
        .get_subcommands()
        .filter(|command| command.get_name() != "help")
    {
        for stage in command
            .get_subcommands()
            .filter(|stage| stage.get_name() != "help")
        {
            let args = ["data", command.get_name(), stage.get_name()];
            let help = response_schema::help(&args);
            assert!(!help.contains("JSON schema:"));
            response_schema::validator(&args, "Success JSON schema:");
            response_schema::validator(&args, "Failure JSON schema:");
            let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
                .args(args)
                .arg("-h")
                .output()
                .unwrap();
            assert!(output.status.success());
            assert!(!String::from_utf8_lossy(&output.stdout).contains("JSON schema:"));
        }
    }
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn required_nullable_fields_and_typed_values_reject_malformed_responses() {
    let latest = response_schema::validator(&["data", "util", "latest"], "Success JSON schema:");
    assert!(latest.is_valid(&json!({"ingestion": null})));
    assert!(!latest.is_valid(&json!({})));
    assert!(!latest.is_valid(&json!({"ingestion": {"ingestion_key": "bad", "path": "/data"}})));
    let clean =
        response_schema::validator(&["data", "declarations", "clean"], "Success JSON schema:");
    let mut payload = json!({"ingestion_key":"00000000-0000-4000-8000-000000000001", "path":"/data", "member_partitions":1,"declarations":2,"funders":3,"funding_entries":4});
    assert!(clean.is_valid(&payload));
    payload["declarations"] = json!("2");
    assert!(!clean.is_valid(&payload));
    let load =
        response_schema::validator(&["data", "declarations", "load"], "Success JSON schema:");
    let mut payload = json!({"ingestion_key":"00000000-0000-4000-8000-000000000001", "outcome":"imported", "declarations":2,"funders":3,"funding_entries":4});
    assert!(load.is_valid(&payload));
    payload["outcome"] = json!("unknown");
    assert!(!load.is_valid(&payload));
    let fetch =
        response_schema::validator(&["data", "declarations", "fetch"], "Success JSON schema:");
    let mut payload = json!({"ingestion_key":"00000000-0000-4000-8000-000000000001", "path":"/data", "stage":"fetch"});
    assert!(fetch.is_valid(&payload));
    payload["stage"] = json!("clean");
    assert!(!fetch.is_valid(&payload));
    let member = response_schema::validator(&["data", "members", "load"], "Success JSON schema:");
    payload["stage"] = json!("load");
    assert!(member.is_valid(&payload));
    payload["stage"] = json!("resolve");
    assert!(!member.is_valid(&payload));
    let error = response_schema::validator(&["data", "util", "latest"], "Failure JSON schema:");
    assert!(!error.is_valid(&json!({"error":{}})));
    assert!(!error.is_valid(&json!({"error":{"message":1}})));
}

#[test]
fn help_groups_pipeline_commands_and_utilities() {
    let mut root = ExposedCli::command();
    let help = root.render_long_help().to_string();
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("util "))
    );
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .args(["util", "uuid-v7"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .args(["data", "util", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for command in ["latest", "copy-latest-raw", "uuid-v7"] {
        assert!(
            help.lines()
                .any(|line| line.trim_start().starts_with(&format!("{command} ")))
        );
    }
    for (mut tree, expected, absent) in [
        (
            DataArgs::command(),
            &["declarations", "members", "util"][..],
            &["latest", "copy-latest-raw", "uuid-v7"][..],
        ),
        (
            UtilArgs::command(),
            &["latest", "copy-latest-raw", "uuid-v7"][..],
            &["declarations", "members"][..],
        ),
    ] {
        let help = tree.render_long_help().to_string();
        for name in expected {
            assert!(
                help.lines()
                    .any(|line| line.trim_start().starts_with(&format!("{name} "))),
                "{help}"
            );
        }
        for name in absent {
            assert!(
                !help
                    .lines()
                    .any(|line| line.trim_start().starts_with(&format!("{name} "))),
                "{help}"
            );
        }
    }
}

#[test]
fn schema_inspection_skips_dotenv_and_async_runtime_startup() -> anyhow::Result<()> {
    let temporary = tempfile::tempdir()?;
    std::fs::write(
        temporary.path().join(".env"),
        "TOKIO_WORKER_THREADS=not-a-number\n",
    )?;
    for command in [
        vec!["data", "declarations", "fetch"],
        vec!["data", "declarations", "clean"],
        vec!["data", "declarations", "resolve"],
        vec!["data", "declarations", "load"],
        vec!["data", "members", "fetch"],
        vec!["data", "members", "load"],
        vec!["data", "util", "latest"],
        vec!["data", "util", "copy-latest-raw"],
        vec!["data", "util", "uuid-v7"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
            .current_dir(temporary.path())
            .env_remove("TOKIO_WORKER_THREADS")
            .args(&command)
            .arg("--schema")
            .output()?;
        assert!(
            output.status.success(),
            "{command:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let document: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert!(document["success"].is_object());
        assert!(document["failure"].is_object());
    }
    Ok(())
}
