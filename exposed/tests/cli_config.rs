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
        let args = ["exposed"]
            .into_iter()
            .chain(command.iter().copied())
            .chain(["--config", "config.yaml"]);
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
