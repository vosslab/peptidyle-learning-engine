#!/usr/bin/env bash
# Exercise the sourceable local-login upgrade boundary without Podman or env.local.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

TEST_DIRECTORY="$(mktemp -d "${TMPDIR:-/tmp}/ple-local-identity-bootstrap.XXXXXX")"
trap 'rm -rf "$TEST_DIRECTORY"' EXIT
REPO_ROOT="$PWD"
LOCAL_ENV_FILE="containers/env.local"
LOCAL_IDENTITY_FILE="$TEST_DIRECTORY/local-identities.json"
LOCAL_CREDENTIAL_FILE="$TEST_DIRECTORY/local-login.txt"
LOCAL_TENANT_ID="00000000-0000-0000-0000-000000000100"
LOCAL_INSTRUCTOR_ID="00000000-0000-0000-0000-000000000101"
LOCAL_STUDENT_ID="00000000-0000-0000-0000-000000000102"
INSTRUCTOR_CREDENTIAL="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
STUDENT_CREDENTIAL="AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE"

die() {
	exit 1
}

local_credential_record() {
	die
}

source containers/local_identity_bootstrap.sh

[ "$(normalize_default_local_env_file "containers/env.local")" = "containers/env.local" ] || die
[ "$(normalize_default_local_env_file "$PWD/containers/env.local")" = "containers/env.local" ] || die
custom_env_file="$TEST_DIRECTORY/env.local"
[ "$(normalize_default_local_env_file "$custom_env_file")" = "$custom_env_file" ] || die

file_mode() {
	if stat -f '%Lp' "$1" >/dev/null 2>&1; then
		stat -f '%Lp' "$1"
	else
		stat -c '%a' "$1"
	fi
}

file_inode() {
	if stat -f '%i' "$1" >/dev/null 2>&1; then
		stat -f '%i' "$1"
	else
		stat -c '%i' "$1"
	fi
}

write_credentials() {
	credential_path="$1"
	printf 'instructor=%s\nstudent=%s\n' "$INSTRUCTOR_CREDENTIAL" "$STUDENT_CREDENTIAL" >"$credential_path"
}

write_sentinel_projection() {
	printf '%s\n' '{"credentials":[{"legacy":"aliasless"}]}' >"$LOCAL_IDENTITY_FILE"
	chmod 644 "$LOCAL_IDENTITY_FILE"
}

assert_rejected_without_replacement() {
	credential_path="$1"
	write_sentinel_projection
	projection_checksum="$(shasum -a 256 "$LOCAL_IDENTITY_FILE")"
	if (LOCAL_CREDENTIAL_FILE="$credential_path"; bootstrap_local_identities) >/dev/null 2>&1; then
		die
	fi
	[ "$projection_checksum" = "$(shasum -a 256 "$LOCAL_IDENTITY_FILE")" ] || die
}

write_credentials "$LOCAL_CREDENTIAL_FILE"
chmod 600 "$LOCAL_CREDENTIAL_FILE"
write_sentinel_projection
credential_checksum="$(shasum -a 256 "$LOCAL_CREDENTIAL_FILE")"
projection_inode="$(file_inode "$LOCAL_IDENTITY_FILE")"
success_stdout="$TEST_DIRECTORY/bootstrap.stdout"
success_stderr="$TEST_DIRECTORY/bootstrap.stderr"
bootstrap_local_identities >"$success_stdout" 2>"$success_stderr"

[ "$credential_checksum" = "$(shasum -a 256 "$LOCAL_CREDENTIAL_FILE")" ] || die
[ "$(file_mode "$LOCAL_CREDENTIAL_FILE")" = "600" ] || die
[ "$(file_mode "$LOCAL_IDENTITY_FILE")" = "644" ] || die
[ "$projection_inode" != "$(file_inode "$LOCAL_IDENTITY_FILE")" ] || die
[ ! -s "$success_stdout" ] && [ ! -s "$success_stderr" ] || die
[ ! -L "$LOCAL_IDENTITY_FILE" ] || die
[ "$(rg -o '"learner_alias"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "3" ] || die
[ "$(rg -o '"learner_alias":"instructor-local"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"learner_alias":"student-local"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"learner_alias":"student-jack"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"tenant_id":"00000000-0000-0000-0000-000000000100"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "3" ] || die
[ "$(rg -o '"user_id":"00000000-0000-0000-0000-000000000101"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"user_id":"00000000-0000-0000-0000-000000000102"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"user_id":"00000000-0000-0000-0000-000000000103"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"display_name":"Dr. Fake Professor"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"display_name":"Mary Fake Student"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"display_name":"Jack Fake Student"' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"roles":\["instructor","administrator"\]' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "1" ] || die
[ "$(rg -o '"roles":\["student"\]' "$LOCAL_IDENTITY_FILE" | wc -l | tr -d '[:space:]')" = "2" ] || die
if rg -F "$INSTRUCTOR_CREDENTIAL" "$LOCAL_IDENTITY_FILE" >/dev/null \
	|| rg -F "$STUDENT_CREDENTIAL" "$LOCAL_IDENTITY_FILE" >/dev/null; then
	die
fi
instructor_hash="$(printf '%s=' "$INSTRUCTOR_CREDENTIAL" | tr '_-' '/+' | openssl base64 -d -A | openssl dgst -sha256 -r | awk '{print $1}')"
student_hash="$(printf '%s=' "$STUDENT_CREDENTIAL" | tr '_-' '/+' | openssl base64 -d -A | openssl dgst -sha256 -r | awk '{print $1}')"
rg -F "\"credential_sha256\":\"$instructor_hash\"" "$LOCAL_IDENTITY_FILE" >/dev/null || die
rg -F "\"credential_sha256\":\"$student_hash\"" "$LOCAL_IDENTITY_FILE" >/dev/null || die

malformed_credentials="$TEST_DIRECTORY/malformed-login.txt"
printf 'instructor=not-canonical\nstudent=%s\n' "$STUDENT_CREDENTIAL" >"$malformed_credentials"
chmod 600 "$malformed_credentials"
assert_rejected_without_replacement "$malformed_credentials"

wrong_mode_credentials="$TEST_DIRECTORY/wrong-mode-login.txt"
write_credentials "$wrong_mode_credentials"
chmod 644 "$wrong_mode_credentials"
assert_rejected_without_replacement "$wrong_mode_credentials"

duplicate_credentials="$TEST_DIRECTORY/duplicate-login.txt"
printf 'instructor=%s\ninstructor=%s\nstudent=%s\n' \
	"$INSTRUCTOR_CREDENTIAL" "$INSTRUCTOR_CREDENTIAL" "$STUDENT_CREDENTIAL" >"$duplicate_credentials"
chmod 600 "$duplicate_credentials"
assert_rejected_without_replacement "$duplicate_credentials"

symlink_credentials="$TEST_DIRECTORY/symlink-login.txt"
ln -s "$LOCAL_CREDENTIAL_FILE" "$symlink_credentials"
assert_rejected_without_replacement "$symlink_credentials"

assert_rejected_without_replacement "$TEST_DIRECTORY/missing-login.txt"

printf '%s\n' 'local identity bootstrap regression: PASS'
