#!/usr/bin/env bash
# Full PLE acceptance for the private, source-pinned WebWork renderer.
#
# This deliberately exercises only PLE's public gateway.  It never reads a
# renderer-internal token, request field, answer hash, source text, or upstream
# response. The controller owns renderer lifecycle readiness.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

WORK_DIRECTORY="$(mktemp -d "${TMPDIR:-/tmp}/ple-webwork-e2e.XXXXXX")"
chmod 700 "$WORK_DIRECTORY"
STACK_DIRECTORY="$WORK_DIRECTORY/stack"
mkdir -m 700 "$STACK_DIRECTORY"
ENV_FILE=""
STACK_MANIFEST_FILE=""
CREDENTIAL_FILE=""
PROJECT_NAME=""
STACK_STARTED=0
FIXTURE_MANIFEST_FILE="$WORK_DIRECTORY/webwork-renderer-fixture.json"
COOKIE_JAR="$WORK_DIRECTORY/student.cookies"

fail() {
	echo "FAIL: $*" >&2
	exit 1
}

require_file() {
	[ -r "$1" ] || fail "required local artifact is missing or unreadable: $1"
}

adapter() {
	local action="$1"
	shift
	python3 -m local_stack_control._consumer_cli "$action" --manifest "$STACK_MANIFEST_FILE" "$@"
}

cleanup_stack() {
	local status="${1:-$?}"
	local cleanup_failed=0
	if [ "${PLE_E2E_KEEP:-0}" = "1" ]; then
		echo "WebWork browser E2E: preserving disposable project $PROJECT_NAME ($STACK_DIRECTORY)"
	elif [ "$STACK_STARTED" = "1" ]; then
		adapter cleanup || cleanup_failed=1
	fi
	if [ "$cleanup_failed" = "1" ]; then
		echo "WebWork browser E2E: exact disposable cleanup failed; preserving $STACK_DIRECTORY" >&2
		[ "$status" -ne 0 ] || status=1
	elif [ "${PLE_E2E_KEEP:-0}" != "1" ]; then
		rm -rf -- "$WORK_DIRECTORY"
	fi
	exit "$status"
}

trap cleanup_stack EXIT

env_value() {
	awk -F= -v setting_name="$1" '$1 == setting_name { value = substr($0, index($0, "=") + 1) } END { print value }' "$ENV_FILE"
}

json_value() {
	python3 - "$1" "$2" <<'PY'
import json
import sys

value = json.load(open(sys.argv[1], encoding="utf-8"))
for part in sys.argv[2].split("."):
    value = value[part]
if not isinstance(value, str):
    raise SystemExit("requested JSON value is not a string")
print(value)
PY
}

assert_no_private_material() {
	python3 - "$@" <<'PY'
import pathlib
import sys

# These are upstream-private names/material.  A response or local diagnostic
# containing one is a failure; the E2E never needs to decode any of them.
forbidden = ("problemSource", "passwd", "AnSwEr", "hidden_input_field", "render_rpc", "render-api")
for name in sys.argv[1:]:
    data = pathlib.Path(name).read_bytes()
    for term in forbidden:
        if term.encode() in data:
            raise SystemExit(f"private renderer material leaked into {name}: {term}")
PY
}

gateway_request() {
	method="$1"
	path="$2"
	output="$3"
	shift 3
	status="$(curl --silent --show-error --output "$output" --write-out '%{http_code}' \
		--max-time 30 --request "$method" --cookie "$COOKIE_JAR" --cookie-jar "$COOKIE_JAR" \
		"$@" "${BASE_URL}${path}")"
	printf '%s\n' "$status"
}

create_attempt() {
	local run_file="$1"
	local attempts_file="$2"
	local run_status attempts_status
	run_status="$(gateway_request POST /api/runs "$run_file" --header 'content-type: application/json' --data "{\"assignmentId\":\"${ASSIGNMENT_ID}\"}")"
	[ "$run_status" = "201" ] || fail "starting a PLE WebWork run returned HTTP $run_status"
	run_id="$(json_value "$run_file" id)"
	attempts_status="$(gateway_request GET "/api/runs/${run_id}/attempts" "$attempts_file")"
	[ "$attempts_status" = "200" ] || fail "listing PLE WebWork attempts returned HTTP $attempts_status"
	python3 - "$attempts_file" <<'PY'
import json
import sys
items = json.load(open(sys.argv[1], encoding="utf-8")).get("items", [])
if len(items) != 1 or not isinstance(items[0].get("id"), str):
    raise SystemExit("expected exactly one WebWork attempt")
print(items[0]["id"])
PY
}

