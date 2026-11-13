use crate::domain::models::membership::MemberProfile;

/// Both imports serialize writes on the canonical member row and compare the same source clock.
pub(super) async fn upsert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    profile: &MemberProfile,
    captured_at: Option<chrono::DateTime<chrono::Utc>>,
    capture_key: Option<uuid::Uuid>,
    commons: bool,
) -> Result<uuid::Uuid, sqlx::Error> {
    let captured_at = captured_at
        .map(|value| {
            time::OffsetDateTime::from_unix_timestamp_nanos(
                i128::from(value.timestamp()) * 1_000_000_000
                    + i128::from(value.timestamp_subsec_nanos()),
            )
        })
        .transpose()
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    let kind = profile.membership.kind.to_string();
    let status_provenance = profile.membership.status_provenance.to_string();
    let start = profile
        .membership
        .membership_start_date
        .as_ref()
        .map(crate::domain::models::source_timestamp::SourceTimestamp::as_str);
    let end = profile
        .membership
        .membership_end_date
        .as_ref()
        .map(crate::domain::models::source_timestamp::SourceTimestamp::as_str);
    let party_id = profile
        .party_id
        .map(i32::try_from)
        .transpose()
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    let id = sqlx::query_scalar!(
        "INSERT INTO exposed.members
         (parliament_member_id, name, party_id, party_name, membership_kind, membership_from,
          membership_start_date, membership_end_date, membership_status, membership_status_active,
          membership_status_provenance, source_captured_at, source_capture_key, commons_imported_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,CASE WHEN $14 THEN now() END)
         ON CONFLICT (parliament_member_id) DO UPDATE SET
             commons_imported_at = COALESCE(exposed.members.commons_imported_at, EXCLUDED.commons_imported_at)
         RETURNING id",
        profile.id.value().cast_signed(), profile.name, party_id, profile.party_name, kind,
        profile.membership.membership_from, start, end, profile.membership.membership_status,
        profile.membership.membership_status_active, status_provenance, captured_at, capture_key, commons
    ).fetch_one(&mut **tx).await?;
    sqlx::query!(
        "UPDATE exposed.members SET name=$2, party_id=$3, party_name=$4, membership_kind=$5,
             membership_from=$6, membership_start_date=$7, membership_end_date=$8,
             membership_status=$9, membership_status_active=$10, membership_status_provenance=$11,
             source_captured_at=$12, source_capture_key=$13
         WHERE id=$1 AND (source_captured_at IS NULL OR (source_captured_at,source_capture_key) < ($12,$13))",
        id, profile.name, party_id, profile.party_name, kind, profile.membership.membership_from,
        start,end,profile.membership.membership_status,profile.membership.membership_status_active,
        status_provenance,captured_at,capture_key
    ).execute(&mut **tx).await?;
    Ok(id)
}
