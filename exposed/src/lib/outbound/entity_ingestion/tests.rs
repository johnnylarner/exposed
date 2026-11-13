use tempfile::TempDir;

use super::ExposedDataPipeline;
use crate::domain::{
    models::{
        entity_ingestion::IngestionKey,
        parliament_member::{MemberId, ParliamentMember},
    },
    repositories::entity_ingestion::EntityIngestionStorage,
};

#[tokio::test]
async fn preserves_members_across_multiple_parquet_batches() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    // The Parquet reader returns at most 1,024 rows in each default batch.
    let members = (1..=2_050)
        .map(|id| {
            ParliamentMember::new(
                format!("Member {id}"),
                MemberId::new(id).unwrap(),
                "Example party".into(),
                7,
                "Example constituency".into(),
            )
        })
        .collect::<Vec<_>>();

    storage.write_raw_members(&members).await?;
    let restored = storage.read_raw_members().await?;

    assert_eq!(restored, members);
    Ok(())
}

#[tokio::test]
async fn reads_an_empty_members_file() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;

    storage.write_raw_members(&[]).await?;

    assert!(storage.read_raw_members().await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn parquet_retains_shared_source_membership_and_capture_provenance() -> anyhow::Result<()> {
    use crate::domain::models::membership::SourceMembership;
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let mut member = ParliamentMember::new(
        "Shared member".into(),
        MemberId::new(1)?,
        "Party".into(),
        7,
        "Former constituency".into(),
    );
    member.profile.membership=serde_json::from_value::<SourceMembership>(serde_json::json!({"house":2,"membershipFromId":4,"membershipFrom":"Life peer","membershipStartDate":"2020-01-02T00:00:00","membershipEndDate":null,"membershipStatus":{"statusDescription":"Leave of Absence","statusIsActive":false}}))?.into_membership().map_err(anyhow::Error::msg)?;
    member.profile.party_id = None;
    member.profile.party_name = None;
    member.captured_at = Some("2026-03-01T12:34:56.123456Z".parse()?);
    member.capture_key = Some(uuid::Uuid::now_v7());
    storage.write_raw_members(&[member.clone()]).await?;
    assert_eq!(storage.read_raw_members().await?, [member]);
    Ok(())
}

#[tokio::test]
async fn old_member_artifacts_keep_unknown_clocks_and_explicit_legacy_classification()
-> anyhow::Result<()> {
    use crate::domain::models::membership::{MembershipKind, MembershipStatusProvenance};
    use arrow::{
        datatypes::{DataType, Field, Schema},
        json::ReaderBuilder,
    };
    use parquet::arrow::async_writer::AsyncArrowWriter;
    #[derive(serde::Serialize)]
    struct LegacyMember {
        parliament_member_id: u32,
        name: &'static str,
        party_id: u32,
        party_name: &'static str,
        constituency: &'static str,
    }
    let tmp = TempDir::new()?;
    let storage = ExposedDataPipeline::new_with_ingestion_key(
        &tmp.path().to_path_buf(),
        IngestionKey::default(),
    )?;
    let schema = std::sync::Arc::new(Schema::new(vec![
        Field::new("parliament_member_id", DataType::UInt32, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("party_id", DataType::UInt32, false),
        Field::new("party_name", DataType::Utf8, false),
        Field::new("constituency", DataType::Utf8, false),
    ]));
    let file = tokio::fs::File::create_new(storage.raw_path().join("members.parquet")).await?;
    let mut writer = AsyncArrowWriter::try_new(file, schema.clone(), None)?;
    let mut decoder = ReaderBuilder::new(schema).build_decoder()?;
    decoder.serialize(&[LegacyMember {
        parliament_member_id: 1,
        name: "Legacy MP",
        party_id: 7,
        party_name: "Party",
        constituency: "Original constituency",
    }])?;
    writer.write(&decoder.flush()?.unwrap()).await?;
    writer.close().await?;
    let restored = storage.read_raw_members().await?;
    let member = &restored[0];
    assert_eq!(member.profile.membership.kind, MembershipKind::ElectedMp);
    assert_eq!(
        member.profile.membership.status_provenance,
        MembershipStatusProvenance::LegacyCurrent
    );
    assert_eq!(member.profile.membership.is_current_commons(), Some(true));
    assert!(member.profile.membership.membership_status_active.is_none());
    assert!(member.profile.membership.membership_start_date.is_none());
    assert!(member.profile.membership.membership_end_date.is_none());
    assert!(member.captured_at.is_none());
    assert!(member.capture_key.is_none());
    Ok(())
}
