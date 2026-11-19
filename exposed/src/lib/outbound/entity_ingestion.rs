use std::sync::Arc;

use arrow::json::{ArrayWriter, ReaderBuilder};
use parquet::arrow::{ParquetRecordBatchStreamBuilder, async_writer::AsyncArrowWriter};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        models::{
            declaration_ingestion::CapturedDeclaration,
            entity_ingestion::IngestionKey,
            membership::{MemberProfile, Membership, MembershipKind},
            parliament_member::{MemberId, ParliamentMember},
            source_timestamp::SourceTimestamp,
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
        member: MemberId,
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
    parliament_member_id: MemberId,
    name: String,
    party_id: u32,
    party_name: String,
    constituency: String,
    #[serde(default)]
    membership_kind: Option<MembershipKind>,
    #[serde(default)]
    membership_start_date: Option<SourceTimestamp>,
    #[serde(default)]
    membership_end_date: Option<SourceTimestamp>,
    #[serde(default)]
    membership_status: Option<String>,
}

impl From<&ParliamentMember> for MemberRecord {
    fn from(member: &ParliamentMember) -> Self {
        Self {
            parliament_member_id: member.member_id(),
            name: member.name().to_string(),
            party_id: member.profile.party_id,
            party_name: member.profile.party_name.clone(),
            constituency: member.profile.membership.membership_from.clone(),
            membership_kind: Some(member.profile.membership.kind),
            membership_start_date: member.profile.membership.membership_start_date.clone(),
            membership_end_date: member.profile.membership.membership_end_date.clone(),
            membership_status: member.profile.membership.membership_status.clone(),
        }
    }
}

impl From<MemberRecord> for ParliamentMember {
    fn from(record: MemberRecord) -> Self {
        Self {
            profile: MemberProfile {
                id: record.parliament_member_id,
                name: record.name,
                parliamentary_name: None,
                party_id: record.party_id,
                party_name: record.party_name,
                membership: Membership {
                    kind: record.membership_kind.unwrap_or(MembershipKind::ElectedMp),
                    membership_from: record.constituency,
                    membership_start_date: record.membership_start_date,
                    membership_end_date: record.membership_end_date,
                    membership_status: record.membership_status,
                },
            },
        }
    }
}

fn read_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::ReadError(error.to_string())
}

fn write_error(error: impl std::fmt::Display) -> EntitySearchPipelineError {
    EntitySearchPipelineError::WriteError(error.to_string())
}
