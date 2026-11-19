//! Shared capture and offline replay for both parliamentary chambers.
use crate::domain::{
    models::{
        member_names::{MemberNamesCapture, MemberNamesError, check_coverage, parse_document},
        membership::Chamber,
    },
    repositories::member_names::{MemberNamesRepository, MemberNamesSource, MemberNamesStorage},
};

/// Captures current chamber documents and missing historical identities before publication.
///
/// # Errors
/// Any malformed, incomplete, or unavailable source leaves no published capture.
pub async fn fetch(
    source: &impl MemberNamesSource,
    storage: &impl MemberNamesStorage,
    repository: &impl MemberNamesRepository,
) -> Result<usize, MemberNamesError> {
    let targets = repository.targets().await?;
    let mut documents = Vec::new();
    let mut captured = std::collections::BTreeSet::new();
    for chamber in [Chamber::Commons, Chamber::Lords] {
        let raw = source.chamber(chamber).await?;
        let names = parse_document(&raw)?;
        if names.iter().any(|name| name.chamber != chamber) {
            return Err(MemberNamesError::Source(
                "bulk response has the wrong chamber".into(),
            ));
        }
        captured.extend(names.into_iter().map(|name| name.id));
        documents.push(raw);
    }
    for (id, chamber) in &targets {
        if !captured.contains(id) {
            let raw = source.member(*id).await?;
            let names = parse_document(&raw)?;
            if names.len() != 1 || names[0].id != *id || names[0].chamber != *chamber {
                return Err(MemberNamesError::Source(format!(
                    "unexpected member response for {id}"
                )));
            }
            documents.push(raw);
        }
    }
    let capture = MemberNamesCapture::new(documents)?;
    check_coverage(&targets, &capture.names()?)?;
    storage.publish(&capture)?;
    Ok(targets.len())
}

/// Replays the entire retained capture without source access.
///
/// # Errors
/// Malformed or incomplete evidence writes no names.
pub async fn load(
    storage: &impl MemberNamesStorage,
    repository: &impl MemberNamesRepository,
) -> Result<usize, MemberNamesError> {
    let capture = storage.read()?;
    repository.refresh(&capture.names()?).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{member_names::MemberNames, parliament_member::MemberId};
    use std::{
        collections::BTreeMap,
        sync::atomic::{AtomicBool, Ordering},
    };
    struct Source {
        fail_lords: bool,
    }
    fn document(id: u32, chamber: Chamber) -> String {
        serde_json::json!({"Members":{"Member":{"@Member_Id":id.to_string(),"House":chamber.name(),"DisplayAs":"Lord Example","FullTitle":"The Lord Example","PreferredNames":{"PreferredName":{"Forename":"Personal","Surname":"Example","StartDate":"2020-01-01T00:00:00","EndDate":{"@xsi:nil":"true"}}}}}}).to_string()
    }
    impl MemberNamesSource for Source {
        fn chamber(
            &self,
            chamber: Chamber,
        ) -> impl Future<Output = Result<String, MemberNamesError>> {
            std::future::ready(match chamber {
                Chamber::Commons => Ok(document(1, chamber)),
                Chamber::Lords if self.fail_lords => {
                    Err(MemberNamesError::Source("unavailable".into()))
                }
                Chamber::Lords => Ok(document(2, chamber)),
            })
        }
        fn member(&self, id: MemberId) -> impl Future<Output = Result<String, MemberNamesError>> {
            std::future::ready(Ok(document(id.value(), Chamber::Commons)))
        }
    }
    struct Storage {
        published: AtomicBool,
    }
    impl MemberNamesStorage for Storage {
        fn publish(&self, _: &MemberNamesCapture) -> Result<(), MemberNamesError> {
            self.published.store(true, Ordering::Relaxed);
            Ok(())
        }
        fn read(&self) -> Result<MemberNamesCapture, MemberNamesError> {
            MemberNamesCapture::new(vec![document(1, Chamber::Commons)])
        }
    }
    struct Repository;
    impl MemberNamesRepository for Repository {
        fn targets(
            &self,
        ) -> impl Future<Output = Result<Vec<(MemberId, Chamber)>, MemberNamesError>> {
            std::future::ready(Ok(vec![
                (MemberId::new(1).unwrap(), Chamber::Commons),
                (MemberId::new(2).unwrap(), Chamber::Lords),
                (MemberId::new(3).unwrap(), Chamber::Commons),
            ]))
        }
        async fn refresh(
            &self,
            names: &BTreeMap<MemberId, MemberNames>,
        ) -> Result<usize, MemberNamesError> {
            check_coverage(&self.targets().await?, names)?;
            Ok(names.len())
        }
    }
    #[tokio::test]
    async fn source_failure_does_not_publish_partial_evidence() {
        let storage = Storage {
            published: AtomicBool::new(false),
        };
        assert!(
            fetch(&Source { fail_lords: true }, &storage, &Repository)
                .await
                .is_err()
        );
        assert!(!storage.published.load(Ordering::Relaxed));
    }
    #[tokio::test]
    async fn captures_missing_historical_members_and_load_checks_full_coverage() {
        let storage = Storage {
            published: AtomicBool::new(false),
        };
        assert_eq!(
            fetch(&Source { fail_lords: false }, &storage, &Repository)
                .await
                .unwrap(),
            3
        );
        assert!(storage.published.load(Ordering::Relaxed));
        assert!(load(&storage, &Repository).await.is_err());
    }
}
