//! Shared latest membership facts from Parliament.
use super::{parliament_member::MemberId, source_timestamp::SourceTimestamp};
use serde::{Deserialize, Serialize};

/// Chamber derived from the membership kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chamber {
    /// House of Commons.
    Commons,
    /// House of Lords.
    Lords,
}

impl Chamber {
    /// Canonical database name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Commons => "Commons",
            Self::Lords => "Lords",
        }
    }
}

/// Canonical local membership classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, strum::EnumIter)]
#[serde(rename_all = "snake_case")]
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
}
impl MembershipKind {
    /// Canonical database name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ElectedMp => "Elected MP",
            Self::LifePeer => "Life peer",
            Self::HereditaryPeer => "Hereditary peer",
            Self::ExceptedHereditaryPeer => "Excepted hereditary peer",
            Self::HereditaryLifePeer => "Hereditary peer given life peerage",
            Self::Bishop => "Bishop",
            Self::JudicialPeer => "Judicial peer",
        }
    }

    /// Chamber implied by this kind.
    #[must_use]
    pub const fn chamber(self) -> Chamber {
        match self {
            Self::ElectedMp => Chamber::Commons,
            Self::LifePeer
            | Self::HereditaryPeer
            | Self::ExceptedHereditaryPeer
            | Self::HereditaryLifePeer
            | Self::Bishop
            | Self::JudicialPeer => Chamber::Lords,
        }
    }
}
impl TryFrom<&str> for MembershipKind {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use strum::IntoEnumIterator;
        Self::iter()
            .find(|kind| kind.name() == value)
            .ok_or_else(|| format!("unknown membership kind {value}"))
    }
}

/// Latest source membership, retaining optional lifecycle dates and status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Membership {
    /// Canonical kind.
    #[serde(skip_serializing)]
    pub kind: MembershipKind,
    /// Constituency or source membership description.
    pub membership_from: String,
    /// Source start timestamp.
    #[serde(skip_serializing)]
    pub membership_start_date: Option<SourceTimestamp>,
    /// Source end timestamp.
    pub membership_end_date: Option<SourceTimestamp>,
    /// Source status description.
    pub membership_status: Option<String>,
}
impl Membership {
    /// Whether the member has an unended Commons membership.
    #[must_use]
    pub fn is_current_commons(&self) -> bool {
        Self::current_commons(
            self.kind,
            self.membership_end_date
                .as_ref()
                .map(SourceTimestamp::as_str),
        )
    }
    /// Projection shared by persisted member summaries.
    #[must_use]
    pub const fn current_commons(kind: MembershipKind, end: Option<&str>) -> bool {
        matches!(kind.chamber(), Chamber::Commons) && end.is_none()
    }
}

/// Canonical latest member profile shared by register projections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemberProfile {
    /// Parliament identity; public register JSON uses a string.
    #[serde(serialize_with = "serialize_member_id")]
    pub id: MemberId,
    /// Source display name.
    pub name: String,
    /// Latest source party identity.
    #[serde(skip)]
    pub party_id: u32,
    /// Latest source party name.
    pub party_name: String,
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

/// Parliament's external membership shape.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMembership {
    house: u32,
    membership_from_id: u32,
    membership_from: String,
    membership_start_date: Option<SourceTimestamp>,
    membership_end_date: Option<SourceTimestamp>,
    membership_status: Option<SourceStatus>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceStatus {
    status_description: Option<String>,
}
impl SourceMembership {
    /// Parses source membership, scoping subtype IDs to their chamber.
    ///
    /// # Errors
    /// Rejects an unsupported chamber.
    pub fn into_membership(self) -> Result<Membership, String> {
        let kind = match self.house {
            1 => MembershipKind::ElectedMp,
            2 => match self.membership_from_id {
                2 | 3 => MembershipKind::HereditaryPeer,
                4 => MembershipKind::LifePeer,
                5 => MembershipKind::Bishop,
                6 => MembershipKind::JudicialPeer,
                9 => MembershipKind::HereditaryLifePeer,
                10 => MembershipKind::ExceptedHereditaryPeer,
                id => return Err(format!("unsupported Lords membership kind {id}")),
            },
            house => return Err(format!("unsupported Parliament house {house}")),
        };
        Ok(Membership {
            kind,
            membership_from: self.membership_from,
            membership_start_date: self.membership_start_date,
            membership_end_date: self.membership_end_date,
            membership_status: self
                .membership_status
                .and_then(|status| status.status_description),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn stored_kind_names_round_trip_and_reject_unknown_names() {
        use strum::IntoEnumIterator;
        for kind in MembershipKind::iter() {
            assert_eq!(MembershipKind::try_from(kind.name()), Ok(kind));
        }
        assert!(MembershipKind::try_from("Unknown kind").is_err());
        assert!(MembershipKind::try_from(Chamber::Commons.name()).is_err());
    }
    #[test]
    fn source_kind_ids_are_scoped_to_the_chamber() {
        for (id, kind) in [
            (2, MembershipKind::HereditaryPeer),
            (3, MembershipKind::HereditaryPeer),
            (4, MembershipKind::LifePeer),
            (5, MembershipKind::Bishop),
            (6, MembershipKind::JudicialPeer),
            (9, MembershipKind::HereditaryLifePeer),
            (10, MembershipKind::ExceptedHereditaryPeer),
        ] {
            for (house, expected) in [(1, MembershipKind::ElectedMp), (2, kind)] {
                let source: SourceMembership = serde_json::from_value(
                    json!({"house":house,"membershipFromId":id,"membershipFrom":"Source label"}),
                )
                .unwrap();
                assert_eq!(source.into_membership().unwrap().kind, expected);
            }
        }
    }
    #[test]
    fn invalid_or_missing_required_membership_fields_fail_parsing() {
        let source = json!({"house":2,"membershipFromId":4,"membershipFrom":"Life peer"});
        for field in ["house", "membershipFromId", "membershipFrom"] {
            let mut malformed = source.clone();
            malformed.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<SourceMembership>(malformed).is_err());
        }
        let unsupported: SourceMembership = serde_json::from_value(
            json!({"house":2,"membershipFromId":999,"membershipFrom":"Unsupported kind"}),
        )
        .unwrap();
        assert!(unsupported.into_membership().is_err());
    }
}
