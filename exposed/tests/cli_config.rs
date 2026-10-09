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
fn commands_accept_named_config() {
    for command in COMMANDS {
        let args = ["exposed-data"]
            .into_iter()
            .chain(command.iter().copied())
            .chain(["--config", "config.yaml"]);
        assert!(DataArgs::try_parse_from(args).is_ok(), "{command:?}");
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
                assert!(stderr.contains("--config <CONFIG>"), "{stderr}");
                assert!(stderr.contains("required arguments"), "{stderr}");
            }
        }
    }
    assert_eq!(std::fs::read_dir(temporary.path())?.count(), 0);
    Ok(())
}