choose_visible_answer() {
	# Select a rendered PLE choice by learner-visible wording only.  The script
	# does not inspect upstream IDs, field names, answer hashes, or source.
	python3 - "$1" "$2" <<'PY'
import json
import sys

document = json.load(open(sys.argv[1], encoding="utf-8"))
classification = sys.argv[2]
names = {
    "hydrophobic": {
        "benzene", "toluene", "ethylene", "propane", "butane", "cyclohexane", "hexane", "octane",
    },
    "hydrophilic": {
        "acetate", "water", "erythrose", "glucose", "sucrose", "glycerol", "glycine", "ethanol",
        "methanol", "ammonia", "sodium chloride", "phosphoric acid", "urea",
    },
}
if classification not in names:
    raise SystemExit(f"unknown visible-choice classification: {classification}")
wire = json.dumps(document, separators=(",", ":"))
for forbidden in ("problemSource", "passwd", "AnSwEr", "hidden_input_field", "render_rpc", "render-api"):
    if forbidden in wire:
        raise SystemExit(f"private renderer material leaked into PLE question: {forbidden}")
choices = document.get("response", {}).get("choices", [])
if not isinstance(choices, list):
    raise SystemExit("PLE question did not expose a multiple-choice projection")
matches = []
for choice in choices:
    if not isinstance(choice, dict):
        continue
    blocks = choice.get("body", [])
    if not isinstance(blocks, list):
        blocks = []
    visible = " ".join(
        [str(choice.get(key, "")) for key in ("label", "text", "content")]
        + [str(block.get("markdown", "")) for block in blocks if isinstance(block, dict)]
    )
    normalized = visible.casefold()
    if any(name in normalized for name in names[classification]) and isinstance(choice.get("id"), str):
        matches.append(choice["id"])
if classification == "hydrophobic" and len(matches) != 1:
    raise SystemExit(f"expected one visible hydrophobic choice, found {len(matches)}")
if classification == "hydrophilic" and not matches:
    raise SystemExit("expected at least one visible hydrophilic distractor")
print(matches[0])
PY
}

assert_completed_deferred_receipt() {
	python3 - "$1" "$2" <<'PY'
import json
import sys

document = json.load(open(sys.argv[1], encoding="utf-8"))
expected_score = float(sys.argv[2])
expected_correct = expected_score == 1.0
if document.get("accepted") is not True:
    raise SystemExit("submission receipt was not accepted")
result = document.get("attempt", {}).get("result")
feedback = document.get("feedback")
if not isinstance(result, dict) or not isinstance(feedback, dict):
    raise SystemExit("completed deferred WebWork receipt omitted its authorized result")
if result.get("correct") is not expected_correct or float(result.get("pointsEarned", -1)) != expected_score or float(result.get("pointsPossible", -1)) != 1.0:
    raise SystemExit("completed deferred WebWork receipt carried the wrong result")
if set(feedback) != {"correctness", "pointsEarned", "pointsPossible"}:
    raise SystemExit("completed deferred WebWork feedback exceeded its exact allowlist")
if feedback.get("correctness") is not expected_correct or float(feedback.get("pointsEarned", -1)) != expected_score or float(feedback.get("pointsPossible", -1)) != 1.0:
    raise SystemExit("completed deferred WebWork feedback carried the wrong result")
for key in ("answerKey", "correctResponse", "gradingPayload", "privateGrading", "problemSource", "passwd", "AnSwEr"):
    if key in json.dumps(document, separators=(",", ":")):
        raise SystemExit(f"submission receipt leaked {key}")
PY
}

assert_summary_score() {
	python3 - "$1" "$2" <<'PY'
import json
import sys

document = json.load(open(sys.argv[1], encoding="utf-8"))
expected = float(sys.argv[2])
for value in (document.get("run", {}).get("score"), document.get("summary", {}).get("latestScore")):
    if value != expected:
        raise SystemExit(f"expected completed-run score {expected}, got {value!r}")
if document.get("run", {}).get("completedAt") is None:
    raise SystemExit("one-question run did not complete")
PY
}

