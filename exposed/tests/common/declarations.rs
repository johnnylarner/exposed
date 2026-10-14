use std::{
    fs::{self, File},
    path::Path,
};

use arrow::json::ArrayWriter;
use exposed::domain::models::entity_ingestion::IngestionKey;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::Deserialize;

/// The saved funding details that will feed the member and funder views.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct DeclarationFunding {
    pub declaration_id: u32,
    pub parliament_member_id: u32,
    #[serde(flatten)]
    pub funding_entry: FundingEntry,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct FundingEntry {
    pub donor_name: Option<String>,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub payment_type: Option<String>,
    pub funder_kind: Option<String>,
    pub company_number: Option<String>,
}

pub fn read_declaration_funding(
    data_dir: &Path,
    key: &IngestionKey,
) -> anyhow::Result<Vec<DeclarationFunding>> {
    let directory = data_dir.join(key.to_string()).join("raw/declarations");
    let mut json = ArrayWriter::new(Vec::new());
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "parquet")
        {
            let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?;
            let columns = reader
                .schema()
                .fields()
                .iter()
                .map(|f| reader.schema().index_of(f.name()))
                .collect::<Result<Vec<_>, _>>()?;
            for batch in reader.build()? {
                json.write(&batch?.project(&columns)?)?;
            }
        }
    }
    json.finish()?;
    Ok(serde_json::from_slice(&json.into_inner())?)
}

pub fn company_declaration(id: u32, name: &str, amount: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id, "category": {"id": 3, "name": "Donations"},
        "registrant": {"type": "Member", "memberDetail": {"id": 1}},
        "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"},
            "fields": [
                {"name": "DonorName", "value": name},
                {"name": "DonorStatus", "value": "Company"},
                {"name": "DonorCompanyIdentifier", "value": "00001234"},
                {"name": "Value", "value": amount}
            ]}]
    })
}

#[derive(Default)]
pub struct DeclarationSource {
    pub records: std::collections::BTreeMap<u32, serde_json::Value>,
    pub requests: Vec<String>,
}

pub async fn start_declaration_source(
    source: std::sync::Arc<std::sync::Mutex<DeclarationSource>>,
) -> anyhow::Result<(super::Guard, String)> {
    use axum::{
        Json, Router,
        extract::{OriginalUri, State},
    };
    type Source = std::sync::Arc<std::sync::Mutex<DeclarationSource>>;
    async fn interests(
        State(source): State<Source>,
        OriginalUri(uri): OriginalUri,
    ) -> Json<serde_json::Value> {
        let mut source = source.lock().unwrap();
        source.requests.push(uri.to_string());
        let url = reqwest::Url::parse(&format!("http://localhost{uri}")).unwrap();
        let query = url
            .query_pairs()
            .into_owned()
            .collect::<std::collections::BTreeMap<_, _>>();
        let discovery = query.contains_key("PublishedFrom") || query.contains_key("UpdatedFrom");
        let items = source
            .records
            .iter()
            .filter(|(id, _)| {
                if let Some(id_requested) = query.get("InterestIds") {
                    id.to_string() == *id_requested
                } else if let Some(member) = query.get("MemberId") {
                    member == "1"
                } else if query.contains_key("PublishedFrom") {
                    **id == 3
                } else {
                    **id == 1
                }
            })
            .map(|(_, record)| {
                let mut record = record.clone();
                if discovery {
                    record["versions"] = serde_json::json!([record["versions"]
                        .as_array()
                        .unwrap()
                        .first()
                        .unwrap()]);
                }
                record
            })
            .collect::<Vec<_>>();
        let total = items.len();
        let skip = query["Skip"].parse::<usize>().unwrap();
        let take = query["Take"].parse::<usize>().unwrap();
        Json(
            serde_json::json!({"totalResults": total, "items": items.into_iter().skip(skip).take(take).collect::<Vec<_>>()}),
        )
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let app = Router::new()
        .route("/Interests", axum::routing::get(interests))
        .with_state(source);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Ok((
        super::Guard { port, server },
        format!("http://127.0.0.1:{port}/Interests"),
    ))
}
