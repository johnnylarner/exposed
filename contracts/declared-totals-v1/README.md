# Declared GBP totals contract v1

The backend owns both headlines. All categories participate; dates are registration
timestamps. The current term starts inclusively at 2024-07-09 00:00:00 UTC.
Undated declarations belong only to historical totals. Known GBP amounts add
exactly, including duplicates, negatives and zero. Missing GBP amounts and unknown
currencies mark coverage incomplete. Foreign currency amounts do not convert into
GBP or make GBP coverage incomplete. A missing sum remains null, not zero.

`fixture.sql` and `expected.json` are a small frozen baseline. The first member's
historical total is 100 + 200 + 30.25 - 5 + .1 + .1 + 7.55 = 333.
Excluding the pre-term 100 and undated 7.55 gives 225.45 for the current term.
Other members distinguish exact cancellation, empty history, foreign-only,
missing-only, a declaration without entries, and incomplete historical coverage
that must not taint a complete current-term zero.

With frontend dependencies and Playwright Chromium installed, run from the root:

```sh
DATABASE_URL=postgres://localhost/exposed_test scripts/check-declared-totals-contract.sh
```

The supplied database must already have the application schema for SQLx compile
checks, and its role must create databases. SQLx applies migrations and the fixture
in isolated test databases. The script does not import or update source records.
The Rust test asserts actual Axum responses against the baseline, then exports
those bytes into a temporary directory. Browser tests feed those responses through
the existing HTTP stub into the built Svelte application. Contract mode fails when
the fresh response files are absent. Ordinary browser tests do not select it.

CI runs on every PR and takes both baseline files from the PR target commit before
compilation. The first PR bootstraps only when that target has neither baseline
nor guard workflow/script; it logs that exception. Partial or subsequently missing
baselines fail. Requested semantic migrations need an explicit new version and
reviewed retirement of the previous contract. Editing expectations to match a
regression is not a fix. The repository check cannot stop coordinated edits to the
check itself; required checks and review rules must be configured on GitHub.
