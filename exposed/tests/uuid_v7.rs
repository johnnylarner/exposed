#[path = "common/response_schema.rs"]
mod response_schema;

use std::{fs, process::Command};

use tempfile::TempDir;
use uuid::{Uuid, Variant, Version};

#[test]
fn generates_fresh_uuid_v7_without_configuration_database_or_files() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let mut generated = Vec::new();
    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_exposed"))
            .current_dir(tmp.path())
            .env("DATABASE_URL", "invalid-url")
            .args(["data", "util", "uuid-v7"])
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let payload: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(payload.as_object().unwrap().len(), 1);
        let uuid = Uuid::parse_str(payload["uuid"].as_str().unwrap())?;
        assert_eq!(uuid.get_version(), Some(Version::SortRand));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
        response_schema::validate(
            &["data", "util", "uuid-v7"],
            "Success JSON schema:",
            &payload,
        );
        generated.push(uuid);
    }
    assert_ne!(generated[0], generated[1]);
    assert_eq!(fs::read_dir(tmp.path())?.count(), 0);
    Ok(())
}
