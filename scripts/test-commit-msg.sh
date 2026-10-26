#!/bin/sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
test_root=$(mktemp -d "${TMPDIR:-/tmp}/exposed-commit-msg.XXXXXX")
trap 'rm -rf "$test_root"' 0
trap 'exit 1' HUP INT TERM

export GIT_CONFIG_NOSYSTEM=1
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_AUTHOR_NAME='Hook test'
export GIT_AUTHOR_EMAIL='hook-test@example.com'
export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE

repo="$test_root/repository with spaces"
hooks="$test_root/hooks with spaces"
mkdir -p "$repo" "$hooks"
cp "$project_root/.githooks/commit-msg" "$hooks/commit-msg"
git init -q "$repo"
cd "$repo"
git config core.hooksPath "$hooks"
git config commit.gpgsign false

message="$test_root/message with spaces"
log="$test_root/commit output"
passed=0

accept() {
    printf '%s\n' "$1" > "$message"
    if ! git -c core.commentChar='#' commit --quiet --allow-empty --cleanup=verbatim -F "$message" > "$log" 2>&1; then
        cat "$log" >&2
        printf 'FAIL: rejected valid message <%s>\n' "$1" >&2
        exit 1
    fi
    git log -1 --format=%B > "$test_root/actual message"
    printf '\n' >> "$message"
    if ! cmp -s "$message" "$test_root/actual message"; then
        printf 'FAIL: changed commit message <%s>\n' "$1" >&2
        exit 1
    fi
    passed=$((passed + 1))
}

reject() {
    before=$(git rev-parse HEAD)
    printf '%s\n' "$1" > "$message"
    if git commit --quiet --allow-empty --cleanup=verbatim -F "$message" > "$log" 2>&1; then
        printf 'FAIL: accepted invalid message <%s>\n' "$1" >&2
        exit 1
    fi
    test "$(git rev-parse HEAD)" = "$before"
    if ! grep -q 'Example: feat(search): add funder filtering' "$log"; then
        cat "$log" >&2
        printf 'FAIL: missing useful validation error\n' >&2
        exit 1
    fi
    passed=$((passed + 1))
}

for type in feat fix docs style refactor perf test build ci chore revert Custom; do
    accept "$type: describe the change"
done
accept 'feat(search): add funder filtering'
accept 'fix!: change response shape'
accept 'feat(api)!: change response shape'
accept 'Docs(search filters): explain usage'
accept 'fix: preserve a literal \ backslash'
accept 'feat: add filtering

A free-form body can contain anything.
This is not another conventional subject.

BREAKING CHANGE: response shape changed'

reject ''
reject 'add funder filtering'
reject ': missing type'
reject 'feat:missing space'
reject 'feat:'
reject 'feat: '
reject 'feat:   '
reject "feat: $(printf '\t')"
reject 'feat(): empty scope'
reject 'feat((api)): nested scope'
reject 'feat(api: unclosed scope'
reject 'feat(api)): extra parenthesis'
reject 'feat!!: extra marker'
reject 'feat2: numeric type'
reject ' feat: leading whitespace'
reject 'feat
(api): split header'
reject 'invalid subject

feat: valid body does not rescue subject'

printf 'Passed %s commit-msg integration cases.\n' "$passed"
