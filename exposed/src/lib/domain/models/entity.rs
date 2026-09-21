use strum::EnumString;

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    name: EntityName,
    kind: EntityKind,
}

impl Entity {
    pub fn new(name: String, kind: EntityKind) -> Self {
        Self {
            name: EntityName(name),
            kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityName(String);

#[derive(Clone, Debug, EnumString, PartialEq)]
pub enum EntityKind {
    Company,
    ParliamentMember,
}
