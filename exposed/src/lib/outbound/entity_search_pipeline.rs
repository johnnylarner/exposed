use std::sync::Arc;

use arrow::array::{RecordBatch, StringArray, UInt32Array};
use parquet::arrow::async_writer::AsyncArrowWriter;

use crate::{
    domain::{
        models::parliament_member::ParliamentMember,
        repositories::entity_ingestion_pipline::{
            EntityIngestionStorage, EntitySearchPipelineError,
        },
    },
    outbound::file_system::{ExposedDataPipeline, members_schema},
};

impl EntityIngestionStorage for ExposedDataPipeline {
    async fn read_raw_data(&self) -> Result<(), EntitySearchPipelineError> {
        let _ = tokio::spawn(async {}).await;
        Ok(())
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
        let write_to = tokio::fs::File::create_new(self.raw_path().join("members.parquet"))
            .await
            .map_err(|e| {
                EntitySearchPipelineError::WriteError(format!("cannot create raw members file:{e}"))
            })?;

        let schema = Arc::new(members_schema());

        let (mut pmi, mut n, mut pn, mut c) = (vec![], vec![], vec![], vec![]);
        for m in members {
            pmi.push(m.member_id());
            n.push(m.name());
            pn.push(m.party_name());
            c.push(m.constituency());
        }

        let (pmi, n, pn, c) = (
            UInt32Array::from(pmi),
            StringArray::from(n),
            StringArray::from(pn),
            StringArray::from(c),
        );

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(pmi), Arc::new(n), Arc::new(pn), Arc::new(c)],
        )
        .map_err(|e| {
            EntitySearchPipelineError::WriteError(format!("cannot create write batch: {e}"))
        })?;

        let mut writer = AsyncArrowWriter::try_new(write_to, schema, None).map_err(|e| {
            EntitySearchPipelineError::WriteError(format!("cannot create writer:{e}"))
        })?;

        writer.write(&batch).await.map_err(|e| {
            EntitySearchPipelineError::WriteError(format!("cannot write to buffer:{e}"))
        })?;

        writer.close().await.map_err(|e| {
            EntitySearchPipelineError::WriteError(format!("cannot flush writer:{e}"))
        })?;

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
