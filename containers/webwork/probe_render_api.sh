#!/usr/bin/env bash
# Exercises the standalone renderer's opaque JSON envelope and one ordinary
# form submission. It never inspects rendered controls or response HTML.
set -euo pipefail
[ "${1:-}" = "" ] || [ "$1" = "--exercise" ] || exit 2

# This is the stable one-field RadioButtons shape used by the existing
# which_hydrophobic-simple.pgml Question Source. Keeping the request minimal
# makes the renderer availability check independent of PLE presentation code.
source_text="$(printf '%s\n' 'DOCUMENT();' 'loadMacros("PGstandard.pl", "PGML.pl", "parserRadioButtons.pl");' '$choice = RadioButtons(["benzene", "water"], "benzene", labels => "ABC");' 'BEGIN_PGML' 'Which compound is hydrophobic?' '[_]{$choice}' 'END_PGML' 'ENDDOCUMENT();')"
encoded_source="$(printf '%s' "$source_text" | base64 | tr -d '\n')"
render_response="$(mktemp)"
grade_response="$(mktemp)"
trap 'rm -f "$render_response" "$grade_response"' EXIT

render_request() {
	output_file="$1"
	shift
	curl --fail --silent --show-error --max-time 15 --request POST http://127.0.0.1:3000/render-api \
		--data-urlencode '_format=json' --data-urlencode 'outputFormat=ple_embed' \
		--data-urlencode "problemSource=$encoded_source" \
		--data-urlencode 'sourceFilePath=private/which_hydrophobic-simple.pgml' \
		--data-urlencode 'problemSeed=271828' --data-urlencode 'displayMode=MathJax' \
		--data-urlencode 'isInstructor=0' --data-urlencode 'showHints=0' \
		--data-urlencode 'showSolutions=0' --data-urlencode 'showCorrectAnswers=0' \
		--data-urlencode 'showSummary=0' --data-urlencode 'pleOrigin=http://ple.example' \
		--data-urlencode 'pleAssetBase=http://ple.example/api/webwork-assets' \
		"$@" --output "$output_file"
}

validate_envelope() {
	perl -MJSON::PP -0777 -e '
		my $value = decode_json(<>);
		my @expected = qw(JWT debug flags problem_result problem_state renderedHTML resources);
		exit 1 unless ref($value) eq "HASH" && keys(%$value) == @expected;
		exit 1 if grep { !exists($value->{$_}) } @expected;
		exit 1 unless !ref($value->{renderedHTML}) && length($value->{renderedHTML});
		exit 1 unless ref($value->{problem_result}) eq "HASH";
		exit 1 unless defined($value->{problem_result}{score}) && $value->{problem_result}{score} =~ /\A(?:0|1)(?:\.0+)?\z/;
	' "$1"
}

render_request "$render_response"
validate_envelope "$render_response"
[ "${1:-}" = "--exercise" ] || exit 0

# An empty selection is an ordinary known answer field for this one-field PG
# source. The check proves form submission reaches the renderer without
# deriving a field name or value from backend HTML.
render_request "$grade_response" --data-urlencode 'submitAnswers=1' --data-urlencode 'AnSwEr0001='
validate_envelope "$grade_response"
perl -MJSON::PP -0777 -e 'my $value = decode_json(<>); exit !($value->{problem_result}{score} == 0);' "$grade_response"
