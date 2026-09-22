//! Funders give financial compensation to [`parliament_members`]

use strum::EnumString;

#[derive(Clone, Debug)]
pub enum Funder {
    Individual(IndividualFunder),
    Company(CompanyFunder),
}

impl Funder {
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
pub struct IndividualFunder {
    name: String,
}

impl IndividualFunder {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Clone, Debug)]
pub struct CompanyFunder {
    name: String,
    company_number: Option<String>,
}

impl CompanyFunder {
    pub fn new(name: String, company_number: Option<String>) -> Self {
        Self {
            name,
            company_number,
        }
    }
}

#[derive(Clone, Debug, EnumString)]
pub enum FunderKind {
    Company,
    Individual,
    Trust,
    #[strum(serialize = "Trade Union")]
    TradeUnion,
    Other,
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
