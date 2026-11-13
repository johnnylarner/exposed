//! Shared latest membership facts from Parliament, independent of imported registers.
use serde::{Deserialize, Serialize};

use super::{parliament_member::MemberId, source_timestamp::SourceTimestamp};

/// Chamber derived from the membership kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chamber {
    /// House of Commons.
    Commons,
    /// House of Lords.
    Lords,
}

/// Local classification. External subtype identifiers are chamber-specific.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MembershipKind {
    /// Elected Commons representative.
    ElectedMp,
    /// Life peer.
    LifePeer,
    /// Hereditary peer.
    HereditaryPeer,
    /// Excepted hereditary peer.
    ExceptedHereditaryPeer,
    /// Hereditary peer subsequently granted a life peerage.
    HereditaryLifePeer,
    /// Lords spiritual.
    Bishop,
    /// Judicial peer.
    JudicialPeer,
    /// Lords membership with an unrecognised or absent subtype.
    OtherLords,
    /// Chamber is not supplied.
    Unknown,
}
impl MembershipKind {
    /// Chamber implied by this kind.
    #[must_use]
    pub const fn chamber(self) -> Option<Chamber> {
        match self {
            Self::ElectedMp => Some(Chamber::Commons),
            Self::LifePeer
            | Self::HereditaryPeer
            | Self::ExceptedHereditaryPeer
            | Self::HereditaryLifePeer
            | Self::Bishop
            | Self::JudicialPeer
            | Self::OtherLords => Some(Chamber::Lords),
            Self::Unknown => None,
        }
    }
}

/// Provenance of membership classification, distinct from source activity.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MembershipStatusProvenance {
    /// Supplied membership from Parliament, with independently optional status.
    #[default]
    Source,
    /// Original database classified the Commons membership as current.
    LegacyCurrent,
    /// Original database classified the Commons membership as former, without dates.
    LegacyFormer,
}

/// Shared source membership. Missing activity is unknown, never inferred from a missing end date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Membership {
    /// Canonical kind, retaining unknown source kinds as `OtherLords`.
    #[serde(skip_serializing)]
    pub kind: MembershipKind,
    /// Origin of a known legacy classification.
    #[serde(skip_serializing)]
    pub status_provenance: MembershipStatusProvenance,
    /// Constituency or source membership description.
    pub membership_from: Option<String>,
    /// Source start timestamp, without an invented timezone.
    #[serde(skip_serializing)]
    pub membership_start_date: Option<SourceTimestamp>,
    /// Source end timestamp.
    pub membership_end_date: Option<SourceTimestamp>,
    /// Source status description, independent of activity and end date.
    pub membership_status: Option<String>,
    /// Source activity flag, including unknown.
    #[serde(skip_serializing)]
    pub membership_status_active: Option<bool>,
}
impl Membership {
    /// Whether the source establishes an unended Commons membership.
    #[must_use]
    pub fn is_current_commons(&self) -> Option<bool> {
        Self::current_commons(
            self.kind,
            self.status_provenance,
            self.membership_end_date
                .as_ref()
                .map(SourceTimestamp::as_str),
        )
    }
    /// Projection shared by persisted member summaries.
    #[must_use]
    pub fn current_commons(
        kind: MembershipKind,
        provenance: MembershipStatusProvenance,
        end: Option<&str>,
    ) -> Option<bool> {
        kind.chamber().map(|chamber| {
            chamber == Chamber::Commons
                && provenance != MembershipStatusProvenance::LegacyFormer
                && end.is_none()
        })
    }
}

/// One canonical latest member profile, shared by all register projections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemberProfile {
    /// Parliament identity; public register JSON uses a string.
    #[serde(serialize_with = "serialize_member_id")]
    pub id: MemberId,
    /// Source display name.
    pub name: String,
    /// Latest source party identity.
    #[serde(skip)]
    pub party_id: Option<u32>,
    /// Latest source party name.
    pub party_name: Option<String>,
    /// Shared latest membership facts.
    #[serde(flatten)]
    pub membership: Membership,
}
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "Serde field serializers receive references"
)]
fn serialize_member_id<S: serde::Serializer>(
    id: &MemberId,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&id.to_string())
}

