fn input(observations: Vec<Observation>) -> ResolutionInput {
    let mut input = ResolutionInput {
        observations: BTreeMap::new(),
        payments: BTreeMap::new(),
        digests: BTreeMap::new(),
    };
    for row in observations {
        input.observations.insert(row.funder_id.clone(), row);
    }
    input
}

fn run_graph(observations: Vec<Observation>, mask: u8) -> Vec<ObservationResolution> {
    let input = input(observations);
    let ids: Vec<_> = input.observations.keys().cloned().collect();
    let mut scoring = ScoringInput {
        candidate_budget: 6,
        rows: Vec::new(),
        members: BTreeMap::new(),
    };
    let mut pairs = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        scoring.members.insert(i.to_string(), vec![id.clone()]);
    }
    let mut bit = 0;
    for left in 0..ids.len() {
        for right in left + 1..ids.len() {
            if mask & (1 << bit) != 0 {
                pairs.push(ScoredPair {
                    left: left.to_string(),
                    right: right.to_string(),
                    probability: 1.0,
                    name_level: 2,
                    address_level: -1,
                });
            }
            bit += 1;
        }
    }
    resolve(&input, &scoring, pairs, "proof").unwrap().0
}
