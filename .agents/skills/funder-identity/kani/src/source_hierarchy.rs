#[test]
fn nested_type_and_identifier_fields_remain_in_their_own_scope() {
    for flags in 0..16_u8 {
        for depth in 1..=2 {
            for donors in [false, true] {
                let mut child = vec![
                    json!({"name": if donors { "Name" } else { "DonorName" }, "value": "Child"}),
                    json!({"name": "Value", "value": "1"}),
                ];
                if flags & 4 != 0 {
                    child.push(json!({"name": "DonorStatus", "value": "Person"}));
                }
                if flags & 8 != 0 {
                    child.push(json!({"name": "DonorCompanyIdentifier", "value": "00000002"}));
                }
                let mut nested =
                    json!({"name": if donors { "Donors" } else { "Payments" }, "values": [child]});
                if depth == 2 {
                    nested = json!({"name": "Intermediate", "values": [[
                        {"name": "DonorStatus", "value": "Organisation"},
                        {"name": "DonorCompanyIdentifier", "value": "00000003"},
                        nested
                    ]]});
                }
                let mut fields = vec![json!({"name": "PaymentType", "value": "Monetary"}), nested];
                if flags & 1 != 0 {
                    fields.push(json!({"name": "DonorStatus", "value": "Company"}));
                }
                if flags & 2 != 0 {
                    fields.push(json!({"name": "DonorCompanyIdentifier", "value": "00000001"}));
                }
                let fields: Vec<SourceField> = serde_json::from_value(json!(fields)).unwrap();
                let mut entries = Vec::new();
                let mut pointers = Vec::new();
                funding_entries(&fields, "/versions/0/fields", &mut entries, &mut pointers)
                    .unwrap();
                assert_eq!(entries.len(), 2);
                assert_eq!(pointers.len(), 2);
                assert_ne!(pointers[0], pointers[1]);
                assert_eq!(
                    entries[0].funder_kind(),
                    (flags & 1 != 0).then_some("Company"),
                    "parent type stays local"
                );
                assert_eq!(
                    entries[0].company_number(),
                    (flags & 2 != 0).then_some("00000001"),
                    "parent identifier stays local"
                );
                assert_eq!(
                    entries[1].funder_kind(),
                    (flags & 4 != 0).then_some("Person"),
                    "nested type stays local and missing type is not inherited"
                );
                assert_eq!(
                    entries[1].company_number(),
                    (flags & 8 != 0).then_some("00000002"),
                    "nested identifier stays local and missing identifier is not inherited"
                );
            }
        }
    }
}
