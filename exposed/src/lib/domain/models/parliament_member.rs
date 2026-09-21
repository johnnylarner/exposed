//! Representatives of the public in parliament.

#[derive(Clone, Debug)]
pub struct ParliamentMember {
    name: ParliamentMemberName,
    member_id: MemberId,
    party_name: PartyName,
    constituency: Constituency,
}

impl ParliamentMember {
    pub fn new(name: String, member_id: usize, party_name: String, constituency: String) -> Self {
        Self {
            name: ParliamentMemberName::new(name),
            member_id: MemberId::new(member_id),
            party_name: PartyName::new(party_name),
            constituency: Constituency::new(constituency),
        }
    }

    pub fn name(&self) -> &str {
        &self.name.0
    }
}

#[derive(Clone, Debug)]
pub struct ParliamentMemberName(String);

impl ParliamentMemberName {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
pub struct PartyName(String);

impl PartyName {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
pub struct Constituency(String);

impl Constituency {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
pub struct MemberId(usize);

impl MemberId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}
