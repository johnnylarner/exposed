#[test]
fn every_parent_candidate_and_flag_combination() {
    for bits in 0..16_u8 {
        for flag in 0..3 {
            check_parent_eligibility_and_flag_gate(
                bits & 1 != 0,
                bits & 2 != 0,
                bits & 4 != 0,
                bits & 8 != 0,
                flag,
            );
        }
    }
}

#[test]
fn every_explicit_ultimate_and_flag_combination() {
    for usable in [false, true] {
        for flag in 0..3 {
            check_explicit_ultimate_precedence_and_withholding(usable, flag);
        }
    }
}

#[test]
fn every_missing_or_withheld_donor_combination() {
    for present in [false, true] {
        for usable in [false, true] {
            check_absent_and_withheld_local_donor_have_distinct_results(present, usable);
        }
    }
}

#[test]
fn every_parent_cycle_ambiguity_and_withholding_combination() {
    for bits in 0..8_u8 {
        check_parent_ambiguity_withholding_and_cycles_block_fallback(
            bits & 1 != 0,
            bits & 2 != 0,
            bits & 4 != 0,
        );
    }
}
