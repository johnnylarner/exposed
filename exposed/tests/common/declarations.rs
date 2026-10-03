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
    pub member_id: String,
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
            let columns = [
                "declaration_id",
                "member_id",
                "donor_name",
                "amount",
                "currency",
                "payment_type",
                "funder_kind",
                "company_number",
            ]
            .into_iter()
            .map(|name| reader.schema().index_of(name))
            .collect::<Result<Vec<_>, _>>()?;
            for batch in reader.build()? {
                json.write(&batch?.project(&columns)?)?;
            }
        }
    }
    json.finish()?;
    Ok(serde_json::from_slice(&json.into_inner())?)
}
