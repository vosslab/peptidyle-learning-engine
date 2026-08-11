#!/usr/bin/env bash
# Full PLE acceptance for the private, source-pinned WebWork renderer.
#
# This deliberately exercises only PLE's public gateway.  It never reads a
# renderer-internal token, request field, answer hash, source text, or upstream
# response. The direct renderer probe remains a container-readiness check.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

ENV_FILE="${PLE_E2E_ENV_FILE:-containers/env.local}"
E2E_ENV_DIRECTORY="$(dirname "$ENV_FILE")"
CREDENTIAL_FILE="${PLE_E2E_STUDENT_CREDENTIAL_FILE:-$E2E_ENV_DIRECTORY/local-login.txt}"
MANIFEST_FILE="${PLE_E2E_WEBWORK_MANIFEST_FILE:-$E2E_ENV_DIRECTORY/local-webwork-demo.json}"
WORK_DIRECTORY="$(mktemp -d "${TMPDIR:-/tmp}/ple-webwork-e2e.XXXXXX")"
COOKIE_JAR="$WORK_DIRECTORY/student.cookies"
trap 'rm -rf "$WORK_DIRECTORY"' EXIT

fail() {
	echo "FAIL: $*" >&2
	exit 1
}

require_file() {
	[ -r "$1" ] || fail "required local artifact is missing or unreadable: $1"
}

env_value() {
	awk -F= -v setting_name="$1" '$1 == setting_name { value = substr($0, index($0, "=") + 1) } END { print value }' "$ENV_FILE"
}

configure_compose() {
	if podman compose version >/dev/null 2>&1; then
		COMPOSE=(podman compose)
	elif command -v podman-compose >/dev/null 2>&1 && podman-compose version >/dev/null 2>&1; then
		COMPOSE=(podman-compose)
	else
		fail "no usable Podman Compose provider is available"
	fi
}

compose() {
	"${COMPOSE[@]}" -f containers/compose.yaml --env-file "$ENV_FILE" "$@"
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

require_file "$ENV_FILE"
configure_compose

# Required/no-SKIP acceptance: this is the supported all-in-one launch path.
./launch_local_stack.sh --no-open --env-file "$ENV_FILE"
require_file "$CREDENTIAL_FILE"
require_file "$MANIFEST_FILE"

gateway_port="$(env_value PLE_GATEWAY_HOST_PORT)"
gateway_port="${gateway_port:-8080}"
case "$gateway_port" in ''|*[!0-9]*) fail "PLE_GATEWAY_HOST_PORT is not an integer" ;; esac
BASE_URL="http://127.0.0.1:${gateway_port}"
ASSIGNMENT_ID="$(json_value "$MANIFEST_FILE" assignmentId)"
student_credential="$(awk -F= '$1 == "student" { print $2 }' "$CREDENTIAL_FILE")"
[ -n "$student_credential" ] || fail "local student login credential is absent"

login_file="$WORK_DIRECTORY/login.json"
login_status="$(gateway_request POST /api/auth/login "$login_file" --header 'content-type: application/json' --data "{\"credential\":\"${student_credential}\"}")"
[ "$login_status" = "200" ] || fail "gateway login returned HTTP $login_status"

