//! Funders give financial compensation to [`parliament_members`]

use strum::{AsRefStr, EnumString};

#[derive(Clone, Debug, PartialEq, Eq)]
/// Funder of MP declarations
pub struct Funder {
    name: String,
    funder_kind: FunderKind,
}

impl Funder {
    #[must_use]
    /// Creates a new instance
    pub const fn new(name: String, funder_kind: FunderKind) -> Self {
        Self { name, funder_kind }
    }

    #[must_use]
    /// Gets funder name
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    /// Gets legal person
    pub fn kind(&self) -> &str {
        self.funder_kind.as_ref()
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
