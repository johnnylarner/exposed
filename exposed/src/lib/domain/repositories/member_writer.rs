//! Atomic publication of an already validated member refresh.
use crate::domain::models::member_ingestion::{MemberImportSummary, MemberRefresh};
use thiserror::Error;

/// Database capability used only by Load.
pub trait MemberWriter: Clone + Send + Sync + 'static {
    /// Check the stored Parliament and reconcile all members in one transaction.
    fn refresh_members(
        &self,
        refresh: &MemberRefresh,
    ) -> impl Future<Output = Result<MemberImportSummary, MemberWriteError>> + Send;
}

/// Database failure; no successful summary is returned before commit.
#[derive(Error, Debug)]
#[error("{0}")]
pub struct MemberWriteError(pub String);