/// Parliament's independently versioned external membership shape.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMembership {
    house: Option<u32>,
    membership_from_id: Option<u32>,
    membership_from: Option<String>,
    membership_start_date: Option<SourceTimestamp>,
    membership_end_date: Option<SourceTimestamp>,
    membership_status: Option<SourceStatus>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceStatus {
    status_description: Option<String>,
    status_is_active: Option<bool>,
}
impl SourceMembership {
    /// Parse a source membership without treating Commons constituency IDs as peerage kinds.
    ///
    /// # Errors
    /// Rejects an unsupported chamber identity.
    pub fn into_membership(self) -> Result<Membership, String> {
        let kind = match self.house {
            Some(1) => MembershipKind::ElectedMp,
            Some(2) => match self.membership_from_id {
                Some(2) => MembershipKind::HereditaryPeer,
                Some(4) => MembershipKind::LifePeer,
                Some(5) => MembershipKind::Bishop,
                Some(6) => MembershipKind::JudicialPeer,
                Some(9) => MembershipKind::HereditaryLifePeer,
                Some(10) => MembershipKind::ExceptedHereditaryPeer,
                None => match self
                    .membership_from
                    .as_deref()
                    .map(str::trim)
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                {
                    Some("life peer") => MembershipKind::LifePeer,
                    Some("hereditary" | "hereditary peer") => MembershipKind::HereditaryPeer,
                    Some("bishops" | "bishop") => MembershipKind::Bishop,
                    Some("life peer (judicial)" | "judicial peer") => MembershipKind::JudicialPeer,
                    Some("hereds given lps" | "hereditary peer given life peerage") => {
                        MembershipKind::HereditaryLifePeer
                    }
                    Some("excepted hereditary" | "excepted hereditary peer") => {
                        MembershipKind::ExceptedHereditaryPeer
                    }
                    _ => MembershipKind::OtherLords,
                },
                _ => MembershipKind::OtherLords,
            },
            None => MembershipKind::Unknown,
            Some(house) => return Err(format!("unsupported Parliament house {house}")),
        };
        let (description, active) = self.membership_status.map_or((None, None), |status| {
            (status.status_description, status.status_is_active)
        });
        Ok(Membership {
            kind,
            status_provenance: MembershipStatusProvenance::Source,
            membership_from: self.membership_from,
            membership_start_date: self.membership_start_date,
            membership_end_date: self.membership_end_date,
            membership_status: description,
            membership_status_active: active,
        })
    }
}
impl Default for Membership {
    fn default() -> Self {
        Self {
            kind: MembershipKind::Unknown,
            status_provenance: MembershipStatusProvenance::Source,
            membership_from: None,
            membership_start_date: None,
            membership_end_date: None,
            membership_status: None,
            membership_status_active: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn membership(value: serde_json::Value) -> Membership {
        serde_json::from_value::<SourceMembership>(value)
            .unwrap()
            .into_membership()
            .unwrap()
    }

    #[test]
    fn activity_end_dates_and_legacy_classifications_are_independent() {
        for activity in [json!(true), json!(false), json!(null)] {
            let mut open = membership(
                json!({"house":1,"membershipFrom":"Example constituency","membershipFromId":4,"membershipStatus":{"statusIsActive":activity}}),
            );
            assert_eq!(open.kind, MembershipKind::ElectedMp);
            assert_eq!(open.is_current_commons(), Some(true));
            open.membership_end_date = Some("2026-01-01T00:00:00".to_string().try_into().unwrap());
            assert_eq!(open.is_current_commons(), Some(false));
            open.membership_end_date = None;
            open.status_provenance = MembershipStatusProvenance::LegacyFormer;
            assert_eq!(open.is_current_commons(), Some(false));
            assert!(open.membership_end_date.is_none());
        }
        assert_eq!(Membership::default().is_current_commons(), None);
    }

    #[test]
    fn chamber_scopes_external_kind_ids_and_preserves_unknown_descriptions() {
        for (id, kind) in [
            (2, MembershipKind::HereditaryPeer),
            (4, MembershipKind::LifePeer),
            (5, MembershipKind::Bishop),
            (6, MembershipKind::JudicialPeer),
            (9, MembershipKind::HereditaryLifePeer),
            (10, MembershipKind::ExceptedHereditaryPeer),
            (999, MembershipKind::OtherLords),
        ] {
            let lords = membership(
                json!({"house":2,"membershipFromId":id,"membershipFrom":"Original source label"}),
            );
            assert_eq!(lords.kind, kind);
            assert_eq!(
                lords.membership_from.as_deref(),
                Some("Original source label")
            );
            assert_eq!(lords.kind.chamber(), Some(Chamber::Lords));
            assert_eq!(lords.is_current_commons(), Some(false));
            assert_eq!(
                membership(json!({"house":1,"membershipFromId":id})).kind,
                MembershipKind::ElectedMp
            );
        }
        assert_eq!(
            membership(json!({"house":2,"membershipFrom":"Hereditary peer"})).kind,
            MembershipKind::HereditaryPeer
        );
        assert!(
            serde_json::from_value::<SourceMembership>(
                json!({"house":2,"membershipStartDate":"yesterday"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<SourceMembership>(json!({"house":3}))
                .unwrap()
                .into_membership()
                .is_err()
        );
    }
}
