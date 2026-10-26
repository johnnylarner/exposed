#[test]
fn all_small_evidence_graphs_preserve_company_and_type_boundaries() {
    let mut checked = 0;
    for count in 1..=4_u32 {
        for assignment in 0..5_u32.pow(count) {
            let mut labels = Vec::new();
            let mut remaining = assignment;
            for _ in 0..count {
                labels.push(remaining % 5);
                remaining /= 5;
            }
            for mask in 0..(1 << (count * (count - 1) / 2)) {
                let mut observations = Vec::new();
                for (index, label) in labels.iter().enumerate() {
                    let mut row = super::observation(["a", "b", "c", "d"][index]);
                    match label {
                        0 => {}
                        1 | 2 => {
                            row.donor_kind = Some("company".into());
                            row.donor_company_number =
                                Some(if *label == 1 { "00000001" } else { "00000002" }.into());
                        }
                        3 => row.donor_kind = Some("person".into()),
                        4 => row.donor_kind = Some("organisation".into()),
                        _ => unreachable!(),
                    }
                    observations.push(row);
                }
                let outcomes = run_graph(observations, mask);
                for (left, label) in labels.iter().enumerate() {
                    if *label == 1 || *label == 2 {
                        let expected = if *label == 1 {
                            "source-reported:companies-house:00000001"
                        } else {
                            "source-reported:companies-house:00000002"
                        };
                        assert_eq!(
                            outcomes[left].identity_id.as_deref(),
                            Some(expected),
                            "company identifier changed; labels={labels:?}, edges={mask}"
                        );
                        assert!(
                            matches!(
                                outcomes[left].identity_basis,
                                IdentityBasis::SourceReportedCompany
                            ),
                            "captured source basis changed; labels={labels:?}, edges={mask}"
                        );
                    }
                    for right in left + 1..labels.len() {
                        let other = labels[right];
                        let company_conflict = matches!((*label, other), (1, 2) | (2, 1));
                        let kind_conflict = (*label == 3 && matches!(other, 1 | 2 | 4))
                            || (other == 3 && matches!(*label, 1 | 2 | 4));
                        if company_conflict || kind_conflict {
                            assert_ne!(
                                outcomes[left].identity_id, outcomes[right].identity_id,
                                "incompatible identities merged; labels={labels:?}, edges={mask}"
                            );
                        }
                    }
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 41_055);
    println!("Checked {checked} complete input graphs with one to four observations.");
}

#[test]
fn every_four_node_bridge_abstains_when_both_companies_are_reachable() {
    for mask in 0..64_u8 {
        let mut first = super::observation("a");
        first.donor_company_number = Some("00000001".into());
        let mut last = super::observation("d");
        last.donor_company_number = Some("00000002".into());
        let outcomes = run_graph(
            vec![
                first,
                super::observation("b"),
                super::observation("c"),
                last,
            ],
            mask,
        );
        let bridge = mask & 8 != 0;
        for (index, to_first, to_last, other_first, other_last) in
            [(1, 1, 16, 2, 32), (2, 2, 32, 1, 16)]
        {
            let reaches_first = mask & to_first != 0 || (bridge && mask & other_first != 0);
            let reaches_last = mask & to_last != 0 || (bridge && mask & other_last != 0);
            if reaches_first && reaches_last {
                assert!(
                    outcomes[index]
                        .identity_id
                        .as_ref()
                        .unwrap()
                        .starts_with("run-local:"),
                    "competing company attachments must abstain; edges={mask}, observation={index}"
                );
            }
        }
    }
}

#[test]
fn company_identity_precedes_missing_or_present_names() {
    for named in [false, true] {
        let mut row = super::observation("a");
        row.name_normalized = named.then(|| "a".into());
        row.donor_company_number = Some("00000001".into());
        let outcomes = run_graph(vec![row], 0);
        assert_eq!(
            outcomes[0].identity_id.as_deref(),
            Some("source-reported:companies-house:00000001"),
            "accepted company identifier precedes name fallback"
        );
    }
}

#[test]
fn explicit_noncompany_metadata_cannot_anchor() {
    for kind in ["person", "organisation", "charity", "mystery", ""] {
        let mut row = super::observation("a");
        row.donor_kind = Some(kind.into());
        row.donor_company_number = Some("00000001".into());
        assert_eq!(
            company(&row),
            None,
            "explicit non-company type blocks a company identifier"
        );
    }
}