source source_me.sh
# Required/no-SKIP acceptance creates a full private fixture rather than
# starting or reusing the retained default `containers` project. The typed
# owner is the only route that can mutate and clean this project.
fixture_receipt="$(python3 tests/e2e/e2e_webwork_browser_fixture.py "$STACK_DIRECTORY")"
ENV_FILE="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["env"])' "$fixture_receipt")"
STACK_MANIFEST_FILE="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["manifest"])' "$fixture_receipt")"
CREDENTIAL_FILE="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["login"])' "$fixture_receipt")"
PROJECT_NAME="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["project"])' "$fixture_receipt")"
STACK_STARTED=1
adapter launch --timeout-seconds 240
require_file "$CREDENTIAL_FILE"
postgres_user="$(env_value POSTGRES_USER)"
postgres_password="$(env_value POSTGRES_PASSWORD)"
postgres_database="$(env_value POSTGRES_DB)"
postgres_port="$(env_value PLE_POSTGRES_HOST_PORT)"
minio_port="$(env_value PLE_MINIO_API_HOST_PORT)"
question_id_secret_file="$(env_value PLE_QUESTION_ID_SECRET_HOST_FILE)"
for required_value in "$postgres_user" "$postgres_password" "$postgres_database" "$postgres_port" "$minio_port"; do
	[ -n "$required_value" ] || fail "renderer fixture requires complete local PostgreSQL and MinIO configuration"
done
require_file "$question_id_secret_file"
case "$postgres_port" in ''|*[!0-9]*) fail "PLE_POSTGRES_HOST_PORT is not an integer" ;; esac
case "$minio_port" in ''|*[!0-9]*) fail "PLE_MINIO_API_HOST_PORT is not an integer" ;; esac
database_url="postgres://${postgres_user}:${postgres_password}@127.0.0.1:${postgres_port}/${postgres_database}"
AWS_ACCESS_KEY_ID="$(env_value MINIO_ROOT_USER)" \
AWS_SECRET_ACCESS_KEY="$(env_value MINIO_ROOT_PASSWORD)" \
	PLE_QUESTION_ID_SECRET_FILE="$question_id_secret_file" \
	cargo tools e2e-seed --webwork-pilot \
		--database-url "$database_url" \
		--apply-migrations \
		--tenant "00000000-0000-0000-0000-000000000100" \
		--instructor "00000000-0000-0000-0000-000000000101" \
		--student "00000000-0000-0000-0000-000000000102" \
		--s3-endpoint "http://127.0.0.1:${minio_port}" \
		--s3-region "us-east-1" \
		--private-content-bucket "private-content" >"$FIXTURE_MANIFEST_FILE"
chmod 600 "$FIXTURE_MANIFEST_FILE"
require_file "$FIXTURE_MANIFEST_FILE"

gateway_port="$(env_value PLE_GATEWAY_HOST_PORT)"
gateway_port="${gateway_port:-8080}"
case "$gateway_port" in ''|*[!0-9]*) fail "PLE_GATEWAY_HOST_PORT is not an integer" ;; esac
BASE_URL="http://127.0.0.1:${gateway_port}"
ASSIGNMENT_ID="$(json_value "$FIXTURE_MANIFEST_FILE" assignmentId)"
student_credential="$(awk -F= '$1 == "student" { print $2 }' "$CREDENTIAL_FILE")"
[ -n "$student_credential" ] || fail "local student login credential is absent"

login_file="$WORK_DIRECTORY/login.json"
login_status="$(gateway_request POST /api/auth/login "$login_file" --header 'content-type: application/json' --data "{\"credential\":\"${student_credential}\"}")"
[ "$login_status" = "200" ] || fail "gateway login returned HTTP $login_status"

# Start one run and fetch its same attempt twice. The structured PLE server
# evidence below proves that issuance may use the adapter cache, while each GET
# replays the persisted attempt snapshot without adapter work.
run_one="$WORK_DIRECTORY/run-one.json"
attempts_one="$WORK_DIRECTORY/attempts-one.json"
api_before_run="$WORK_DIRECTORY/api-before-run.log"
api_after_run="$WORK_DIRECTORY/api-after-run.log"
api_after_first="$WORK_DIRECTORY/api-after-first.log"
api_after_second="$WORK_DIRECTORY/api-after-second.log"
adapter read-evidence-logs >"$api_before_run" 2>&1 || fail "cannot read PLE API structured evidence"
attempt_one="$(create_attempt "$run_one" "$attempts_one")"
adapter read-evidence-logs >"$api_after_run" 2>&1 || fail "cannot read PLE API evidence after run creation"
question_one="$WORK_DIRECTORY/question-one.json"
first_status="$(gateway_request GET "/api/attempts/${attempt_one}/question" "$question_one")"
[ "$first_status" = "200" ] || fail "first PLE WebWork question request returned HTTP $first_status"
adapter read-evidence-logs >"$api_after_first" 2>&1 || fail "cannot read PLE API evidence after first question GET"
question_two="$WORK_DIRECTORY/question-two.json"
second_status="$(gateway_request GET "/api/attempts/${attempt_one}/question" "$question_two")"
[ "$second_status" = "200" ] || fail "second PLE WebWork question request returned HTTP $second_status"
adapter read-evidence-logs >"$api_after_second" 2>&1 || fail "cannot read PLE API evidence after second question GET"
cmp --silent "$question_one" "$question_two" || fail "same PLE WebWork attempt did not project identically from its persisted snapshot"

