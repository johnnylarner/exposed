use std::process::Command;

use clap::Parser;
use exposed::inbound::cli::DataArgs;
use tempfile::TempDir;

const COMMANDS: &[&[&str]] = &[
    &["latest"],
    &["copy-latest-raw"],
    &["declarations", "fetch"],
    &["declarations", "clean"],
    &["declarations", "resolve"],
    &["declarations", "load"],
    &["members", "fetch"],
    &["members", "load"],
];

#[test]
fn commands_accept_global_config_at_each_position() {
    for command in COMMANDS {
        for position in 0..=command.len() {
            for config in [
                vec!["--config", "config.yaml"],
                vec!["--config=config.yaml"],
            ] {
                let args = ["exposed-data"]
                    .into_iter()
                    .chain(command[..position].iter().copied())
                    .chain(config)
                    .chain(command[position..].iter().copied());
                assert!(
                    DataArgs::try_parse_from(args).is_ok(),
                    "{command:?} at {position}"
                );
            }
        }
    }
}

#[test]
fn commands_require_named_config() -> anyhow::Result<()> {
    let temporary = TempDir::new()?;
    for command in COMMANDS {
        for positional_config in [false, true] {
            let mut process = Command::new(env!("CARGO_BIN_EXE_exposed"));
            process
                .current_dir(temporary.path())
                .arg("data")
                .args(*command);
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
                assert!(
                    stderr.contains("required argument was not provided: config"),
                    "{stderr}"
                );
            }
        }
    }
    assert_eq!(std::fs::read_dir(temporary.path())?.count(), 0);
    Ok(())
}
