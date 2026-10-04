use std::sync::Arc;

use arrow::json::{ArrayWriter, ReaderBuilder};
use parquet::arrow::{ParquetRecordBatchStreamBuilder, async_writer::AsyncArrowWriter};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            declaration_ingestion::{CapturedDeclaration, MemberAsId},
            entity_ingestion::IngestionKey,
            parliament_member::ParliamentMember,
        },
        repositories::entity_ingestion::{EntityIngestionStorage, EntitySearchPipelineError},
    },
    outbound::file_system::{ExposedDataPipeline, members_schema},
};
use futures::TryStreamExt;

#[cfg(test)]
mod tests;

impl EntityIngestionStorage for ExposedDataPipeline {
    async fn write_raw_declarations(
        &self,
        member: MemberAsId,
        declarations: &[CapturedDeclaration],
    ) -> Result<(), EntitySearchPipelineError> {
        self.write_declaration_partition(member, declarations).await
    }

    fn ingestion_key(&self) -> IngestionKey {
        self.key().into()
    }
    async fn read_raw_members(&self) -> Result<Vec<ParliamentMember>, EntitySearchPipelineError> {
        let file = tokio::fs::File::open(self.raw_path().join("members.parquet"))
            .await
            .map_err(read_error)?;
        let mut stream = ParquetRecordBatchStreamBuilder::new(file)
            .await
            .map_err(read_error)?
            .build()
            .map_err(read_error)?;
        let mut json = ArrayWriter::new(Vec::new());
        while let Some(batch) = stream.try_next().await.map_err(read_error)? {
            json.write(&batch).map_err(read_error)?;
        }
        json.finish().map_err(read_error)?;
        let records: Vec<MemberRecord> =
            serde_json::from_slice(&json.into_inner()).map_err(read_error)?;
        Ok(records.into_iter().map(ParliamentMember::from).collect())
    }
    async fn read_cleaned_data(&self) -> Result<(), EntitySearchPipelineError> {
        let _ = tokio::spawn(async {}).await;
        Ok(())
    }
    async fn read_resolved_data(&self) -> Result<(), EntitySearchPipelineError> {
        let _ = tokio::spawn(async {}).await;
        Ok(())
    }
    async fn write_raw_members(
        &self,
        members: &[ParliamentMember],
    ) -> Result<(), EntitySearchPipelineError> {
        let file = tokio::fs::File::create_new(self.raw_path().join("members.parquet"))
            .await
            .map_err(write_error)?;
        let schema = Arc::new(members_schema());
        let mut writer =
            AsyncArrowWriter::try_new(file, schema.clone(), None).map_err(write_error)?;
        let records = members.iter().map(MemberRecord::from).collect::<Vec<_>>();
        let mut decoder = ReaderBuilder::new(schema)
            .build_decoder()
            .map_err(write_error)?;
        decoder.serialize(&records).map_err(write_error)?;
        if let Some(batch) = decoder.flush().map_err(write_error)? {
            writer.write(&batch).await.map_err(write_error)?;
        }
        writer.close().await.map_err(write_error)?;
        Ok(())
    }
    async fn write_cleaned_data(&self) -> Result<(), EntitySearchPipelineError> {
        let _ = tokio::spawn(async {}).await;
        Ok(())
    }
    async fn write_resolved_data(&self) -> Result<(), EntitySearchPipelineError> {
        let _ = tokio::spawn(async {}).await;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
struct MemberRecord {
    parliament_member_id: u32,
    name: String,
    party_id: u32,
    party_name: String,
    constituency: String,
}

impl From<&ParliamentMember> for MemberRecord {
    fn from(member: &ParliamentMember) -> Self {
        Self {
            parliament_member_id: member.member_id(),
            name: member.name().to_string(),
            party_id: member.party_id(),
            party_name: member.party_name().to_string(),
            constituency: member.constituency().to_string(),
        }
    }
}

impl From<MemberRecord> for ParliamentMember {
    fn from(record: MemberRecord) -> Self {
        Self::new(
            record.name,
            record.parliament_member_id,
            record.party_name,
            record.party_id,
            record.constituency,
        )
    }
}

fn read_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::ReadError(error.to_string())
}

fn write_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::WriteError(error.to_string())
}
