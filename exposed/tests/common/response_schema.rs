#![allow(dead_code)]

use serde_json::Value;
use std::process::Command;

pub fn help(command: &[&str]) -> String {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
        .current_dir(temp.path())
        .env("DATABASE_URL", "invalid-url")
        .args(command)
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    String::from_utf8(output.stdout).unwrap()
}

pub fn schema(command: &[&str], label: &str) -> Value {
    let text = help(command);
    let tail = text
        .split_once(label)
        .expect("schema section exists")
        .1
        .trim_start();
    serde_json::Deserializer::from_str(tail)
        .into_iter::<Value>()
        .next()
        .unwrap()
        .unwrap()
}

pub fn validator(command: &[&str], label: &str) -> jsonschema::Validator {
    jsonschema::options()
        .should_validate_formats(true)
        .build(&schema(command, label))
        .unwrap()
}

pub fn validate(command: &[&str], label: &str, payload: &Value) {
    validator(command, label).validate(payload).unwrap();
}
