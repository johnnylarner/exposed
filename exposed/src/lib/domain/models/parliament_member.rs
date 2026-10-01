//! Representatives of the public in parliament.

/// Member of Parliament
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParliamentMember {
    name: ParliamentMemberName,
    _member_id: MemberId,
    _party_name: PartyName,
    _constituency: Constituency,
}

impl ParliamentMember {
    #[must_use]
    /// Creates a new Member of Parliament
    pub const fn new(
        name: String,
        member_id: usize,
        party_name: String,
        constituency: String,
    ) -> Self {
        Self {
            name: ParliamentMemberName::new(name),
            _member_id: MemberId::new(member_id),
            _party_name: PartyName::new(party_name),
            _constituency: Constituency::new(constituency),
        }
    }

    #[must_use]
    /// Member's name
    pub fn name(&self) -> &str {
        &self.name.0
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
/// Member parliament API ID
#[allow(dead_code)]
pub struct MemberId(usize);

impl MemberId {
    #[must_use]
    /// Creates member parliament API ID
    pub const fn new(id: usize) -> Self {
        Self(id)
    }
}
