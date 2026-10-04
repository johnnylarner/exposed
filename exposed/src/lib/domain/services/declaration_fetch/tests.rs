#![allow(
    clippy::unused_async_trait_impl,
    reason = "Test adapters keep async port methods uniform"
)]

use std::{sync::Arc, time::Duration};

use tokio::sync::{Semaphore, mpsc};
use uuid::Uuid;

use super::*;
use crate::domain::{
    models::{
        declaration_ingestion::{CapturedDeclaration, MemberAsId},
        entity_search::EntitySearchRequest,
        parliament_member::ParliamentMember,
        search_similarity::SearchSimilarity,
    },
    repositories::{
        entity_ingestion::EntitySearchPipelineError, parliament_api::ParliamentApiError,
        parliament_member_repository::ParliamentMemberRepoError,
    },
};

#[derive(Clone)]
struct Fake {
    members: Vec<MemberAsId>,
    key: IngestionKey,
    started: mpsc::UnboundedSender<u32>,
    writes: mpsc::UnboundedSender<u32>,
    permits: Arc<Semaphore>,
    fail: bool,
}

impl ParliamentMemberRepo for Fake {
    async fn get_stored_member_ids(&self) -> Result<Vec<MemberAsId>, ParliamentMemberRepoError> {
        Ok(self.members.clone())
    }
    async fn get_members_by_text_search_score(
        &self,
        _: &EntitySearchRequest,
    ) -> Result<Vec<(ParliamentMember, SearchSimilarity)>, ParliamentMemberRepoError> {
        unreachable!()
    }
    async fn upsert_members(
        &self,
        _: &[ParliamentMember],
    ) -> Result<(), ParliamentMemberRepoError> {
        unreachable!()
    }
}

impl ParliamentApi for Fake {
    async fn get_sitting_members(&self) -> Result<Vec<ParliamentMember>, ParliamentApiError> {
        unreachable!()
    }
    async fn get_declarations(
        &self,
        member: MemberAsId,
    ) -> Result<Vec<CapturedDeclaration>, ParliamentApiError> {
        self.started.send(member.parliament_member_id()).unwrap();
        Ok(Vec::new())
    }
}

impl EntityIngestionStorage for Fake {
    async fn write_raw_declarations(
        &self,
        member: MemberAsId,
        _: &[CapturedDeclaration],
    ) -> Result<(), EntitySearchPipelineError> {
        self.permits.acquire().await.unwrap().forget();
        if self.fail {
            return Err(EntitySearchPipelineError::WriteError("failed".into()));
        }
        self.writes.send(member.parliament_member_id()).unwrap();
        Ok(())
    }
    fn ingestion_key(&self) -> IngestionKey {
        self.key.clone()
    }
    async fn read_raw_members(&self) -> Result<Vec<ParliamentMember>, EntitySearchPipelineError> {
        unreachable!()
    }
    async fn read_cleaned_data(&self) -> Result<(), EntitySearchPipelineError> {
        unreachable!()
    }
    async fn read_resolved_data(&self) -> Result<(), EntitySearchPipelineError> {
        unreachable!()
    }
    async fn write_raw_members(
        &self,
        _: &[ParliamentMember],
    ) -> Result<(), EntitySearchPipelineError> {
        unreachable!()
    }
    async fn write_cleaned_data(&self) -> Result<(), EntitySearchPipelineError> {
        unreachable!()
    }
    async fn write_resolved_data(&self) -> Result<(), EntitySearchPipelineError> {
        unreachable!()
    }
}

fn fixture(
    count: u32,
) -> (
    Fake,
    mpsc::UnboundedReceiver<u32>,
    mpsc::UnboundedReceiver<u32>,
) {
    let (started, starts) = mpsc::unbounded_channel();
    let (writes, saved) = mpsc::unbounded_channel();
    (
        Fake {
            members: (1..=count)
                .map(|id| MemberAsId::new(Uuid::now_v7(), id).unwrap())
                .collect(),
            key: IngestionKey::default(),
            started,
            writes,
            permits: Arc::new(Semaphore::new(0)),
            fail: false,
        },
        starts,
        saved,
    )
}

async fn receive(receiver: &mut mpsc::UnboundedReceiver<u32>) -> u32 {
    tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn keeps_ten_members_active_through_storage_and_refills_each_slot() {
    let (fake, mut starts, mut saved) = fixture(23);
    let service = DeclarationFetcherService::new(fake.clone(), fake.clone(), fake.clone());
    let run = tokio::spawn(async move { service.fetch_declarations().await });
    for _ in 0..10 {
        receive(&mut starts).await;
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(50), starts.recv())
            .await
            .is_err()
    );
    fake.permits.add_permits(1);
    let first_saved = receive(&mut saved).await;
    assert_eq!(receive(&mut starts).await, 11);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), starts.recv())
            .await
            .is_err()
    );
    fake.permits.add_permits(22);
    assert_eq!(run.await.unwrap().unwrap(), fake.key);
    let mut written = vec![first_saved];
    while let Ok(id) = saved.try_recv() {
        written.push(id);
    }
    assert_eq!(written.len(), 23);
    written.sort_unstable();
    written.dedup();
    assert_eq!(written.len(), 23);
}

#[tokio::test]
async fn storage_failure_cancels_pending_members_without_scheduling_more() {
    let (mut fake, mut starts, mut saved) = fixture(23);
    fake.fail = true;
    let service = DeclarationFetcherService::new(fake.clone(), fake.clone(), fake.clone());
    let run = tokio::spawn(async move { service.fetch_declarations().await });
    for _ in 0..10 {
        receive(&mut starts).await;
    }
    fake.permits.add_permits(1);
    let result = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err(EntityIngestionError::IoError(_))));
    assert!(starts.try_recv().is_err());
    assert!(saved.try_recv().is_err());
    assert_eq!(Arc::strong_count(&fake.permits), 1);
}

#[tokio::test]
async fn rejects_empty_cohort_without_starting_tasks() {
    let (fake, mut starts, _) = fixture(0);
    let service = DeclarationFetcherService::new(fake.clone(), fake.clone(), fake);
    assert!(matches!(
        service.fetch_declarations().await,
        Err(EntityIngestionError::DataError(_))
    ));
    assert!(starts.try_recv().is_err());
}
