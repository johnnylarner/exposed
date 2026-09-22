//! Funders give financial compensation to [`parliament_members`]

use strum::EnumString;

#[derive(Clone, Debug)]
/// Funder kinds
pub enum Funder {
    /// Legal person funder
    Individual(IndividualFunder),
    /// Legal entity funder
    Company(CompanyFunder),
}

impl Funder {
    /// Creates a new funder
    pub fn name(&self) -> &str {
        match self {
            Self::Company(c) => &c.name,
            Self::Individual(i) => &i.name,
        }
    }
}

impl From<IndividualFunder> for Funder {
    fn from(value: IndividualFunder) -> Self {
        Self::Individual(value)
    }
}

impl From<CompanyFunder> for Funder {
    fn from(value: CompanyFunder) -> Self {
        Self::Company(value)
    }
}

#[derive(Clone, Debug)]
/// Legal person funder
pub struct IndividualFunder {
    name: String,
}

impl IndividualFunder {
    /// Creates a legal person funder
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Clone, Debug)]
/// Legal entity funder
pub struct CompanyFunder {
    name: String,
    company_number: Option<String>,
}

impl CompanyFunder {
    /// Creates a legal entity funder
    pub fn new(name: String, company_number: Option<String>) -> Self {
        Self {
            name,
            company_number,
        }
    }
}

#[derive(Clone, Debug, EnumString)]
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
    #[strum(disabled)]
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
