//! Representatives of the public in parliament.

#[derive(Clone, Debug)]
/// Member of Parliament
pub struct ParliamentMember {
    name: ParliamentMemberName,
    member_id: MemberId,
    party_name: PartyName,
    constituency: Constituency,
}

impl ParliamentMember {
    /// Creates a new Member of Parliament
    pub fn new(name: String, member_id: usize, party_name: String, constituency: String) -> Self {
        Self {
            name: ParliamentMemberName::new(name),
            member_id: MemberId::new(member_id),
            party_name: PartyName::new(party_name),
            constituency: Constituency::new(constituency),
        }
    }

    /// Member's name
    pub fn name(&self) -> &str {
        &self.name.0
    }
}

#[derive(Clone, Debug)]

/// Member name
pub struct ParliamentMemberName(String);

impl ParliamentMemberName {
    /// Creates member name
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
/// Party name
pub struct PartyName(String);

impl PartyName {
    /// Create party name
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
/// Member constituency
pub struct Constituency(String);

impl Constituency {
    /// Creates member constituency
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
/// Member parliament API ID
pub struct MemberId(usize);

impl MemberId {
    /// Creates member parliament API ID
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}
