use super::{FunderNameFeatures, NameStatus};

fn features(name: &str) -> FunderNameFeatures {
    FunderNameFeatures::from_name(Some(name))
}

#[test]
fn keeps_raw_names_and_parallel_features_without_erasing_geography() {
    let name = features("  Ｍｒ.  James O’Brien-Smith  ");
    assert_eq!(
        name.name_raw.as_deref(),
        Some("  Ｍｒ.  James O’Brien-Smith  ")
    );
    assert_eq!(
        name.name_normalized.as_deref(),
        Some("mr. james o’brien-smith")
    );
    assert_eq!(name.name_tokens.as_deref(), Some("mr james obrien smith"));
    assert_eq!(name.person_core.as_deref(), Some("james obrien smith"));
    assert_eq!(name.person_initials.as_deref(), Some("jos"));
    assert_eq!(
        features("Friends of Sinn Féin Canada")
            .name_accent_folded
            .as_deref(),
        Some("friends of sinn fein canada")
    );
    assert_eq!(
        features("BJD (GB) Limited").organisation_core.as_deref(),
        Some("bjd gb")
    );
    assert_eq!(
        features("BJD (GB) Limited").name_normalized.as_deref(),
        Some("bjd (gb) limited")
    );
    assert_eq!(
        features("U.K. Example LTD.").name_tokens.as_deref(),
        Some("uk example ltd")
    );
    assert_eq!(
        features("Example (U.K.), Ltd.").name_tokens.as_deref(),
        Some("example uk ltd")
    );
    assert_eq!(
        features("Communication Workers’ Union")
            .name_tokens
            .as_deref(),
        Some("communication workers union")
    );
}

#[test]
fn extracts_only_explicit_aliases_and_keeps_primary_and_full_names() {
    let name = features("CSC Computer Sciences Limited (Trading as DXC Technology Ltd) (UK)");
    assert_eq!(name.explicit_aliases, ["DXC Technology Ltd"]);
    assert_eq!(name.alias_normalized, ["dxc technology ltd"]);
    assert_eq!(
        name.primary_name_normalized.as_deref(),
        Some("csc computer sciences limited (uk)")
    );
    assert_eq!(
        name.parenthetical_text,
        ["Trading as DXC Technology Ltd", "UK"]
    );
    assert!(
        name.name_tokens
            .unwrap()
            .contains("trading as dxc technology")
    );
    assert_eq!(
        features("CSC Computer Sciences Limited (Trading as DXC Technology Ltd)")
            .organisation_core
            .as_deref(),
        Some("csc computer sciences")
    );
    for (raw, expected) in [
        (
            "British Academy of Songwriters, Composers and Authors, trading as The Ivors Academy",
            "The Ivors Academy",
        ),
        (
            "Fletchers Solicitors Limited t/a Fletchers Solicitors",
            "Fletchers Solicitors",
        ),
        ("Pierce Protocols Limited trading as Heni", "Heni"),
    ] {
        assert_eq!(features(raw).explicit_aliases, [expected]);
    }
    assert!(
        features("MPM Connect Ltd (a company controlled by Peter Hearn)")
            .explicit_aliases
            .is_empty()
    );
    assert!(
        features("Example (sponsored by Other)")
            .explicit_aliases
            .is_empty()
    );
}

#[test]
fn validates_acronyms_and_records_literal_ambiguity_evidence() {
    let aslef = features("ASLEF (Associated Society of Locomotive Engineers and Firemen)");
    assert_eq!(aslef.explicit_acronym.as_deref(), Some("aslef"));
    assert_eq!(
        aslef.acronym_expansion_normalized.as_deref(),
        Some("associated society of locomotive engineers and firemen")
    );
    assert_eq!(
        features("Associated Society of Locomotive Engineers and Firemen (ASLEF)")
            .explicit_acronym
            .as_deref(),
        Some("aslef")
    );
    assert_eq!(features("BJD (GB) Limited").explicit_acronym, None);
    assert!(features("Alan Milburn & Ruth Briel").has_conjunction);
    assert!(features("Unite the Union (UK").has_unbalanced_parentheses);
    assert!(features("Unite) the Union").has_unbalanced_parentheses);
    assert!(!features("Unite (the Union)").has_unbalanced_parentheses);
    assert_eq!(
        features("Sir Trevor Chinn").person_core.as_deref(),
        Some("trevor chinn")
    );
    assert_eq!(
        features("Lord Michael Farmer").person_core.as_deref(),
        Some("michael farmer")
    );
    assert_eq!(
        features("Example Limited Edition")
            .organisation_core
            .as_deref(),
        Some("example limited edition")
    );
}

#[test]
fn withholds_matching_features_for_missing_and_confidential_names() {
    for raw in [None, Some(""), Some(" \t ")] {
        let name = FunderNameFeatures::from_name(raw);
        assert_eq!(name.name_status, NameStatus::Missing);
        assert!(name.name_normalized.is_none());
    }
    let name = features(" Confidential ");
    assert_eq!(name.name_status, NameStatus::Withheld);
    assert_eq!(name.name_raw.as_deref(), Some(" Confidential "));
    assert!(name.name_token_key.is_none());
    assert_eq!(
        features("Confidential Company Ltd").name_status,
        NameStatus::Present
    );
    assert_eq!(
        features("National  Liberal Club")
            .name_normalized
            .as_deref(),
        Some("national liberal club")
    );
    assert_eq!(
        features("Labour Together Limited").organisation_core,
        features("LABOUR TOGETHER LTD.").organisation_core
    );
    assert_eq!(
        features("The Club Club").name_token_key.as_deref(),
        Some("club club the")
    );
}
