//! Representatives of the public in parliament.

/// Member of Parliament
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParliamentMember {
    name: ParliamentMemberName,
    member_id: MemberId,
    party_name: PartyName,
    party_id: PartyId,
    constituency: Constituency,
}

impl ParliamentMember {
    #[must_use]
    /// Creates a new Member of Parliament
    pub const fn new(
        name: String,
        member_id: MemberId,
        party_name: String,
        party_id: u32,
        constituency: String,
    ) -> Self {
        Self {
            name: ParliamentMemberName::new(name),
            member_id,
            party_name: PartyName::new(party_name),
            party_id: PartyId::new(party_id),
            constituency: Constituency::new(constituency),
        }
    }

    #[must_use]
    /// Member's name
    pub fn name(&self) -> &str {
        &self.name.0
    }

    #[must_use]
    /// Member ID
    pub const fn member_id(&self) -> MemberId {
        self.member_id
    }

    #[must_use]
    /// Party name
    pub fn party_name(&self) -> &str {
        &self.party_name.0
    }

    #[must_use]
    /// Party ID
    pub const fn party_id(&self) -> u32 {
        self.party_id.0
    }

    #[must_use]
    /// Constituency
    pub fn constituency(&self) -> &str {
        &self.constituency.0
    }
}

/// Member name
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParliamentMemberName(String);

impl ParliamentMemberName {
    #[must_use]
    /// Creates member name
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Party name
#[allow(dead_code)]
pub struct PartyName(String);

impl PartyName {
    #[must_use]
    /// Create party name
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Member constituency
#[allow(dead_code)]
pub struct Constituency(String);

impl Constituency {
    #[must_use]
    /// Creates member constituency
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Member party ID
#[allow(dead_code)]
pub struct PartyId(u32);

impl PartyId {
    #[must_use]
    /// Creates party ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Member Parliament API identity, representable by the database source-ID column.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "u32", into = "u32")]
pub struct MemberId(u32);

/// A missing or out-of-range Parliament member identity.
#[derive(Debug, thiserror::Error)]
#[error("Invalid MP ID")]
pub struct MemberIdError;

impl MemberId {
    /// Creates a positive Parliament identity.
    ///
    /// # Errors
    /// Rejects zero and values outside the source database column's range.
    pub fn new(id: u32) -> Result<Self, MemberIdError> {
        if id == 0 || i32::try_from(id).is_err() {
            return Err(MemberIdError);
        }
        Ok(Self(id))
    }

    /// Numeric Parliament identity.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for MemberId {
    type Error = MemberIdError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<MemberId> for u32 {
    fn from(value: MemberId) -> Self {
        value.value()
    }
}
impl std::fmt::Display for MemberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::str::FromStr for MemberId {
    type Err = MemberIdError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value.parse().map_err(|_| MemberIdError)?)
    }
}

#[cfg(test)]
mod tests {
    use super::MemberId;

    #[test]
    fn member_identity_boundaries_share_the_checked_numeric_contract() {
        for invalid in [0, i32::MAX.cast_unsigned() + 1, u32::MAX] {
            assert!(MemberId::new(invalid).is_err());
            assert!(invalid.to_string().parse::<MemberId>().is_err());
            assert!(serde_json::from_value::<MemberId>(serde_json::json!(invalid)).is_err());
        }
        for valid in [1, 4613, i32::MAX.cast_unsigned()] {
            let id = MemberId::new(valid).unwrap();
            assert_eq!(id.value(), valid);
            assert_eq!(valid.to_string().parse::<MemberId>().unwrap(), id);
            assert_eq!(serde_json::to_value(id).unwrap(), serde_json::json!(valid));
            assert_eq!(
                serde_json::from_value::<MemberId>(serde_json::json!(valid)).unwrap(),
                id
            );
        }
        for invalid in [
            serde_json::json!("00000000-0000-0000-0000-000000000001"),
            serde_json::json!("4613"),
            serde_json::json!(-1),
            serde_json::json!(1.5),
        ] {
            assert!(serde_json::from_value::<MemberId>(invalid).is_err());
        }
    }
}