structured_event_count() {
	# `ple.webwork.cache` is a deliberately non-sensitive server event target.
	# The Rust contract emits only the event name, never tenant/source/seed,
	# renderer credentials, cache keys, answer fields, or response data.
	local pattern="ple[.]webwork[.]cache.*event=\"${2}\""
	grep -Ec "$pattern" "$1" || true
}
calls_before="$(structured_event_count "$api_before_run" renderer_call)"
hits_before="$(structured_event_count "$api_before_run" cache_hit)"
calls_after_run="$(structured_event_count "$api_after_run" renderer_call)"
hits_after_run="$(structured_event_count "$api_after_run" cache_hit)"
calls_after_first="$(structured_event_count "$api_after_first" renderer_call)"
hits_after_first="$(structured_event_count "$api_after_first" cache_hit)"
calls_after_second="$(structured_event_count "$api_after_second" renderer_call)"
hits_after_second="$(structured_event_count "$api_after_second" cache_hit)"
# A prior failed local acceptance can leave this deterministic pilot's active
# run resumable. Fresh creation emits one renderer call; safe resumption emits
# none. Neither same-attempt GET may call the adapter or emit its safe-cache
# witness, because both must return the authoritative persisted snapshot.
[ "$calls_after_run" -eq "$calls_before" ] || [ "$calls_after_run" -eq $((calls_before + 1)) ] || fail "run creation emitted an invalid WebWork renderer_call count"
[ "$calls_after_first" -eq "$calls_after_run" ] || fail "first PLE question GET made an unexpected renderer call"
[ "$hits_after_first" -eq "$hits_after_run" ] || fail "first PLE question GET emitted an unexpected WebWork cache_hit event"
[ "$calls_after_second" -eq "$calls_after_first" ] || fail "same-attempt replay made an unexpected renderer call"
[ "$hits_after_second" -eq "$hits_after_first" ] || fail "same-attempt replay emitted an unexpected WebWork cache_hit event"

correct_choice="$(choose_visible_answer "$question_one" hydrophobic)"
receipt_one="$WORK_DIRECTORY/receipt-one.json"
correct_submission="{\"response\":{\"kind\":\"multipleChoice\",\"selected\":[\"${correct_choice}\"]}}"
submit_one_status="$(gateway_request POST "/api/submissions/${attempt_one}" "$receipt_one" --header 'content-type: application/json' --header 'idempotency-key: ple-webwork-correct-1' --data "$correct_submission")"
[ "$submit_one_status" = "200" ] || fail "correct PLE WebWork submission returned HTTP $submit_one_status"
assert_completed_deferred_receipt "$receipt_one" 1.0
receipt_one_replay="$WORK_DIRECTORY/receipt-one-replay.json"
replay_status="$(gateway_request POST "/api/submissions/${attempt_one}" "$receipt_one_replay" --header 'content-type: application/json' --header 'idempotency-key: ple-webwork-correct-1' --data "$correct_submission")"
[ "$replay_status" = "200" ] || fail "idempotent WebWork submission replay returned HTTP $replay_status"
cmp --silent "$receipt_one" "$receipt_one_replay" || fail "idempotent WebWork submission replay changed its receipt"
summary_one="$WORK_DIRECTORY/summary-one.json"
run_one_id="$(json_value "$run_one" id)"
summary_status="$(gateway_request GET "/api/runs/${run_one_id}/summary?pageSize=1" "$summary_one")"
[ "$summary_status" = "200" ] || fail "correct WebWork run summary returned HTTP $summary_status"
assert_summary_score "$summary_one" 1.0

