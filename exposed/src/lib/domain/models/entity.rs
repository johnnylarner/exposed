use strum::EnumString;

#[derive(Clone, Debug, PartialEq)]
/// Search entity result
pub struct Entity {
    name: EntityName,
    kind: EntityKind,
}

impl Entity {
    /// Creates a new entity
    pub fn new(name: String, kind: EntityKind) -> Self {
        Self {
            name: EntityName(name),
            kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Search entity name
pub struct EntityName(String);

#[derive(Clone, Debug, EnumString, PartialEq)]
/// Search entity kind
pub enum EntityKind {
    /// Company entity
    Company,
    /// Member entitiy
    ParliamentMember,
}
