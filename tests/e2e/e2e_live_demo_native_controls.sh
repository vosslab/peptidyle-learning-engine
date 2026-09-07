#!/usr/bin/env bash
# Disposable M12 acceptance: real native static-PLE response formats, never submission or grading.

set -euo pipefail

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
readonly repository_root
readonly project_name="ple-live-demo-browser"
readonly runtime_environment_path="local_stack_state/live_demo_browser/workspace/env.local"

usage() {
	echo "Usage: bash tests/e2e/e2e_live_demo_native_controls.sh [--format|--keyboard]" >&2
}

mode="all"
case "${1:-}" in
	"") ;;
	--format) mode="format" ;;
	--keyboard) mode="keyboard" ;;
	*) usage; exit 2 ;;
esac

# shellcheck disable=SC1091
source "$repository_root/source_me.sh"
cd "$repository_root"

require_live_demo() {
	if [ ! -f "$runtime_environment_path" ]; then
		echo "Native response-control evidence requires the fixed Live Demo to be running" >&2
		exit 2
	fi
}

service_id() {
	local service="$1" identifiers
	identifiers="$(podman ps -q --filter "label=com.docker.compose.project=$project_name" --filter "label=com.docker.compose.service=$service")"
	if [ "$(printf '%s\n' "$identifiers" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Native response-control evidence requires one running $service service" >&2
		exit 1
	fi
	printf '%s\n' "$identifiers"
}

gateway_port() {
	local entries
	entries="$(rg --no-messages '^PLE_GATEWAY_HOST_PORT=[0-9]+$' "$runtime_environment_path" || true)"
	if [ "$(printf '%s\n' "$entries" | sed '/^$/d' | wc -l | tr -d '[:space:]')" != "1" ]; then
		echo "Native response-control evidence requires one validated gateway port setting" >&2
		exit 1
	fi
	printf '%s\n' "${entries#PLE_GATEWAY_HOST_PORT=}"
}

request() {
	local path="$1" cookie="${2:-}" method="${3:-GET}" body="${4:-}" content_type="${5:-application/json}" if_match="${6:-}"
	local gateway port
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	local -a args=(--silent --show-error --insecure --max-time 12 --write-out $'\n%{http_code}' --header "Host: localhost:$port" --request "$method")
	if [ "$method" != "GET" ]; then args+=(--header "Origin: https://localhost:$port" --header "Content-Type: $content_type"); fi
	if [ -n "$cookie" ]; then args+=(--header "Cookie: $cookie"); fi
	if [ -n "$if_match" ]; then args+=(--header "If-Match: \"$if_match\""); fi
	if [ -n "$body" ]; then args+=(--data "$body"); fi
	podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$path"
}

persona_cookie() {
	local persona="$1" gateway port headers cookie
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	headers="$(podman exec "$gateway" curl --silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --header "Host: localhost:$port" --header "Origin: https://localhost:$port" --header 'Content-Type: application/json' --request POST --data "{\"persona\":\"$persona\"}" 'https://localhost:8080/api/auth/live-demo/accounts')"
	cookie="$(printf '%s\n' "$headers" | sed -n 's/^set-cookie: \([^;]*\).*/\1/Ip' | head -n 1)"
	if [ -z "$cookie" ]; then echo "seeded demo did not issue an Authenticated Session" >&2; exit 1; fi
	printf '%s\n' "$cookie"
}

response_status() { printf '%s' "${1##*$'\n'}"; }
response_body() { printf '%s' "${1%$'\n'*}"; }

# Keep redirect headers private to the assertion process: this route deliberately contains an
# immutable asset location and ETag that are neither browser contracts nor test output.
asset_route_response() {
	local path="$1" cookie="${2:-}" gateway port
	gateway="$(service_id gateway)"; port="$(gateway_port)"
	local -a args=(--silent --show-error --insecure --max-time 12 --dump-header - --output /dev/null --write-out $'\n%{http_code}' --header "Host: localhost:$port")
	if [ -n "$cookie" ]; then args+=(--header "Cookie: $cookie"); fi
	podman exec "$gateway" curl "${args[@]}" "https://localhost:8080$path"
}

assert_ready_asset_redirect() {
	printf '%s' "$1" | python3 -c '
import sys
raw = sys.stdin.read().replace("\r", "")
lines = raw.rstrip().splitlines()
if not lines or lines[-1] != "302":
    raise SystemExit("authorized Question Asset route did not redirect")
blocks = [block for block in raw.rsplit("\n302", 1)[0].split("\n\n") if block.startswith("HTTP/")]
if not blocks:
    raise SystemExit("authorized Question Asset route omitted response headers")
headers = {}
for line in blocks[-1].splitlines()[1:]:
    if ":" in line:
        name, value = line.split(":", 1)
        headers[name.lower()] = value.strip()
if not headers.get("location"):
    raise SystemExit("authorized Question Asset route omitted its redirect location")
if headers.get("cache-control") != "public, max-age=31536000, immutable":
    raise SystemExit("authorized Question Asset route omitted immutable cache policy")
if headers.get("x-content-type-options") != "nosniff" or headers.get("referrer-policy") != "no-referrer":
    raise SystemExit("authorized Question Asset route omitted required security headers")
'
}

concealed_asset_shape() {
	printf '%s' "$1" | python3 -c '
import json, sys
raw = sys.stdin.read().replace("\r", "")
lines = raw.rstrip().splitlines()
if not lines or lines[-1] != "404":
    raise SystemExit("concealed Question Asset route did not return 404")
blocks = [block for block in raw.rsplit("\n404", 1)[0].split("\n\n") if block.startswith("HTTP/")]
headers = {}
if blocks:
    for line in blocks[-1].splitlines()[1:]:
        if ":" in line:
            name, value = line.split(":", 1)
            headers[name.lower()] = value.strip()
if "location" in headers or "etag" in headers:
    raise SystemExit("concealed Question Asset route disclosed rendition metadata")
safe = ("cache-control", "content-length", "content-type", "referrer-policy", "x-content-type-options")
print(json.dumps({name: headers.get(name) for name in safe}, sort_keys=True))
'
}

source_payload() {
	local format="$1"
	PLE_NATIVE_FORMAT="$format" python3 -c '
import json, os
kind = os.environ["PLE_NATIVE_FORMAT"]
responses = {
 "singleChoice": {"kind":"singleChoice","choices":[{"id":"one","text":"One"},{"id":"two","text":"Two"}],"correctChoice":"one"},
 "multipleAnswer": {"kind":"multipleAnswer","choices":[{"id":"one","text":"One"},{"id":"two","text":"Two"}],"correctChoices":["one"]},
 "fillIn": {"kind":"fillIn","answers":["one"],"matchMode":"normalized","maxLength":24},
 "multiFillIn": {"kind":"multiFillIn","blanks":[{"id":"first","label":"First","answers":["one"],"matchMode":"normalized","maxLength":24},{"id":"second","label":"Second","answers":["two"],"matchMode":"normalized","maxLength":24}]},
 "numeric": {"kind":"numeric","answer":1,"tolerance":{"kind":"exact"},"unit":None},
 "matching": {"kind":"matching","prompts":[{"id":"first","text":"First"},{"id":"second","text":"Second"}],"choices":[{"id":"one","text":"One"},{"id":"two","text":"Two"}],"matches":[{"prompt":"first","choice":"one"},{"prompt":"second","choice":"two"}]},
 "ordering": {"kind":"ordering","items":[{"id":"one","text":"One"},{"id":"two","text":"Two"},{"id":"three","text":"Three"}],"correctOrder":["one","two","three"]},
 }
print(json.dumps({"format":"pleQuestionJson","version":3,"questionTitle":f"M12 {kind}","questionDescription":f"Disposable native {kind} control evidence.","prompt":"Choose a response format-valid value.","response":responses[kind],"feedback":{"correct":None,"incorrect":None},"questionHint":None,"tags":["live-demo","m12-native"],"questionLicense":"CC-BY-4.0","questionCitation":None,"language":"en-US"}, separators=(",",":")))
'
}

draft_and_publish() {
	local format="$1" cookie="$2" created draft edit published
	created="$(request '/api/authoring/drafts' "$cookie" POST "$(source_payload "$format")" 'application/vnd.peptidyle.question+json')"
	if [ "$(response_status "$created")" != "201" ]; then echo "Instructor could not create native $format Draft Question" >&2; exit 1; fi
	read -r draft edit < <(python3 -c 'import json, re, sys; v=json.loads(sys.argv[1]); assert re.fullmatch(r"D-[1-9][0-9]*",v.get("draftQuestion","")); assert isinstance(v.get("editNumber"),int); print(v["draftQuestion"],v["editNumber"])' "$(response_body "$created")")
	published="$(request "/api/authoring/drafts/$draft/publish" "$cookie" POST '{"authors":["Live Demo Instructor"]}' 'application/json' "$edit")"
	if [ "$(response_status "$published")" != "200" ]; then echo "Instructor could not publish native $format Question" >&2; exit 1; fi
	python3 -c 'import json, re, sys; v=json.loads(sys.argv[1]); assert set(v)=={"questionId"} and re.fullmatch(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}",v["questionId"]); print(v["questionId"])' "$(response_body "$published")"
}

latest_course_reference() {
	local postgres
	postgres="$(service_id postgres)"
	podman exec "$postgres" sh -lc 'psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At -c "SELECT chr(67)||chr(45)||reference_number FROM ple_data.course_instance ORDER BY reference_number DESC LIMIT 1"'
}

claim_student_record() {
	local course="$1" instructor_cookie="$2" student_cookie="$3" imported claimed
	imported="$(request "/api/course-instances/$course/roster" "$instructor_cookie" POST '{"entries":[{"email":"mary.student@live-demo.invalid","rosterId":"m12-student"}]}')"
	if [ "$(response_status "$imported")" != "201" ]; then echo "Instructor could not import M12 Student roster row" >&2; exit 1; fi
	claimed="$(request "/api/course-instances/$course/roster/claim" "$student_cookie" POST '{}')"
	if [ "$(response_status "$claimed")" != "200" ]; then echo "Student could not claim M12 Course Invitation" >&2; exit 1; fi
}

prepared_course=""
prepared_assignment=""
prepared_student_cookie=""
issued_payload=""

assert_seeded_hotspot_available() {
	local course="$1" instructor_cookie="$2" picker
	picker="$(request "/api/course-instances/$course/assignment-question-picker" "$instructor_cookie")"
	if [ "$(response_status "$picker")" != "200" ]; then echo "Instructor could not load the fixed Question Picker" >&2; exit 1; fi
	python3 -c '
import json, sys
items = json.loads(sys.argv[1])
if not isinstance(items, list) or "PNE-0004" not in {item.get("questionId") for item in items if isinstance(item, dict)}:
    raise SystemExit("fixed HOTSPOT Question is unavailable to the Assignment Workspace")
' "$(response_body "$picker")"
}

create_and_issue_native_catalog() {
	local instructor_cookie student_cookie course created assignment edit saved released issued format
	local -a question_ids=() formats=(singleChoice multipleAnswer fillIn multiFillIn numeric matching ordering)
	# M10 supplies a direct-Instructor Course Instance; this runner then uses public M6/M10/M11 routes only.
	bash "$repository_root/tests/e2e/e2e_live_demo_assignment_release.sh" --service >/dev/null
	instructor_cookie="$(persona_cookie elenaInstructor)"; student_cookie="$(persona_cookie maryStudent)"
	course="$(latest_course_reference)"
	if ! [[ "$course" =~ ^C-[1-9][0-9]*$ ]]; then echo "M10 prerequisite did not produce a public Course Instance Reference" >&2; exit 1; fi
	assert_seeded_hotspot_available "$course" "$instructor_cookie"
	for format in "${formats[@]}"; do question_ids+=("$(draft_and_publish "$format" "$instructor_cookie")"); done
	# The seed/publisher owns PNE-0004 and its immutable public rendition.  Do not recreate the
	# HOTSPOT source or pass asset identity through this runner.
	question_ids+=("PNE-0004")
	created="$(request "/api/course-instances/$course/assignments" "$instructor_cookie" POST '{"title":"M12 native controls","instructions":"Use the native response controls."}')"
	if [ "$(response_status "$created")" != "201" ]; then echo "Instructor could not create M12 Assignment" >&2; exit 1; fi
	read -r assignment edit < <(python3 -c 'import json, re, sys; v=json.loads(sys.argv[1]); assert re.fullmatch(r"A-[1-9][0-9]*",v.get("reference","")); assert str(v.get("editNumber","")).isdigit(); print(v["reference"],v["editNumber"])' "$(response_body "$created")")
	local question_json; question_json="$(printf '%s\n' "${question_ids[@]}" | python3 -c 'import json,sys; print(json.dumps([line.strip() for line in sys.stdin if line.strip()]))')"
	saved="$(request "/api/course-instances/$course/assignments/$assignment" "$instructor_cookie" PUT "{\"title\":\"M12 native controls\",\"instructions\":\"Use the native response controls.\",\"questionIds\":$question_json,\"dueAt\":null,\"lateWorkRule\":\"accept\"}" 'application/json' "$edit")"
	if [ "$(response_status "$saved")" != "200" ]; then echo "Instructor could not save eight native M12 Questions" >&2; exit 1; fi
	edit="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["editNumber"])' "$(response_body "$saved")")"
	released="$(request "/api/course-instances/$course/assignments/$assignment/release" "$instructor_cookie" POST '{}' 'application/json' "$edit")"
	if [ "$(response_status "$released")" != "201" ]; then echo "Instructor could not release eight native M12 Questions" >&2; exit 1; fi
	claim_student_record "$course" "$instructor_cookie" "$student_cookie"
	issued="$(request "/api/course-instances/$course/assignments/$assignment/start" "$student_cookie" POST '{}')"
	if [ "$(response_status "$issued")" != "201" ]; then echo "Student could not issue the M12 native Question catalog" >&2; exit 1; fi
	prepared_course="$course"
	prepared_assignment="$assignment"
	prepared_student_cookie="$student_cookie"
	issued_payload="$(response_body "$issued")"
}

issued_hotspot_asset() {
	python3 -c '
import json, re, sys
questions = json.loads(sys.argv[1]).get("questions")
assets = [question.get("response", {}).get("surface", {}).get("questionAsset", {}).get("questionAsset") for question in questions if question.get("response", {}).get("kind") == "hotspot"]
if len(assets) != 1 or not isinstance(assets[0], str) or not re.fullmatch(r"[0-9a-fA-F-]{36}", assets[0]):
    raise SystemExit("issued HOTSPOT did not retain one logical Question Asset reference")
print(assets[0])
' "$issued_payload"
}

prove_asset_route() {
	local asset_id ready anonymous other_student absent malformed first_shape shape
	asset_id="$(issued_hotspot_asset)"
	ready="$(asset_route_response "/api/assets/$asset_id" "$prepared_student_cookie")"
	assert_ready_asset_redirect "$ready"
	anonymous="$(asset_route_response "/api/assets/$asset_id")"
	other_student="$(asset_route_response "/api/assets/$asset_id" "$(persona_cookie jackStudent)")"
	absent="$(asset_route_response '/api/assets/00000000-0000-0000-0000-000000009999' "$prepared_student_cookie")"
	malformed="$(asset_route_response '/api/assets/not-a-question-asset' "$prepared_student_cookie")"
	first_shape="$(concealed_asset_shape "$anonymous")"
	for shape in "$(concealed_asset_shape "$other_student")" "$(concealed_asset_shape "$absent")" "$(concealed_asset_shape "$malformed")"; do
		if [ "$shape" != "$first_shape" ]; then echo "concealed Question Asset responses are distinguishable" >&2; exit 1; fi
	done
	echo "Question Asset route: authorized immutable redirect and indistinguishable concealment complete"
}

prove_format() {
	create_and_issue_native_catalog
	# The actual answer-free issuance is decoded by the same strict browser contract.  The malformed
	# value is entirely in memory; this runner never calls an M13 submission or grading endpoint.
	node --import tsx --input-type=module -e '
import { decodeLiveAssignmentAttempt } from "./src/api/decoders/assignment_attempt_issuance.ts";
const actual = JSON.parse(process.argv[1]);
const decoded = decodeLiveAssignmentAttempt(actual);
const kinds = decoded.questions.map((question) => question.response.kind).sort();
const expected = ["fillIn", "hotspot", "matching", "multiFillIn", "multipleAnswer", "numerical", "ordering", "singleChoice"];
if (JSON.stringify(kinds) !== JSON.stringify(expected)) throw new Error("issued catalog did not contain exactly the eight native response formats");
const malformed = structuredClone(actual); malformed.questions[0].response.unexpected = true;
let rejected = false; try { decodeLiveAssignmentAttempt(malformed); } catch { rejected = true; }
if (!rejected) throw new Error("strict response decoder accepted an in-memory malformed presentation");
' "$issued_payload"
	prove_asset_route
	echo "Native response controls format: exact eight issued static PLE formats decode strictly before submission"
}

prove_keyboard() {
	create_and_issue_native_catalog
	node tests/playwright/e2e_live_demo_native_controls_browser.mjs "$(gateway_port)" "$prepared_course" "$prepared_assignment"
	echo "Native response controls keyboard: eight native controls, including the fixed HOTSPOT asset, reach valid local states without submission"
}

require_live_demo
case "$mode" in
	format) prove_format ;;
	keyboard) prove_keyboard ;;
	all) prove_format; prove_keyboard ;;
esac
echo "Live Demo Native Response Controls: PASS"
