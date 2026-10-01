//! Acquisition and offline loading, with separate infrastructure capabilities.
use crate::domain::{
    models::{
        entity_ingestion::{
            EntityIngestionError, EntityIngestionOutcome, EntityIngestionRequest,
            EntityIngestionTarget, MemberIngestionStage,
        },
        member_ingestion::{
            CaptureContext, CaptureId, MemberCapture, MemberObservations, MemberProfile, invalid,
        },
    },
    repositories::{
        entity_ingestion_pipline::EntityIngestionStorage, member_writer::MemberWriter,
        parliament_api::ParliamentApi,
    },
};
use chrono::{DateTime, NaiveDate, Utc};
use interface::EntitySearchIngestionService;
use std::collections::{BTreeMap, BTreeSet};

/// Public service boundary used by inbound adapters.
pub mod interface;

/// Capture orchestration. It has no database capability.
#[derive(Clone)]
pub struct FetchService<S, P> {
    storage: S,
    source: P,
    started_at: DateTime<Utc>,
    observation_date: NaiveDate,
}
impl<S, P> FetchService<S, P> {
    /// Supply source/storage adapters and one fixed observation context.
    #[must_use]
    pub const fn new(
        storage: S,
        source: P,
        started_at: DateTime<Utc>,
        observation_date: NaiveDate,
    ) -> Self {
        Self {
            storage,
            source,
            started_at,
            observation_date,
        }
    }
}
impl<S: EntityIngestionStorage, P: ParliamentApi> EntitySearchIngestionService
    for FetchService<S, P>
{
    async fn run_ingestion(
        &self,
        req: &EntityIngestionRequest,
    ) -> Result<EntityIngestionOutcome, EntityIngestionError> {
        let capture_id = CaptureId::new();
        let EntityIngestionTarget::Members(MemberIngestionStage::Fetch { term_start }) =
            req.target()
        else {
            return Err(EntityIngestionError::InvalidStage(
                "this service supports member Fetch".into(),
            ));
        };
        if *term_start > self.observation_date {
            return Err(invalid("term start is after observation date"));
        }
        eprintln!("Capture {capture_id}: fetching current Commons observations");
        let current_commons = unique_profiles(self.source.current_commons().await?)?
            .into_keys()
            .collect();
        eprintln!(
            "Capture {capture_id}: fetching historical Commons candidates since {term_start}"
        );
        let profiles = unique_profiles(
            self.source
                .commons_candidates(*term_start, self.observation_date)
                .await?,
        )?;
        let ids: Vec<_> = profiles.keys().copied().collect();
        let mut histories = Vec::new();
        for batch in ids.chunks(100) {
            let returned = self.source.member_histories(batch).await?;
            let returned_ids: BTreeSet<_> =
                returned.iter().map(|h| h.parliament_member_id).collect();
            if returned_ids.len() != returned.len()
                || returned_ids != batch.iter().copied().collect()
            {
                return Err(invalid(format!(
                    "history response must match requested member IDs exactly: {batch:?}; returned {returned_ids:?}"
                )));
            }
            histories.extend(returned);
        }
        histories.sort_by_key(|history| history.parliament_member_id);
        let observations = MemberObservations {
            profiles: profiles.into_values().collect(),
            current_commons,
            histories,
        };
        observations.validate()?;
        let counts = observations.counts();
        let capture = MemberCapture {
            context: CaptureContext {
                capture_id,
                term_start: *term_start,
                observation_date: self.observation_date,
                started_at: self.started_at,
            },
            observations,
        };
        let capture_id = self.storage.write_member_capture(capture).await?;
        Ok(EntityIngestionOutcome::Fetch { capture_id, counts })
    }
}

/// Offline loading. Source access is absent from its contract.
#[derive(Clone)]
pub struct LoadService<S, W> {
    storage: S,
    writer: W,
}
impl<S, W> LoadService<S, W> {
    /// Supply completed-capture storage and an atomic member writer.
    #[must_use]
    pub const fn new(storage: S, writer: W) -> Self {
        Self { storage, writer }
    }
}
impl<S: EntityIngestionStorage, W: MemberWriter> EntitySearchIngestionService
    for LoadService<S, W>
{
    async fn run_ingestion(
        &self,
        req: &EntityIngestionRequest,
    ) -> Result<EntityIngestionOutcome, EntityIngestionError> {
        let EntityIngestionTarget::Members(MemberIngestionStage::Load { capture_id }) =
            req.target()
        else {
            return Err(EntityIngestionError::InvalidStage(
                "this service supports member Load".into(),
            ));
        };
        eprintln!("Capture {capture_id}: validating offline member data");
        let capture = self.storage.read_member_capture(*capture_id).await?;
        let refresh = capture.prepare_refresh()?;
        eprintln!(
            "Capture {capture_id}: loading {} eligible members",
            refresh.members.len()
        );
        let summary = self.writer.refresh_members(&refresh).await?;
        Ok(EntityIngestionOutcome::Load {
            capture_id: *capture_id,
            term_start: capture.context.term_start,
            observation_date: capture.context.observation_date,
            summary,
        })
    }
}

fn unique_profiles(
    profiles: Vec<MemberProfile>,
) -> Result<BTreeMap<i32, MemberProfile>, EntityIngestionError> {
    let mut unique = BTreeMap::new();
    for profile in profiles {
        let id = profile.parliament_member_id;
        if id <= 0 {
            return Err(invalid(format!(
                "member {id}: source identity must be positive"
            )));
        }
        if let Some(previous) = unique.get(&id) {
            if previous != &profile {
                return Err(invalid(format!(
                    "member {id}: conflicting duplicate profile"
                )));
            }
            eprintln!("Member {id}: identical repeated profile collapsed");
        } else {
            unique.insert(id, profile);
        }
    }
    Ok(unique)
}
