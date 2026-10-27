#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

#[derive(Debug)]
pub enum ParliamentApiError {}
#[derive(Debug)]
pub enum EntitySearchPipelineError {}

mod domain {
    pub(crate) mod models {
        pub(crate) mod funder_name {
            include!(concat!(env!("OUT_DIR"), "/funder_name.rs"));
        }
    }
}
mod peerage {
    include!(concat!(
        env!("EXPOSED_SOURCE_ROOT"),
        "/exposed/src/lib/domain/models/declaration_resolution/peerage.rs"
    ));
}
use peerage::PeerageAliases;

include!(concat!(env!("OUT_DIR"), "/domain.rs"));

#[cfg(test)]
mod source_hierarchy {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};
    type ApiError = serde_json::Error;
    include!(concat!(env!("OUT_DIR"), "/source_projection.rs"));
    include!("source_hierarchy.rs");
}

mod identity {
    include!(concat!(
        env!("EXPOSED_SOURCE_ROOT"),
        "/exposed/src/lib/domain/models/declaration_resolution/identity.rs"
    ));
    #[cfg(any(kani, test))]
    include!("identity_cases.rs");
    #[cfg(kani)]
    include!("identity_proofs.rs");
    #[cfg(test)]
    include!("identity_exhaustive.rs");
}

mod attribution {
    include!(concat!(
        env!("EXPOSED_SOURCE_ROOT"),
        "/exposed/src/lib/domain/models/declaration_resolution/attribution.rs"
    ));
    #[cfg(test)]
    include!("attribution_cases.rs");
    #[cfg(test)]
    include!("attribution_exhaustive.rs");
}

#[cfg(any(kani, test))]
fn observation(id: &str) -> Observation {
    Observation {
        funder_id: FunderObservationId(id.into()),
        member_id: MemberId(1),
        declaration_id: DeclarationId(NonZeroU32::new(1).unwrap()),
        register_id: 1,
        role: FunderRole::Donor,
        source_scope: "declaration".into(),
        source_pointer: "/versions/0/fields".into(),
        funding_entry_id: None,
        funding_ordinal: None,
        donor_kind: None,
        donor_company_number: None,
        name_raw: Some("A".into()),
        name_normalized: Some("a".into()),
        person_name_normalized: None,
        organisation_core: None,
        alias_normalized: Vec::new(),
        address_normalized: None,
        address_match_quality: AddressMatchQuality::Unavailable,
    }
}
