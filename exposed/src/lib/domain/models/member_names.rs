//! Complete preferred personal names from Parliament MNIS, retaining raw evidence.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{membership::Chamber, parliament_member::MemberId, source_timestamp::SourceTimestamp};

/// A complete preferred personal name. Partial or blank names cannot be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalName {
    first: String,
    last: String,
}
impl PersonalName {
    /// Parses both official preferred-name parts without inferring or splitting titles.
    ///
    /// # Errors
    /// Rejects either missing name part.
    pub fn new(first: &str, last: &str) -> Result<Self, MemberNamesError> {
        Ok(Self {
            first: nonempty(first)?,
            last: nonempty(last)?,
        })
    }
    /// Preferred first name, including compound parts.
    #[must_use]
    pub fn first(&self) -> &str {
        &self.first
    }
    /// Preferred surname, including compound parts.
    #[must_use]
    pub fn last(&self) -> &str {
        &self.last
    }
    /// Primary public name.
    #[must_use]
    pub fn display(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
}

/// Parsed official names for a single Parliament identity.
#[derive(Clone, Debug)]
pub struct MemberNames {
    /// Stable Parliament identity.
    pub id: MemberId,
    /// Source chamber used to check the stored membership.
    pub chamber: Chamber,
    /// Complete preferred personal name.
    pub personal: PersonalName,
    /// Source parliamentary display spelling.
    pub parliamentary: String,
    /// Source full parliamentary title.
    pub formal: String,
}

/// Source, capture, and repository failures remain explicit.
#[derive(Debug, thiserror::Error)]
pub enum MemberNamesError {
    /// Invalid or incomplete source evidence.
    #[error("member names source: {0}")]
    Source(String),
    /// Capture cannot be read or published.
    #[error("member names capture: {0}")]
    Capture(String),
    /// Database failure.
    #[error("member names database: {0}")]
    Database(String),
}

/// Fetch and replay stages share the standard data CLI.
#[derive(Clone, Debug, Serialize, schemars::JsonSchema, strum::EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum MemberNamesStage {
    /// Capture complete official source evidence.
    Fetch,
    /// Refresh stored names from a retained capture, offline.
    Load,
}

/// Raw official responses. Parsing always precedes publication and loading.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberNamesCapture {
    version: u8,
    documents: Vec<String>,
}
impl MemberNamesCapture {
    /// Constructs a replayable capture only from valid source documents.
    ///
    /// # Errors
    /// Rejects malformed documents and duplicate Parliament identities.
    pub fn new(documents: Vec<String>) -> Result<Self, MemberNamesError> {
        let capture = Self {
            version: 1,
            documents,
        };
        capture.names()?;
        Ok(capture)
    }
    /// Parses all retained source evidence.
    ///
    /// # Errors
    /// Rejects unsupported captures, incomplete names, or ambiguous active names.
    pub fn names(&self) -> Result<BTreeMap<MemberId, MemberNames>, MemberNamesError> {
        if self.version != 1 || self.documents.is_empty() {
            return Err(MemberNamesError::Capture(
                "unsupported or empty capture".into(),
            ));
        }
        let mut names = BTreeMap::new();
        for document in &self.documents {
            for member in parse_document(document)? {
                if names.insert(member.id, member).is_some() {
                    return Err(MemberNamesError::Source(
                        "duplicate Parliament identity".into(),
                    ));
                }
            }
        }
        Ok(names)
    }
}

