//! Funders give financial compensation to [`parliament_members`]

use std::str::FromStr;
use uuid::Uuid;

use strum::{AsRefStr, EnumString};

#[derive(Clone, Debug, PartialEq, Eq)]
/// Funder of MP declarations
pub struct Funder {
    id: FunderId,
    name: String,
    kind: FunderKind,
}

impl Funder {
    #[must_use]
    /// Creates a new instance
    pub const fn new(id: FunderId, name: String, funder_kind: FunderKind) -> Self {
        Self {
            id,
            name,
            kind: funder_kind,
        }
    }

    /// Persistent funder identity.
    #[must_use]
    pub const fn id(&self) -> FunderId {
        self.id
    }

    #[must_use]
    /// Gets funder name
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Gets legal person
    pub fn kind(&self) -> &str {
        self.kind.as_ref()
    }
}

#[derive(Clone, Debug, AsRefStr, EnumString, PartialEq, Eq)]
/// All possible funder kinds
#[allow(missing_docs)] // Self explanatory
pub enum FunderKind {
    Company,
    Individual,
    Trust,
    #[strum(serialize = "Trade Union")]
    TradeUnion,
    Other,
    /// We don't know what this is supposed to be
    #[strum(serialize = "Not Specified")]
    NotSpecified,
    #[strum(serialize = "Building society")]
    BuildingSociety,
    #[strum(serialize = "Limited Liability Partnership")]
    LimitedLiabilityPartnership,
    #[strum(serialize = "Unincorporated association")]
    UnincorporatedAssociation,
    #[strum(serialize = "Friendly society")]
    FriendlySociety,
    #[strum(serialize = "Registered Party")]
    RegisteredParty,
}

/// Persistent exact-name funder identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunderId(Uuid);

impl FunderId {
    /// Wraps a database identity.
    #[must_use]
    pub const fn new(id: Uuid) -> Self {
        Self(id)
    }
    /// Database identity.
    #[must_use]
    pub const fn value(self) -> Uuid {
        self.0
    }
}

impl FromStr for FunderId {
    type Err = uuid::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}
