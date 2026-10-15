#[path = "common/response_schema.rs"]
mod response_schema;

use clap::CommandFactory;
use exposed::inbound::cli::{DataArgs, ExposedCli, UtilArgs};
use serde_json::json;
use std::process::Command;

#[test]
fn every_command_documents_responses_without_configuration_or_side_effects() {
    for (group, command_tree) in [("data", DataArgs::command()), ("util", UtilArgs::command())] {
        for command in command_tree
            .get_subcommands()
            .filter(|command| command.get_name() != "help")
        {
            let command = command.get_name();
            let help = response_schema::help(&[group, command]);
            assert!(help.contains("https://json-schema.org/draft/2020-12/schema"));
            assert!(help.contains("Failure JSON schema:"));
            assert!(help.contains("Clap argument errors"));
            let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
                .args([group, command, "-h"])
                .output()
                .unwrap();
            assert!(output.status.success());
            assert!(!String::from_utf8_lossy(&output.stdout).contains("JSON schema:"));
        }
    }
    let help = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .args(["data", "help", "declarations"])
        .env("DATABASE_URL", "invalid-url")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Success JSON schema (load):"));
    for stage in ["fetch", "clean", "resolve", "load"] {
        response_schema::validator(
            &["data", "declarations"],
            &format!("Success JSON schema ({stage}):"),
        );
    }
}

#[test]
fn required_nullable_fields_and_typed_values_reject_malformed_responses() {
    let latest = response_schema::validator(&["util", "latest"], "Success JSON schema:");
    assert!(latest.is_valid(&json!({"ingestion": null})));
    assert!(!latest.is_valid(&json!({})));
    assert!(!latest.is_valid(&json!({"ingestion": {"ingestion_key": "bad", "path": "/data"}})));
    let clean =
        response_schema::validator(&["data", "declarations"], "Success JSON schema (clean):");
    let mut payload = json!({"ingestion_key":"00000000-0000-4000-8000-000000000001", "path":"/data", "member_partitions":1,"declarations":2,"funders":3,"funding_entries":4});
    assert!(clean.is_valid(&payload));
    payload["declarations"] = json!("2");
    assert!(!clean.is_valid(&payload));
    let load = response_schema::validator(&["data", "declarations"], "Success JSON schema (load):");
    let mut payload = json!({"ingestion_key":"00000000-0000-4000-8000-000000000001", "outcome":"imported", "declarations":2,"funders":3,"funding_entries":4});
    assert!(load.is_valid(&payload));
    payload["outcome"] = json!("unknown");
    assert!(!load.is_valid(&payload));
    let fetch =
        response_schema::validator(&["data", "declarations"], "Success JSON schema (fetch):");
    let mut payload = json!({"ingestion_key":"00000000-0000-4000-8000-000000000001", "path":"/data", "stage":"fetch"});
    assert!(fetch.is_valid(&payload));
    payload["stage"] = json!("clean");
    assert!(!fetch.is_valid(&payload));
    let member = response_schema::validator(&["data", "members"], "Success JSON schema:");
    payload["stage"] = json!("load");
    assert!(member.is_valid(&payload));
    payload["stage"] = json!("resolve");
    assert!(!member.is_valid(&payload));
    let error = response_schema::validator(&["util", "latest"], "Failure JSON schema:");
    assert!(!error.is_valid(&json!({"error":{}})));
    assert!(!error.is_valid(&json!({"error":{"message":1}})));
}

#[test]
fn help_groups_pipeline_commands_and_utilities() {
    let mut root = ExposedCli::command();
    let help = root.render_long_help().to_string();
    assert!(
        help.lines()
            .any(|line| line.trim_start().starts_with("util "))
    );
    for (mut tree, expected, absent) in [
        (
            DataArgs::command(),
            &["declarations", "members"][..],
            &["latest", "copy-latest-raw"][..],
        ),
        (
            UtilArgs::command(),
            &["latest", "copy-latest-raw"][..],
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
