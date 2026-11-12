use crate::domain::models::parliament_member::MemberId;

pub(super) async fn acquire(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    member_id: MemberId,
) -> Result<uuid::Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO exposed.members (parliament_member_id) VALUES ($1)
         ON CONFLICT (parliament_member_id) DO UPDATE
         SET parliament_member_id = EXCLUDED.parliament_member_id RETURNING id",
        member_id.value().cast_signed()
    )
    .fetch_one(&mut **tx)
    .await
}
