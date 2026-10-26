macro_rules! company_case {
    ($name:ident, $kind:expr, $number:expr, $expected:expr) => {
        #[kani::proof]
        fn $name() {
            let kind: Option<&str> = $kind;
            let number: Option<&str> = $number;
            let expected: Option<&str> = $expected;
            let mut row = super::observation("a");
            row.donor_kind = kind.map(str::to_owned);
            row.donor_company_number = number.map(str::to_owned);
            assert_eq!(
                company(&row).as_deref(),
                expected,
                "captured company metadata follows type and identifier precedence"
            );
        }
    };
}

company_case!(
    missing_kind_accepts_valid_identifier,
    None,
    Some("00000001"),
    Some("00000001")
);
company_case!(
    company_kind_accepts_valid_identifier,
    Some("company"),
    Some("00000001"),
    Some("00000001")
);
company_case!(
    person_kind_excludes_company_identifier,
    Some("person"),
    Some("00000001"),
    None
);
company_case!(
    organisation_kind_excludes_company_identifier,
    Some("organisation"),
    Some("00000001"),
    None
);
company_case!(
    unrecognized_kind_excludes_company_identifier,
    Some("mystery"),
    Some("00000001"),
    None
);
company_case!(
    company_identifier_requires_a_digit,
    None,
    Some("ABCDEFGH"),
    None
);
company_case!(
    company_identifier_rejects_short_value,
    None,
    Some("1234567"),
    None
);
company_case!(
    company_identifier_rejects_long_value,
    None,
    Some("123456789"),
    None
);
company_case!(
    company_identifier_rejects_punctuation,
    None,
    Some("00000!01"),
    None
);
company_case!(
    company_identifier_normalizes_case_and_space,
    Some(" COMPANY "),
    Some(" ab123456 \t"),
    Some("AB123456")
);

#[kani::proof]
fn every_ascii_suffix_obeys_company_format() {
    let suffix: u8 = kani::any();
    kani::assume(suffix < 128);
    let mut bytes = *b"00000000";
    bytes[7] = suffix;
    let mut row = super::observation("a");
    row.donor_company_number = Some(String::from_utf8(bytes.to_vec()).unwrap());
    let actual = company(&row);
    if suffix.is_ascii_alphanumeric() {
        bytes[7] = suffix.to_ascii_uppercase();
        assert_eq!(
            actual.as_deref(),
            Some(std::str::from_utf8(&bytes).unwrap()),
            "ASCII suffix preserves the normalized company identifier"
        );
    } else {
        assert!(
            actual.is_none(),
            "non-alphanumeric ASCII suffix cannot identify a company"
        );
    }
}
