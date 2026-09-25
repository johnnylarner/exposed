//! Exercise real NDJSON pipes and the production Python worker loop.
use crate::imports::adapters::parliament::{CliTransport, Request, Transport};
use std::os::unix::fs::PermissionsExt;

#[tokio::test]
async fn cli_source_reuses_worker_and_excludes_application_secrets() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let script = root
        .join("target")
        .join(format!("api-worker-{}", uuid::Uuid::new_v4()));
    let python = root.join("ingest/.venv/bin/python");
    let code = format!("#!{}\n", python.display())
        + r#"
import os
from exposed.cli import source_worker
class Fixture:
    count = 0
    def fetch(self, request):
        self.count += 1
        return {"payload": {"count": self.count, "operation": request["operation"],
            "secrets": [k for k in os.environ if k in ("DATABASE_URL", "EXPOSED_IMPORT_TOKEN", "WHATSAPP_ACCESS_TOKEN")]},
            "fetched_at": "2026-09-25T00:00:00+00:00", "context": "worker fixture"}
raise SystemExit(source_worker(Fixture()))
"#;
    std::fs::create_dir_all(root.join("target")).unwrap();
    std::fs::write(&script, code).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let source = CliTransport::new(script.clone(), root.join("ingest"));
    let first = source
        .fetch(Request::CurrentMembers { offset: 0 })
        .await
        .unwrap();
    let second = source
        .fetch(Request::CurrentMembers { offset: 100 })
        .await
        .unwrap();
    assert_eq!(first.payload["count"], 1);
    assert_eq!(second.payload["count"], 2);
    assert_eq!(second.payload["secrets"], serde_json::json!([]));
    drop(source);
    std::fs::remove_file(script).unwrap();
}