/// Checks coverage against stored identities. No import may partially refresh the database.
///
/// # Errors
/// Rejects a missing identity or a mismatched chamber.
pub fn check_coverage(
    targets: &[(MemberId, Chamber)],
    names: &BTreeMap<MemberId, MemberNames>,
) -> Result<(), MemberNamesError> {
    for (id, chamber) in targets {
        if !names.get(id).is_some_and(|name| name.chamber == *chamber) {
            return Err(MemberNamesError::Source(format!(
                "missing preferred name for {chamber:?} member {id}"
            )));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Rows<T> {
    Many(Vec<T>),
    One(T),
}
impl<T> Rows<T> {
    fn into_vec(self) -> Vec<T> {
        match self {
            Self::Many(rows) => rows,
            Self::One(row) => vec![row],
        }
    }
}
#[derive(Deserialize)]
struct Document {
    #[serde(rename = "Members")]
    members: Members,
}
#[derive(Deserialize)]
struct Members {
    #[serde(rename = "Member")]
    member: Rows<SourceMember>,
}
#[derive(Deserialize)]
struct SourceMember {
    #[serde(rename = "@Member_Id")]
    id: String,
    #[serde(rename = "House")]
    chamber: Chamber,
    #[serde(rename = "DisplayAs")]
    display: String,
    #[serde(rename = "FullTitle")]
    formal: String,
    #[serde(rename = "PreferredNames")]
    names: PreferredNames,
}
#[derive(Deserialize)]
struct PreferredNames {
    #[serde(rename = "PreferredName")]
    names: Rows<PreferredName>,
}
#[derive(Deserialize)]
struct PreferredName {
    #[serde(rename = "Forename")]
    first: Option<String>,
    #[serde(rename = "Surname")]
    last: Option<String>,
    #[serde(rename = "StartDate")]
    _start: SourceTimestamp,
    #[serde(rename = "EndDate")]
    end: EndDate,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum EndDate {
    Ended(SourceTimestamp),
    Open {
        #[serde(rename = "@xsi:nil")]
        nil: String,
    },
}
impl EndDate {
    fn is_open(&self) -> Result<bool, MemberNamesError> {
        match self {
            Self::Ended(date) => {
                let _ = date.as_str();
                Ok(false)
            }
            Self::Open { nil } if nil == "true" => Ok(true),
            Self::Open { .. } => Err(MemberNamesError::Source(
                "invalid end-date nil marker".into(),
            )),
        }
    }
}
fn nonempty(value: &str) -> Result<String, MemberNamesError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(MemberNamesError::Source("blank name part".into()));
    }
    Ok(value.into())
}

/// Decodes MNIS singleton/collection responses, including their UTF-8 BOM.
///
/// # Errors
/// Requires exactly one explicitly open preferred name per member.
pub fn parse_document(raw: &str) -> Result<Vec<MemberNames>, MemberNamesError> {
    let document: Document = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|error| MemberNamesError::Source(error.to_string()))?;
    let rows = document.members.member.into_vec();
    if rows.is_empty() {
        return Err(MemberNamesError::Source("empty member response".into()));
    }
    rows.into_iter()
        .map(|row| {
            let id = row
                .id
                .parse()
                .map_err(|error| MemberNamesError::Source(format!("invalid member ID: {error}")))?;
            let mut active = None;
            for name in row.names.names.into_vec() {
                if name.end.is_open()? {
                    if active.is_some() {
                        return Err(MemberNamesError::Source(format!(
                            "multiple active preferred names for {id}"
                        )));
                    }
                    active = Some(PersonalName::new(
                        name.first.as_deref().ok_or_else(|| {
                            MemberNamesError::Source(format!(
                                "missing preferred first name for {id}"
                            ))
                        })?,
                        name.last.as_deref().ok_or_else(|| {
                            MemberNamesError::Source(format!("missing preferred surname for {id}"))
                        })?,
                    )?);
                }
            }
            Ok(MemberNames {
                id,
                chamber: row.chamber,
                personal: active.ok_or_else(|| {
                    MemberNamesError::Source(format!("no active preferred name for {id}"))
                })?,
                parliamentary: nonempty(&row.display)?,
                formal: nonempty(&row.formal)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn member() -> serde_json::Value {
        json!({"@Member_Id":"4005","House":"Commons","DisplayAs":"Bob Blackman","FullTitle":"Bob Blackman MP","PreferredNames":{"PreferredName":{"Forename":" Bob ","Surname":"Blackman","StartDate":"2010-05-06T00:00:00","EndDate":{"@xsi:nil":"true"}}}})
    }
    #[test]
    fn preferred_parts_preserve_compounds_and_ignore_formal_names() {
        let raw = format!("\u{feff}{}", json!({"Members":{"Member":member()}}));
        let names = parse_document(&raw).unwrap();
        assert_eq!(names[0].personal.display(), "Bob Blackman");
        assert_eq!(names[0].formal, "Bob Blackman MP");
        assert_eq!(
            PersonalName::new("Anne Marie", "de Souza")
                .unwrap()
                .display(),
            "Anne Marie de Souza"
        );
        assert!(PersonalName::new(" ", "Surname").is_err());
        let mut row = member();
        let active = row["PreferredNames"]["PreferredName"].clone();
        let mut ended = active.clone();
        ended["Forename"] = json!(null);
        ended["EndDate"] = json!("2010-05-05T00:00:00");
        row["PreferredNames"]["PreferredName"] = json!([ended, active]);
        assert_eq!(
            parse_document(&json!({"Members":{"Member":[row]}}).to_string()).unwrap()[0]
                .personal
                .first(),
            "Bob"
        );
    }
    #[test]
    fn missing_ambiguous_or_malformed_active_names_fail() {
        let row = member();
        for end in [
            json!(null),
            json!({"@xsi:nil":"false"}),
            json!("invalid"),
            json!("2020-01-01T00:00:00"),
        ] {
            let mut row = row.clone();
            row["PreferredNames"]["PreferredName"]["EndDate"] = end;
            assert!(parse_document(&json!({"Members":{"Member":row}}).to_string()).is_err());
        }
        for part in ["Forename", "Surname"] {
            for value in [json!(null), json!("  ")] {
                let mut row = member();
                row["PreferredNames"]["PreferredName"][part] = value;
                assert!(parse_document(&json!({"Members":{"Member":row}}).to_string()).is_err());
            }
        }
        let mut row = member();
        let active = row["PreferredNames"]["PreferredName"].clone();
        row["PreferredNames"]["PreferredName"] = json!([active, active]);
        assert!(parse_document(&json!({"Members":{"Member":row}}).to_string()).is_err());
        let raw = json!({"Members":{"Member":member()}}).to_string();
        assert!(MemberNamesCapture::new(vec![raw.clone(), raw]).is_err());
    }
}
