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
        member_id: u32,
        party_name: String,
        party_id: u32,
        constituency: String,
    ) -> Self {
        Self {
            name: ParliamentMemberName::new(name),
            member_id: MemberId::new(member_id),
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
    pub const fn member_id(&self) -> u32 {
        self.member_id.0
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Member parliament API ID
#[allow(dead_code)]
pub struct MemberId(u32);

impl MemberId {
    #[must_use]
    /// Creates member parliament API ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl MemberId {
    /// Numeric Parliament identity.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

impl std::str::FromStr for MemberId {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let id = value
            .parse::<u32>()
            .map_err(|_| "Invalid MP ID".to_string())?;
        if id == 0 || i32::try_from(id).is_err() {
            return Err("Invalid MP ID".to_string());
        }
        Ok(Self(id))
    }
}
