//! Validated members used by both search and member ingestion.
use thiserror::Error;

/// Member of Parliament with a complete, valid source profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParliamentMember {
    name: String,
    member_id: i32,
    party_id: i32,
    party_name: String,
    latest_house: i16,
    latest_membership_from: String,
}

impl ParliamentMember {
    /// Create a member with all required profile fields.
    ///
    /// # Errors
    /// Rejects invalid identities, blank profile fields, or an unknown House.
    pub fn new(
        name: String,
        parliament_member_id: i32,
        party_id: i32,
        party_name: String,
        latest_house: i16,
        latest_membership_from: String,
    ) -> Result<Self, ParliamentMemberError> {
        for (valid, field) in [
            (parliament_member_id > 0, "parliament_member_id"),
            (!name.trim().is_empty(), "name"),
            (party_id > 0, "party_id"),
            (!party_name.trim().is_empty(), "party_name"),
            (matches!(latest_house, 1 | 2), "latest_house"),
            (
                !latest_membership_from.trim().is_empty(),
                "latest_membership_from",
            ),
        ] {
            if !valid {
                return Err(ParliamentMemberError::InvalidField {
                    member_id: parliament_member_id,
                    field,
                });
            }
        }
        Ok(Self {
            name,
            member_id: parliament_member_id,
            party_id,
            party_name,
            latest_house,
            latest_membership_from,
        })
    }
    /// Member's source display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Parliament's numeric member identity.
    #[must_use]
    pub const fn parliament_member_id(&self) -> i32 {
        self.member_id
    }
    /// Required latest party identity.
    #[must_use]
    pub const fn party_id(&self) -> i32 {
        self.party_id
    }
    /// Required latest party name.
    #[must_use]
    pub fn party_name(&self) -> &str {
        &self.party_name
    }
    /// Latest House from the member profile.
    #[must_use]
    pub const fn latest_house(&self) -> i16 {
        self.latest_house
    }
    /// Required latest membership location.
    #[must_use]
    pub fn latest_membership_from(&self) -> &str {
        &self.latest_membership_from
    }
}

/// A member profile cannot be represented by a valid domain member.
#[derive(Error, Debug)]
pub enum ParliamentMemberError {
    /// A required member profile value is invalid.
    #[error("member {member_id}: invalid {field}")]
    InvalidField {
        /// Parliament's numeric member identity.
        member_id: i32,
        /// Invalid source field.
        field: &'static str,
    },
}
