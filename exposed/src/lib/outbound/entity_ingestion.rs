use std::sync::Arc;

use arrow::array::{RecordBatch, StringArray, UInt32Array};
use itertools::izip;
use parquet::arrow::{ParquetRecordBatchStreamBuilder, async_writer::AsyncArrowWriter};

use crate::{
    domain::{
        models::{entity_ingestion::IngestionKey, parliament_member::ParliamentMember},
        repositories::entity_ingestion::{EntityIngestionStorage, EntitySearchPipelineError},
    },
    outbound::file_system::{ExposedDataPipeline, members_schema},
};
use futures::TryStreamExt;

impl EntityIngestionStorage for ExposedDataPipeline {
    fn ingestion_key(&self) -> IngestionKey {
        self.key().into()
    }
    async fn read_raw_members(&self) -> Result<Vec<ParliamentMember>, EntitySearchPipelineError> {
        let read_from = tokio::fs::File::open(self.raw_path().join("members.parquet"))
            .await
            .map_err(|e| {
                EntitySearchPipelineError::ReadError(format!("cannot read raw members file:{e}"))
            })?;

        let stream = ParquetRecordBatchStreamBuilder::new(read_from)
            .await
            .map_err(|e| {
                EntitySearchPipelineError::ReadError(format!("cannot read raw members file:{e}"))
            })?
            .build()
            .map_err(|e| {
                EntitySearchPipelineError::ReadError(format!("cannot read raw members file:{e}"))
            })?;

        let results = stream.try_collect::<Vec<_>>().await.unwrap();
        let first_batch = results.first().ok_or_else(|| {
            EntitySearchPipelineError::ReadError("must be a read result for members".to_string())
        })?;
        let ids: Vec<_> = first_batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .ok_or_else(|| EntitySearchPipelineError::ReadError("cannot cast ids".to_string()))?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        let names: Vec<_> = first_batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| EntitySearchPipelineError::ReadError("cannot cast names".to_string()))?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        let party_ids: Vec<_> = first_batch
            .column(2)
            .as_any()
            .downcast_ref::<UInt32Array>()
            .ok_or_else(|| {
                EntitySearchPipelineError::ReadError("cannot cast party ids".to_string())
            })?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        let parties: Vec<_> = first_batch
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| EntitySearchPipelineError::ReadError("cannot cast parties".to_string()))?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        let constituencies: Vec<_> = first_batch
            .column(4)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| {
                EntitySearchPipelineError::ReadError("cannot cast constituencies".to_string())
            })?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();

        let mut members = Vec::with_capacity(names.len());
        for (pmi, n, pi, pn, c) in izip!(ids, names, party_ids, parties, constituencies) {
            members.push(ParliamentMember::new(
                n.to_string(),
                pmi,
                pn.to_string(),
                pi,
                c.to_string(),
            ));
        }

        Ok(members)
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

        let (mut pmi, mut n, mut pi, mut pn, mut c) = (vec![], vec![], vec![], vec![], vec![]);
        for m in members {
            pmi.push(m.member_id());
            n.push(m.name());
            pi.push(m.party_id());
            pn.push(m.party_name());
            c.push(m.constituency());
        }

        let (pmi, n, pi, pn, c) = (
            UInt32Array::from(pmi),
            StringArray::from(n),
            UInt32Array::from(pi),
            StringArray::from(pn),
            StringArray::from(c),
        );

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(pmi),
                Arc::new(n),
                Arc::new(pi),
                Arc::new(pn),
                Arc::new(c),
            ],
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
