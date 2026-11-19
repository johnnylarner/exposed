//! Official source, raw evidence files, and the existing-member refresh transaction.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use super::ExposedDatabase;
use crate::domain::{
    models::{
        entity_ingestion::IngestionKey,
        member_names::{MemberNames, MemberNamesCapture, MemberNamesError, check_coverage},
        membership::{Chamber, MembershipKind},
        parliament_member::MemberId,
    },
    repositories::member_names::{MemberNamesRepository, MemberNamesSource, MemberNamesStorage},
};

impl From<sqlx::Error> for MemberNamesError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error.to_string())
    }
}

/// MNIS official preferred names. The source documents are retained byte for byte as UTF-8.
pub struct MemberNamesApi {
    client: reqwest::Client,
}
impl MemberNamesApi {
    /// Builds the official source client with a bounded request timeout.
    ///
    /// # Errors
    /// Rejects client configuration failures.
    pub fn new() -> Result<Self, MemberNamesError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(source)?,
        })
    }
    async fn get(&self, query: &str) -> Result<String, MemberNamesError> {
        let bytes = self.client.get(format!("https://data.parliament.uk/membersdataplatform/services/mnis/members/query/{query}/PreferredNames/"))
            .header(reqwest::header::ACCEPT, "application/json")
            .send().await.map_err(source)?.error_for_status().map_err(source)?.bytes().await.map_err(source)?;
        String::from_utf8(bytes.to_vec()).map_err(source)
    }
}
fn source(error: impl std::fmt::Display) -> MemberNamesError {
    MemberNamesError::Source(error.to_string())
}
impl MemberNamesSource for MemberNamesApi {
    async fn chamber(&self, chamber: Chamber) -> Result<String, MemberNamesError> {
        self.get(&format!("House={}", chamber.name())).await
    }
    async fn member(&self, id: MemberId) -> Result<String, MemberNamesError> {
        self.get(&format!("id={id}")).await
    }
}

