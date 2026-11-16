use crate::domain::models::{membership::MemberProfile, source_timestamp::SourceTimestamp};

pub(super) async fn upsert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    profile: &MemberProfile,
) -> Result<uuid::Uuid, sqlx::Error> {
    let kind = profile.membership.kind.name();
    let start = profile
        .membership
        .membership_start_date
        .as_ref()
        .map(SourceTimestamp::as_str);
    let end = profile
        .membership
        .membership_end_date
        .as_ref()
        .map(SourceTimestamp::as_str);
    let party_id =
        i32::try_from(profile.party_id).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    sqlx::query_scalar!(
        "INSERT INTO exposed.members
         (parliament_member_id, name, parliamentary_name, party_id, party_name, membership_kind, membership_from,
          membership_start_date, membership_end_date, membership_status)
         VALUES ($1,$2,$2,$3,$4,(SELECT id FROM exposed.membership_kinds WHERE name = $5),$6,$7,$8,$9)
         ON CONFLICT (parliament_member_id) DO UPDATE SET
             name=CASE WHEN members.personal_first_name IS NULL THEN EXCLUDED.name ELSE members.name END,
             parliamentary_name=EXCLUDED.parliamentary_name, party_id=EXCLUDED.party_id, party_name=EXCLUDED.party_name,
             membership_kind=EXCLUDED.membership_kind, membership_from=EXCLUDED.membership_from,
             membership_start_date=EXCLUDED.membership_start_date,
             membership_end_date=EXCLUDED.membership_end_date, membership_status=EXCLUDED.membership_status
         RETURNING id",
        profile.id.value().cast_signed(), profile.name, party_id, profile.party_name, kind,
        profile.membership.membership_from, start, end, profile.membership.membership_status
    ).fetch_one(&mut **tx).await
}
