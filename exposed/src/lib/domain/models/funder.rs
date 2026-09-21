//! Funders give financial compensation to [`parliament_members`]

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
    company_number: Option<usize>,
}

impl CompanyFunder {
    pub fn new(name: String, company_number: Option<usize>) -> Self {
        Self {
            name,
            company_number,
        }
    }
}
