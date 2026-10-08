# Donor-name policy verification

Implemented approved candidate A in the dedicated donor-name worktree on branch implement/donor-name-resolution-20261008. No scoring, attribution, output column, or database schema changes.

Model the Domain shaped the implementation. CandidateEdge carries typed eligibility once, so company ambiguity and component merging use the same classification. Component provenance derives from accepted statistical edges plus seeded company groups rather than mutable flags.

Throughput checkpoint. The bounded candidate map is already produced by profile expansion. Ambiguity and statistical support use temporary component partitions over that map. No extra worker requests or synthetic pairs are introduced.

Verification environment uses isolated PostgreSQL18 at port 55488, target /private/tmp/exposed-resolution-verification-target, and the existing pinned Splink interpreter in the prior worktree.

- Initial library run exposed five old tests intentionally superseded by donor-name acceptance. Updated expectations or used payer roles where the old statistical rule still applies.
- Rust library suite passed 59 tests before final supplementary role-threshold and seeded-company tests.
- Real Splink worker suite passed all four tests.
- Real clean-to-resolve CLI test passed. Four retained occurrences produce one statistical charity identity and one addressless Gary Lubner donor-name identity. Donor score remains real, positive, and below 0.999. Cleaned funders and funding_entries files remain byte-for-byte identical. Attribution remains donor for all four payments. Existing results cannot be overwritten.
- Final formatting, library, and Clippy outcomes recorded below when complete.

Independent review is owned by the parent agent. Architecture exploration was completed before implementation and supplied as the accepted design. No subagents were spawned by this code owner.

Final outcomes.

- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo test --locked --package exposed --lib` passed all 61 tests.
- `cargo test --locked --package exposed --test declaration_resolution_cli -- --ignored` passed its real Splink CLI test.
- `python -m unittest discover -s resolution/tests -v` passed all four real worker tests.
- Strict Clippy is blocked by the existing `run_cli` length lint under the crate's deny attributes. `-A clippy::too_many_lines` cannot override those attributes. `cargo clippy --locked --package exposed --lib --tests -- -D warnings --force-warn clippy::too_many_lines` passed with only function-length warnings for `run_cli`, the expanded resolution CLI fixture, and the existing cleaning CLI fixture. No application lint suppression or unrelated CLI refactor was added.
- Diff review removed no further code. There are no synthetic scores, secondary eligibility implementations, new public services, or mutable provenance flags.

No policy or architecture deviations from candidate A. The existing CLI integration test was expanded to contain both real-score paths in one clean-to-resolve invocation rather than adding a duplicate fixture setup.
