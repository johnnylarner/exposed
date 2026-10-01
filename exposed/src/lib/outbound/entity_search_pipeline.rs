use crate::{
    domain::repositories::entity_ingestion_pipline::{
        EntityIngestionStorage, EntitySearchPipelineError,
    },
    outbound::file_system::ExposedDataPipeline,
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
    async fn write_raw_data(&self) -> Result<(), EntitySearchPipelineError> {
        let _ = tokio::spawn(async {}).await;
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
