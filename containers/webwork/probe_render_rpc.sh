#!/usr/bin/env bash
# Prints neither request nor response: both can contain protected material.
set -euo pipefail
[ "${1:-}" = "" ] || [ "$1" = "--exercise" ] || exit 2
[ -r "$PLE_WEBWORK_RENDER_PASSWORD_FILE" ] || exit 1
render_password="$(cat "$PLE_WEBWORK_RENDER_PASSWORD_FILE")"
source_text="$(printf '%s\n' \
	'DOCUMENT();' \
	'loadMacros("PGstandard.pl", "parserRadioButtons.pl");' \
	'TEXT(beginproblem());' \
	'$choice = RadioButtons(["blue", "red", "yellow"], "blue");' \
	'BEGIN_TEXT' \
	'What is the best color? $choice->buttons()' \
	'END_TEXT' \
	'ANS($choice->cmp);' \
	'ENDDOCUMENT();')"
encoded_source="$(printf '%s' "$source_text" | base64 | tr -d '\n')"
response_file="$(mktemp)"
second_response_file="$(mktemp)"
grade_response_file="$(mktemp)"
incorrect_grade_response_file="$(mktemp)"
trap 'rm -f "$response_file" "$second_response_file" "$grade_response_file" "$incorrect_grade_response_file"' EXIT
render_request() {
	output_file="$1"
	curl --fail --silent --show-error --max-time 10 --request POST http://127.0.0.1:8080/webwork2/render_rpc \
	--data-urlencode "courseID=${PLE_WEBWORK_RENDER_COURSE_ID}" \
	--data-urlencode "user=${PLE_WEBWORK_RENDER_USER}" \
	--data-urlencode "passwd=${render_password}" \
	--data-urlencode "problemSource=${encoded_source}" \
	--data-urlencode 'fileName=ple_readiness.pg' --data-urlencode 'problemSeed=271828' \
	--data-urlencode 'outputformat=json' --data-urlencode 'displayMode=MathJax' --output "$output_file"
}
render_request "$response_file"
grep -q '"body_part' "$response_file"
! grep -q '"error"' "$response_file"
[ "${1:-}" = "--exercise" ] || exit 0

render_request "$second_response_file"
cmp --silent "$response_file" "$second_response_file"
answer_field="$(sed -n 's/.*name=\\\"\\\(AnSwEr[0-9][0-9]*\\\)\\\".*/\\1/p' "$response_file" | head -n 1)"
[ -n "$answer_field" ]
curl --fail --silent --show-error --max-time 10 --request POST http://127.0.0.1:8080/webwork2/render_rpc \
	--data-urlencode "courseID=${PLE_WEBWORK_RENDER_COURSE_ID}" \
	--data-urlencode "user=${PLE_WEBWORK_RENDER_USER}" \
	--data-urlencode "passwd=${render_password}" \
	--data-urlencode "problemSource=${encoded_source}" \
	--data-urlencode 'fileName=ple_readiness.pg' --data-urlencode 'problemSeed=271828' \
	--data-urlencode 'outputformat=json' --data-urlencode 'displayMode=MathJax' \
	--data-urlencode 'WWsubmit=1' --data-urlencode "${answer_field}=blue" --output "$grade_response_file"
! grep -q '"error"' "$grade_response_file"
grep -Eq '"score"[[:space:]]*:[[:space:]]*100([.0]|,|})' "$grade_response_file"
curl --fail --silent --show-error --max-time 10 --request POST http://127.0.0.1:8080/webwork2/render_rpc \
	--data-urlencode "courseID=${PLE_WEBWORK_RENDER_COURSE_ID}" \
	--data-urlencode "user=${PLE_WEBWORK_RENDER_USER}" \
	--data-urlencode "passwd=${render_password}" \
	--data-urlencode "problemSource=${encoded_source}" \
	--data-urlencode 'fileName=ple_readiness.pg' --data-urlencode 'problemSeed=271828' \
	--data-urlencode 'outputformat=json' --data-urlencode 'displayMode=MathJax' \
	--data-urlencode 'WWsubmit=1' --data-urlencode "${answer_field}=red" --output "$incorrect_grade_response_file"
! grep -q '"error"' "$incorrect_grade_response_file"
grep -Eq '"score"[[:space:]]*:[[:space:]]*0([.0]|,|})' "$incorrect_grade_response_file"