# Start one run and fetch its same attempt twice. The structured PLE server
# evidence below proves that issuing renders once while both GETs are cache hits.
run_one="$WORK_DIRECTORY/run-one.json"
attempts_one="$WORK_DIRECTORY/attempts-one.json"
api_before_run="$WORK_DIRECTORY/api-before-run.log"
api_after_run="$WORK_DIRECTORY/api-after-run.log"
api_after_first="$WORK_DIRECTORY/api-after-first.log"
api_after_second="$WORK_DIRECTORY/api-after-second.log"
compose logs --no-color api >"$api_before_run" 2>&1 || fail "cannot read PLE API structured evidence"
attempt_one="$(create_attempt "$run_one" "$attempts_one")"
compose logs --no-color api >"$api_after_run" 2>&1 || fail "cannot read PLE API evidence after run creation"
question_one="$WORK_DIRECTORY/question-one.json"
first_status="$(gateway_request GET "/api/attempts/${attempt_one}/question" "$question_one")"
[ "$first_status" = "200" ] || fail "first PLE WebWork question request returned HTTP $first_status"
compose logs --no-color api >"$api_after_first" 2>&1 || fail "cannot read PLE API evidence after first question GET"
question_two="$WORK_DIRECTORY/question-two.json"
second_status="$(gateway_request GET "/api/attempts/${attempt_one}/question" "$question_two")"
[ "$second_status" = "200" ] || fail "second PLE WebWork question request returned HTTP $second_status"
compose logs --no-color api >"$api_after_second" 2>&1 || fail "cannot read PLE API evidence after second question GET"
cmp --silent "$question_one" "$question_two" || fail "same PLE WebWork attempt did not project identically on cache replay"

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
# run resumable. Fresh creation emits one miss; safe resumption emits none.
[ "$calls_after_run" -eq "$calls_before" ] || [ "$calls_after_run" -eq $((calls_before + 1)) ] || fail "run creation emitted an invalid WebWork renderer_call count"
[ "$calls_after_first" -eq "$calls_after_run" ] || fail "first PLE question GET made an unexpected renderer call"
[ "$hits_after_first" -eq $((hits_after_run + 1)) ] || fail "first PLE question GET did not produce exactly one WebWork cache_hit event"
[ "$calls_after_second" -eq "$calls_after_first" ] || fail "same-attempt replay made an unexpected renderer call"
[ "$hits_after_second" -eq $((hits_after_first + 1)) ] || fail "same-attempt replay did not produce exactly one WebWork cache_hit event"
[ "$hits_after_second" -gt "$hits_before" ] || fail "WebWork cache-hit evidence was unexpectedly all-zero"

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
compose logs --no-color api >"$api_before_second_run" 2>&1 || fail "cannot read PLE API evidence before the fresh second run"
attempt_two="$(create_attempt "$run_two" "$attempts_two")"
compose logs --no-color api >"$api_after_second_run" 2>&1 || fail "cannot read PLE API evidence after the fresh second run"
calls_before_second_run="$(structured_event_count "$api_before_second_run" renderer_call)"
calls_after_second_run="$(structured_event_count "$api_after_second_run" renderer_call)"
[ "$calls_after_second_run" -eq $((calls_before_second_run + 1)) ] || fail "fresh continued-practice run did not produce exactly one WebWork renderer_call cache-miss event"
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
compose stop webwork-renderer >/dev/null
renderer_stopped=1
restore_renderer() {
	if [ "${renderer_stopped:-0}" = 1 ]; then
		# Hypnotoad leaves manager state in the container tmpfs across a plain
		# stop/start. Recreate this stateless service so recovery starts from the
		# same clean image boundary as a normal launch.
		compose up -d --force-recreate --no-deps webwork-renderer >/dev/null
		renderer_stopped=0
		renderer_container_id="$(podman ps \
			--filter label=io.podman.compose.project=containers \
			--filter label=io.podman.compose.service=webwork-renderer \
			--format '{{.ID}}')"
		[ -n "$renderer_container_id" ] || fail "restored renderer container was not found"
		started_at=$SECONDS
		until podman exec -i "$renderer_container_id" bash -s -- <containers/webwork/probe_render_api.sh >/dev/null 2>&1; do
			[ $((SECONDS - started_at)) -lt 180 ] || fail "renderer did not regain readiness after outage restoration"
			sleep 2
		done
	fi
}
trap 'restore_renderer; rm -rf "$WORK_DIRECTORY"' EXIT
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
bash run_playwright_tests.sh tests/playwright/webwork_run.spec.ts
echo "PASS: PLE WebWork live acceptance proved safe gateway projection, same-attempt cache replay, full/zero scoring, renderer-outage isolation, and private-material non-disclosure."
