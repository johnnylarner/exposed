use std::process::Command;

use clap::Parser;
use exposed::inbound::cli::ExposedCli;
use tempfile::TempDir;

const COMMANDS: &[&[&str]] = &[
    &["util", "latest"],
    &["util", "copy-latest-raw"],
    &["data", "declarations", "fetch"],
    &["data", "declarations", "clean"],
    &["data", "declarations", "resolve"],
    &["data", "declarations", "load"],
    &["data", "members", "fetch"],
    &["data", "members", "load"],
];

#[test]
fn commands_accept_named_config() {
    for command in COMMANDS {
        let mut args = ["exposed"]
            .into_iter()
            .chain(command.iter().copied())
            .chain(["--config", "config.yaml"])
            .collect::<Vec<_>>();
        if *command != ["util", "latest"] {
            args.extend(["--ingestion-key", "00000000-0000-4000-8000-000000000001"]);
        }
        assert!(ExposedCli::try_parse_from(args).is_ok(), "{command:?}");
    }
}

#[test]
fn commands_require_named_config() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    for command in COMMANDS {
        for positional_config in [false, true] {
            let mut process = Command::new(env!("CARGO_BIN_EXE_exposed"));
            process.current_dir(temporary.path()).args(*command);
            if positional_config {
                process.arg("config.yaml");
            }
            let output = process.output()?;
            assert_eq!(output.status.code(), Some(2), "{command:?}");
            assert!(output.stdout.is_empty(), "{command:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            if positional_config {
                assert!(
                    stderr.contains("unexpected argument 'config.yaml'"),
                    "{stderr}"
                );
            } else {
                assert!(stderr.contains("--config <CONFIG>"), "{stderr}");
                assert!(stderr.contains("required arguments"), "{stderr}");
            }
        }
    }
    assert_eq!(std::fs::read_dir(temporary.path())?.count(), 0);
    Ok(())
}

#[test]
fn utility_commands_are_rejected_under_data() {
    for command in ["latest", "copy-latest-raw"] {
        let error =
            ExposedCli::try_parse_from(["exposed", "data", command, "--config", "config.yaml"])
                .err()
                .expect("utilities belong under util");
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }
}

#[test]
fn long_help_documents_only_selected_stage_without_reading_configuration() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    for command in COMMANDS {
        let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
            .current_dir(temporary.path())
            .env_remove("DATABASE_URL")
            .args(*command)
            .args(["--config", "missing.yaml", "--help"])
            .output()?;
        assert!(output.status.success(), "{command:?}");
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout)?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            help.contains("Path to a YAML configuration file."),
            "{help}"
        );
        assert!(
            help.contains("Relative data_dir paths resolve against the current working directory.")
        );
        assert!(help.contains("data_dir (string, required)"), "{help}");
        assert!(!help.contains("JSON schema:"));
        let database = matches!(
            *command,
            ["data", "declarations", "fetch" | "load"] | ["data", "members", "load"]
        );
        assert_eq!(help.contains("DATABASE_URL"), database, "{help}");
        let fetch = command.last() == Some(&"fetch");
        assert_eq!(
            help.contains("batch_size (integer, required)"),
            fetch,
            "{help}"
        );
        let resolve = command.last() == Some(&"resolve");
        assert_eq!(
            help.contains("candidate_budget (integer, optional, default 1000000)"),
            resolve,
            "{help}"
        );
    }
    assert_eq!(std::fs::read_dir(temporary.path())?.count(), 0);
    Ok(())
}

#[test]
fn existing_stages_require_keys_before_reading_files_or_environment() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    for command in COMMANDS
        .iter()
        .filter(|command| command.first() == Some(&"data") && command.last() != Some(&"fetch"))
    {
        let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
            .current_dir(temporary.path())
            .env_remove("DATABASE_URL")
            .args(*command)
            .args(["--config", "missing.yaml"])
            .output()?;
        assert_eq!(output.status.code(), Some(2), "{command:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("--ingestion-key <INGESTION_KEY>"), "{error}");
        assert!(!error.contains("unable to read"));
        assert!(!error.contains("DATABASE_URL must"));
    }
    assert_eq!(std::fs::read_dir(temporary.path())?.count(), 0);
    Ok(())
}

#[test]
fn invalid_keys_report_uuid_errors() {
    let error = ExposedCli::try_parse_from([
        "exposed",
        "data",
        "members",
        "load",
        "--config",
        "missing.yaml",
        "--ingestion-key",
        "bad",
    ])
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("invalid ingestion ID"), "{error}");
    assert!(!error.contains("invalid stage"));
}

#[test]
fn every_yaml_config_error_names_selected_path() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    let path = temporary.path().join("selected.yaml");
    for contents in [
        None,
        Some("data_dir: ["),
        Some("batch_size: wrong\ndata_dir: datasets\n"),
    ] {
        if let Some(contents) = contents {
            std::fs::write(&path, contents)?;
        }
        for command in COMMANDS {
            if contents == Some("batch_size: wrong\ndata_dir: datasets\n")
                && command.last() != Some(&"fetch")
            {
                continue;
            }
            let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
                .current_dir(temporary.path())
                .env_remove("DATABASE_URL")
                .args(*command)
                .arg("--config")
                .arg(&path)
                .args(if *command == ["util", "latest"] {
                    vec![]
                } else {
                    vec!["--ingestion-key", "00000000-0000-4000-8000-000000000001"]
                })
                .output()?;
            assert_eq!(output.status.code(), Some(1), "{command:?}");
            let payload: serde_json::Value = serde_json::from_slice(&output.stderr)?;
            assert!(
                payload["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains(path.to_str().unwrap()),
                "{payload}"
            );
        }
    }
    Ok(())
}

#[test]
fn member_load_missing_artifact_does_not_create_raw_directory() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    let config = temporary.path().join("config.yaml");
    std::fs::write(&config, "data_dir: data\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(temporary.path())
        .env("DATABASE_URL", "postgresql://invalid")
        .args(["data", "members", "load", "--config"])
        .arg(config)
        .args(["--ingestion-key", "00000000-0000-4000-8000-000000000001"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("raw/members.parquet"));
    assert!(!temporary.path().join("data").exists());
    Ok(())
}

#[test]
fn documented_resolver_default_matches_yaml_parser() -> anyhow::Result<()> {
    use exposed::inbound::cli::config::DeclarationResolverConfig;

    let temporary = TempDir::new()?;
    let path = temporary.path().join("config.yaml");
    std::fs::write(&path, "data_dir: datasets\n")?;
    let config = DeclarationResolverConfig::try_from(&path)?;
    assert_eq!(config.candidate_budget, 1_000_000);
    assert_eq!(config.data_dir, std::path::PathBuf::from("datasets"));
    std::fs::write(&path, "candidate_budget: 1\n")?;
    assert!(DeclarationResolverConfig::try_from(&path).is_err());
    std::fs::write(&path, "data_dir: datasets\ncandidate_budget: 0\n")?;
    assert!(DeclarationResolverConfig::try_from(&path).is_err());
    Ok(())
}
