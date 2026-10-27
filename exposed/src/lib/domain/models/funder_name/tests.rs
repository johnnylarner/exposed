use super::{FunderNameFeatures, NameStatus, PeerageName};

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

#[test]
fn separates_ordered_titles_without_guessing_name_roles() {
    use super::PersonTitle;
    for (raw, titles, remainder) in [
        (
            "Mrs. Jane Smith",
            vec![PersonTitle::Mrs],
            Some("jane smith"),
        ),
        ("Mr John Smith", vec![PersonTitle::Mr], Some("john smith")),
        (
            "Lord (Michael) Farmer",
            vec![PersonTitle::Lord],
            Some("michael farmer"),
        ),
        (
            "  Lord ( Michael  John )  Farmer  ",
            vec![PersonTitle::Lord],
            Some("michael john farmer"),
        ),
        (
            "Dame Élodie O’Brien-Smith",
            vec![PersonTitle::Dame],
            Some("élodie o’brien-smith"),
        ),
        (
            "Rt. Hon. Dr. Dr Jane de la Cruz",
            vec![
                PersonTitle::Rt,
                PersonTitle::Hon,
                PersonTitle::Dr,
                PersonTitle::Dr,
            ],
            Some("jane de la cruz"),
        ),
        ("Jane Lord Smith", vec![], Some("jane lord smith")),
        ("Lord", vec![PersonTitle::Lord], None),
        ("Ｍｒ. James", vec![PersonTitle::Mr], Some("james")),
    ] {
        let parsed = features(raw);
        assert_eq!(parsed.name_raw.as_deref(), Some(raw));
        assert_eq!(parsed.person_titles, titles, "{raw}");
        assert_eq!(parsed.person_name_normalized.as_deref(), remainder, "{raw}");
    }
    for raw in [None, Some(""), Some("anonymous"), Some("withheld")] {
        let parsed = FunderNameFeatures::from_name(raw);
        assert!(parsed.person_titles.is_empty());
        assert!(parsed.person_name_normalized.is_none());
    }
    for title in [
        "mr",
        "mrs",
        "ms",
        "miss",
        "mx",
        "dr",
        "prof",
        "professor",
        "sir",
        "dame",
        "lord",
        "lady",
        "rev",
        "reverend",
        "rt",
        "hon",
    ] {
        let parsed = features(&format!("{title}. Smith"));
        assert_eq!(
            serde_json::to_value(&parsed.person_titles).unwrap(),
            serde_json::json!([title])
        );
        assert_eq!(parsed.person_core.as_deref(), Some("smith"));
    }
}

#[test]
fn unwraps_only_simple_post_title_parentheses_and_preserves_evidence() {
    let parsed = features("Lord (Michael) Farmer");
    assert_eq!(parsed.parenthetical_text, ["Michael"]);
    assert_eq!(
        parsed.name_normalized.as_deref(),
        Some("lord (michael) farmer")
    );
    assert_eq!(
        parsed.organisation_core.as_deref(),
        Some("lord michael farmer")
    );
    assert_eq!(parsed.person_core.as_deref(), Some("michael farmer"));
    assert_eq!(parsed.person_initials.as_deref(), Some("mf"));
    for raw in [
        "Lord (Michael)Farmer",
        "Lord ((Michael)) Farmer",
        "Lord (Michael Farmer",
        "Lord () Farmer",
        "Lord (Michael) Farmer)",
        "Lord (Michael 2) Farmer",
        "Lord (Michael and Jane) Farmer",
        "Lord (trading as Michael) Farmer",
        "Lord (trustee) Farmer",
        "Lord Smith (trustee)",
        "BJD (GB) Limited",
        "HSBC UK (Ian Stuart, CEO)",
    ] {
        let parsed = features(raw);
        let expected = raw.to_lowercase();
        let remainder = expected.strip_prefix("lord ").unwrap_or(&expected);
        assert_eq!(
            parsed.person_name_normalized.as_deref(),
            Some(remainder),
            "{raw}"
        );
        assert_eq!(parsed.name_normalized.as_deref(), Some(expected.as_str()));
    }
    for raw in ["(Mr) Smith", "Mr, Smith", "Mr-Mrs Smith"] {
        let parsed = features(raw);
        assert_eq!(parsed.person_core.as_deref(), Some("smith"));
        assert_eq!(parsed.person_initials.as_deref(), Some("s"));
    }
}

#[test]
fn peerage_bridges_require_complete_matching_personal_names() {
    for raw in [
        "Baron Vale of Mere (Alice Vale)",
        "Lord Vale of Mere (Alice Vale )",
        "BARON  Vale of Mere ( Alice Vale )",
        "Lord D’Arcy of Île (Élodie D’Arcy)",
        "Baron Smith-Jones of Mere (Anne Smith-Jones)",
    ] {
        let parsed = PeerageName::parse(raw).unwrap();
        assert!(parsed.bridge().is_some(), "{raw}");
    }
    for raw in [
        "Baron Vale of Mere (Alice Other)",
        "Baron Vale of Mere (Vale)",
        "Baron Vale of Mere (A Vale)",
        "Baron Vale of Mere (A. Vale)",
        "Baron Vale of Mere (Alice Vale 2)",
        "Baron Vale of Mere (Alice and Vale)",
        "Baron Vale of Mere (director Alice Vale)",
        "Baron Vale of Mere (representative Alice Vale)",
        "Baron Vale of Mere (on behalf of Alice Vale)",
        "Baron Vale of Mere (trading as Alice Vale)",
        "Baron Vale of Mere ((Alice Vale))",
        "Baron Vale of Mere (Alice Vale) (Alice Vale)",
        "Baron Vale of Mere (Alice Vale) annotation",
        "Baron Vale of Mere (Alice Vale",
        "Baron Vale of Mere Alice Vale)",
        "Baron Vale of Mere ()",
        "Baron Vale (Alice Vale)",
        "Lady Vale of Mere (Alice Vale)",
        "Dame Vale of Mere (Alice Vale)",
        "Baroness Vale of Mere (Alice Vale)",
        "Lady Vale of Mere",
        "Dame Alice Vale of Mere",
        "Vale of Mere",
        "Alice Vale of Mere",
    ] {
        assert!(PeerageName::parse(raw).is_none(), "{raw}");
    }
    assert_eq!(
        PeerageName::parse("Baron Vale of Mere (Alice Vale )")
            .unwrap()
            .spelling(),
        "vale of mere (alice vale)"
    );
}
