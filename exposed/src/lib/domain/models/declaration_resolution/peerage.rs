use super::{BTreeMap, BTreeSet, FunderObservationId, Observation, identity::individual_donor};
use crate::domain::models::funder_name::PeerageName;

#[derive(Default)]
pub(super) struct PeerageAliases {
    personal_names: BTreeMap<FunderObservationId, String>,
}

impl PeerageAliases {
    pub(super) fn from_observations<'a>(
        observations: impl Iterator<Item = &'a Observation>,
    ) -> Self {
        let parsed = observations
            .filter(|observation| individual_donor(observation))
            .filter_map(|observation| {
                PeerageName::parse(observation.name_raw.as_deref()?)
                    .map(|name| (observation.funder_id.clone(), name))
            })
            .collect::<Vec<_>>();
        let mut styles = BTreeMap::<&str, BTreeSet<&str>>::new();
        let mut aliases = BTreeMap::<&str, BTreeSet<&str>>::new();
        for (_, name) in &parsed {
            if let Some(bridge) = name.bridge() {
                styles
                    .entry(bridge.style())
                    .or_default()
                    .insert(bridge.personal_name());
                for alias in bridge
                    .styles()
                    .into_iter()
                    .chain([name.spelling(), bridge.personal_name()])
                {
                    aliases
                        .entry(alias)
                        .or_default()
                        .insert(bridge.personal_name());
                }
            }
        }
        let mut quarantined = BTreeSet::new();
        for (_, name) in &parsed {
            if let Some(bridge) = name.bridge()
                && styles[bridge.style()].len() != 1
            {
                quarantined.extend(bridge.styles());
                quarantined.insert(name.spelling());
            }
        }
        let personal_names = parsed
            .iter()
            .filter_map(|(id, name)| {
                if quarantined.contains(name.spelling()) {
                    return None;
                }
                let targets = aliases.get(name.spelling())?;
                (targets.len() == 1).then(|| (id.clone(), (*targets.first().unwrap()).to_owned()))
            })
            .collect();
        Self { personal_names }
    }

    pub(super) fn personal_name(&self, id: &FunderObservationId) -> Option<&str> {
        self.personal_names.get(id).map(String::as_str)
    }

    pub(super) fn agree(&self, left: &FunderObservationId, right: &FunderObservationId) -> bool {
        self.personal_name(left)
            .zip(self.personal_name(right))
            .is_some_and(|(left, right)| left == right)
    }
}