/// One immutable raw capture per ingestion run.
pub struct MemberNamesFiles {
    path: PathBuf,
}
impl MemberNamesFiles {
    /// Selects the dedicated raw member-name artifact in an ingestion run.
    #[must_use]
    pub fn new(data_dir: &Path, key: &IngestionKey) -> Self {
        Self {
            path: data_dir.join(key.to_string()).join("raw/member-names.json"),
        }
    }
    /// Artifact destination.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}
fn capture(error: impl std::fmt::Display) -> MemberNamesError {
    MemberNamesError::Capture(error.to_string())
}
impl MemberNamesStorage for MemberNamesFiles {
    fn publish(&self, evidence: &MemberNamesCapture) -> Result<(), MemberNamesError> {
        evidence.names()?;
        let parent = self.path.parent().expect("capture path has a parent");
        std::fs::create_dir_all(parent).map_err(capture)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(capture)?;
        serde_json::to_writer(temporary.as_file_mut(), evidence).map_err(capture)?;
        temporary.as_file().sync_all().map_err(capture)?;
        temporary.persist_noclobber(&self.path).map_err(capture)?;
        Ok(())
    }
    fn read(&self) -> Result<MemberNamesCapture, MemberNamesError> {
        serde_json::from_reader(std::fs::File::open(&self.path).map_err(capture)?).map_err(capture)
    }
}

fn targets(rows: Vec<(i32, String)>) -> Result<Vec<(MemberId, Chamber)>, MemberNamesError> {
    rows.into_iter()
        .map(|(id, kind)| {
            Ok((
                MemberId::new(u32::try_from(id).map_err(source)?).map_err(source)?,
                MembershipKind::try_from(kind.as_str())
                    .map_err(source)?
                    .chamber(),
            ))
        })
        .collect()
}
impl MemberNamesRepository for ExposedDatabase {
    async fn targets(&self) -> Result<Vec<(MemberId, Chamber)>, MemberNamesError> {
        let rows = sqlx::query!("SELECT m.parliament_member_id, k.name AS kind FROM exposed.members m JOIN exposed.membership_kinds k ON k.id = m.membership_kind ORDER BY m.parliament_member_id")
            .fetch_all(self.pool()).await?;
        targets(
            rows.into_iter()
                .map(|row| (row.parliament_member_id, row.kind))
                .collect(),
        )
    }
    async fn refresh(
        &self,
        names: &BTreeMap<MemberId, MemberNames>,
    ) -> Result<usize, MemberNamesError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query!("LOCK TABLE exposed.members IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        let rows = sqlx::query!("SELECT m.parliament_member_id, k.name AS kind FROM exposed.members m JOIN exposed.membership_kinds k ON k.id = m.membership_kind ORDER BY m.parliament_member_id")
            .fetch_all(&mut *tx).await?;
        let targets = targets(
            rows.into_iter()
                .map(|row| (row.parliament_member_id, row.kind))
                .collect(),
        )?;
        check_coverage(&targets, names)?;
        for (id, _) in &targets {
            let name = &names[id];
            let primary = name.personal.display();
            sqlx::query!("UPDATE exposed.members SET name=$2, personal_first_name=$3, personal_last_name=$4, parliamentary_name=$5, formal_name=$6 WHERE parliament_member_id=$1",
                id.value().cast_signed(), primary, name.personal.first(), name.personal.last(), name.parliamentary, name.formal).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(targets.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        models::{entity_search::EntitySearchRequest, parliament_member::ParliamentMember},
        repositories::{
            lords::LordsRepository, parliament_member_repository::ParliamentMemberRepo,
        },
        services::member_names,
    };
    use serde_json::json;
    use sqlx::PgPool;

    fn source(id: u32, house: Chamber, first: &str, last: &str, parliamentary: &str) -> String {
        json!({"Members":{"Member":{"@Member_Id":id.to_string(),"House":house.name(),"DisplayAs":parliamentary,"FullTitle":format!("The {parliamentary}"),"PreferredNames":{"PreferredName":{"Forename":first,"Surname":last,"StartDate":"2010-01-01T00:00:00","EndDate":{"@xsi:nil":"true"}}}}}}).to_string()
    }
    fn request(term: &str) -> EntitySearchRequest {
        EntitySearchRequest::new_with_strictness(term.into(), 10, 0.8).unwrap()
    }
    #[sqlx::test(migrations = "../db/migrations")]
    async fn offline_refresh_is_complete_preserves_legacy_replay_and_searches_all_names(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let db = ExposedDatabase::from(pool.clone());
        let mp = ParliamentMember::new(
            "Sir John Smith".into(),
            MemberId::new(1).unwrap(),
            "Party".into(),
            1,
            "Constituency".into(),
        );
        db.upsert_members(std::slice::from_ref(&mp)).await.unwrap();
        sqlx::query!("INSERT INTO exposed.members (parliament_member_id,name,party_id,party_name,membership_kind,membership_from) VALUES (2,'Lord Addington',1,'Party',(SELECT id FROM exposed.membership_kinds WHERE name='Life peer'),'Life peer')").execute(&pool).await?;
        let directory = tempfile::tempdir().unwrap();
        let key = IngestionKey::default();
        let storage = MemberNamesFiles::new(directory.path(), &key);
        let documents = vec![
            source(1, Chamber::Commons, "John", "Smith", "Sir John Smith"),
            source(2, Chamber::Lords, "Dominic", "Hubbard", "Lord Addington"),
        ];
        storage
            .publish(&MemberNamesCapture::new(vec![documents[0].clone()]).unwrap())
            .unwrap();
        assert!(member_names::load(&storage, &db).await.is_err());
        let unchanged =
            sqlx::query_scalar!("SELECT name FROM exposed.members WHERE parliament_member_id=1")
                .fetch_one(&pool)
                .await?;
        assert_eq!(unchanged, "Sir John Smith");
        std::fs::remove_file(storage.path()).unwrap();
        storage
            .publish(&MemberNamesCapture::new(documents).unwrap())
            .unwrap();
        assert_eq!(member_names::load(&storage, &db).await.unwrap(), 2);
        assert_eq!(member_names::load(&storage, &db).await.unwrap(), 2);
        assert!(storage.publish(&storage.read().unwrap()).is_err());
        db.upsert_members(&[mp]).await.unwrap();
        let preserved = sqlx::query!("SELECT name,personal_first_name,personal_last_name,parliamentary_name FROM exposed.members WHERE parliament_member_id=1").fetch_one(&pool).await?;
        assert_eq!(preserved.name, "John Smith");
        assert_eq!(preserved.personal_first_name.as_deref(), Some("John"));
        assert_eq!(preserved.personal_last_name.as_deref(), Some("Smith"));
        assert_eq!(
            preserved.parliamentary_name.as_deref(),
            Some("Sir John Smith")
        );
        for term in ["John Smith", "Sir John Smith", "The Sir John Smith"] {
            let found = db
                .get_members_by_text_search_score(&request(term))
                .await
                .unwrap();
            assert_eq!(found.len(), 1, "{term}");
            assert_eq!(found[0].0.name(), "John Smith");
            assert_eq!(
                found[0].0.profile.parliamentary_name.as_deref(),
                Some("Sir John Smith")
            );
        }
        for term in ["Jahn Smyth", "Sir Jahn Smyth"] {
            let found = db
                .get_members_by_phonetic_search(&request(term))
                .await
                .unwrap();
            assert_eq!(found.len(), 1, "{term}");
            assert_eq!(found[0].0.name(), "John Smith");
        }
        for term in ["Dominic Hubbard", "Lord Addington", "The Lord Addington"] {
            let found = db.search_lords(&request(term)).await.unwrap();
            assert_eq!(found.len(), 1, "{term}");
            assert_eq!(found[0].0.name, "Dominic Hubbard");
            assert_eq!(
                found[0].0.parliamentary_name.as_deref(),
                Some("Lord Addington")
            );
        }
        let legacy = crate::domain::models::membership::MemberProfile {
            id: MemberId::new(2).unwrap(),
            name: "Lord Addington".into(),
            parliamentary_name: None,
            party_id: 1,
            party_name: "Party".into(),
            membership: crate::domain::models::membership::Membership {
                kind: MembershipKind::LifePeer,
                membership_from: "Life peer".into(),
                membership_start_date: None,
                membership_end_date: None,
                membership_status: None,
            },
        };
        let mut tx = pool.begin().await?;
        super::super::member_identity::upsert(&mut tx, &legacy).await?;
        tx.commit().await?;
        assert_eq!(
            db.lord_register(MemberId::new(2).unwrap())
                .await
                .unwrap()
                .unwrap()
                .member
                .name,
            "Dominic Hubbard"
        );
        Ok(())
    }
}