run_two="$WORK_DIRECTORY/run-two.json"
attempts_two="$WORK_DIRECTORY/attempts-two.json"
api_before_second_run="$WORK_DIRECTORY/api-before-second-run.log"
api_after_second_run="$WORK_DIRECTORY/api-after-second-run.log"
adapter read-evidence-logs >"$api_before_second_run" 2>&1 || fail "cannot read PLE API evidence before the fresh second run"
attempt_two="$(create_attempt "$run_two" "$attempts_two")"
adapter read-evidence-logs >"$api_after_second_run" 2>&1 || fail "cannot read PLE API evidence after the fresh second run"
calls_before_second_run="$(structured_event_count "$api_before_second_run" renderer_call)"
hits_before_second_run="$(structured_event_count "$api_before_second_run" cache_hit)"
calls_after_second_run="$(structured_event_count "$api_after_second_run" renderer_call)"
hits_after_second_run="$(structured_event_count "$api_after_second_run" cache_hit)"
# Continued practice creates a fresh random seed/key, so this live issuance
# proves exactly one renderer call but cannot deterministically prove a cache
# hit. A rare key collision can produce one cache hit; offline adapter tests
# own the deterministic same-seed cache-hit contract.
[ "$calls_after_second_run" -eq $((calls_before_second_run + 1)) ] || fail "fresh continued-practice issuance did not produce exactly one WebWork renderer_call event"
second_run_hit_delta=$((hits_after_second_run - hits_before_second_run))
case "$second_run_hit_delta" in
	0|1) ;;
	*) fail "fresh continued-practice issuance emitted an invalid WebWork cache_hit delta: $second_run_hit_delta" ;;
esac
question_three="$WORK_DIRECTORY/question-three.json"
third_status="$(gateway_request GET "/api/attempts/${attempt_two}/question" "$question_three")"
[ "$third_status" = "200" ] || fail "second PLE WebWork question request returned HTTP $third_status"
incorrect_choice="$(choose_visible_answer "$question_three" hydrophilic)"
receipt_two="$WORK_DIRECTORY/receipt-two.json"
submit_two_status="$(gateway_request POST "/api/submissions/${attempt_two}" "$receipt_two" --header 'content-type: application/json' --header 'idempotency-key: ple-webwork-incorrect-1' --data "{\"response\":{\"kind\":\"multipleChoice\",\"selected\":[\"${incorrect_choice}\"]}}")"
[ "$submit_two_status" = "200" ] || fail "incorrect PLE WebWork submission returned HTTP $submit_two_status"
assert_completed_deferred_receipt "$receipt_two" 0.0
summary_two="$WORK_DIRECTORY/summary-two.json"
run_two_id="$(json_value "$run_two" id)"
summary_status="$(gateway_request GET "/api/runs/${run_two_id}/summary?pageSize=1" "$summary_two")"
[ "$summary_status" = "200" ] || fail "incorrect WebWork run summary returned HTTP $summary_status"
assert_summary_score "$summary_two" 0.0

# Renderer failure is local to WebWork.  Native health remains available and a
# fresh WebWork attempt fails closed with 503 rather than falling back.
adapter stop-outage-service >/dev/null
renderer_stopped=1
restore_renderer() {
	if [ "${renderer_stopped:-0}" = 1 ]; then
		# The typed owner recreates this stateless renderer and reproves it ready.
		adapter restart --service webwork-renderer --timeout-seconds 240 >/dev/null
		renderer_stopped=0
	fi
}
finish() {
	local status="$?"
	restore_renderer || status=1
	cleanup_stack "$status"
}
trap finish EXIT
health_file="$WORK_DIRECTORY/health.json"
health_status="$(gateway_request GET /health "$health_file")"
[ "$health_status" = "200" ] || fail "native gateway health was not isolated from renderer outage"
outage_run="$WORK_DIRECTORY/outage-run.json"
outage_status="$(gateway_request POST /api/runs "$outage_run" --header 'content-type: application/json' --data "{\"assignmentId\":\"${ASSIGNMENT_ID}\"}")"
[ "$outage_status" = "503" ] || fail "renderer outage did not fail WebWork run issuance closed with HTTP 503 (got $outage_status)"
restore_renderer

assert_no_private_material "$login_file" "$run_one" "$attempts_one" "$question_one" "$question_two" "$receipt_one" "$receipt_one_replay" "$summary_one" "$run_two" "$attempts_two" "$question_three" "$receipt_two" "$summary_two" "$health_file" "$outage_run" "$api_before_run" "$api_after_run" "$api_after_first" "$api_after_second"
export PLE_WEBWORK_LIVE_REQUIRED=1
export PLE_WEBWORK_LIVE_BASE_URL="$BASE_URL"
export PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE="$CREDENTIAL_FILE"
export PLE_WEBWORK_LIVE_ASSIGNMENT_ID="$ASSIGNMENT_ID"
[ -f tests/playwright/webwork_run.spec.ts ] || fail "required browser acceptance spec tests/playwright/webwork_run.spec.ts is missing"
npx playwright test --config playwright.config.ts tests/playwright/webwork_run.spec.ts
echo "PASS: PLE WebWork live acceptance proved safe gateway projection, persisted same-attempt snapshot replay without adapter events, fresh issuance renderer evidence with an optional random-key cache hit, full/zero scoring, renderer-outage isolation, and private-material non-disclosure."
