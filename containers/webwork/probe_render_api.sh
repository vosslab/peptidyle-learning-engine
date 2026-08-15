#!/usr/bin/env bash
# Prints neither request nor response: both contain protected renderer state.
set -euo pipefail
[ "${1:-}" = "" ] || [ "$1" = "--exercise" ] || exit 2

source_text="$(printf '%s\n' \
	'DOCUMENT();' \
	'loadMacros("PGstandard.pl", "PGML.pl", "parserRadioButtons.pl");' \
	'$choice = RadioButtons(["blue", "red", "yellow"], "blue");' \
	'BEGIN_PGML' \
	'What is the best color?' \
	'[_]{$choice}' \
	'END_PGML' \
	'BEGIN_PGML_SOLUTION' \
	'The expected selection is blue.' \
	'END_PGML_SOLUTION' \
	'ENDDOCUMENT();')"
encoded_source="$(printf '%s' "$source_text" | base64 | tr -d '\n')"
render_response="$(mktemp)"
second_response="$(mktemp)"
correct_response="$(mktemp)"
incorrect_response="$(mktemp)"
trap 'rm -f "$render_response" "$second_response" "$correct_response" "$incorrect_response"' EXIT

render_request() {
	output_file="$1"
	shift
	curl --fail --silent --show-error --max-time 15 --request POST http://127.0.0.1:3000/render-api \
		--data-urlencode '_format=json' \
		--data-urlencode 'outputFormat=default' \
		--data-urlencode "problemSource=$encoded_source" \
		--data-urlencode 'sourceFilePath=private/ple_readiness.pg' \
		--data-urlencode 'problemSeed=271828' \
		--data-urlencode 'displayMode=MathJax' \
		--data-urlencode 'isInstructor=0' \
		--data-urlencode 'showHints=0' \
		--data-urlencode 'showSolutions=0' \
		--data-urlencode 'showSummary=0' \
		--data-urlencode 'hidePreviewButton=1' \
		--data-urlencode 'hideCheckAnswersButton=1' \
		--data-urlencode 'hideAttemptsTable=1' \
		--data-urlencode 'hideMessages=1' \
		--data-urlencode 'showCorrectAnswersButton=0' \
		--data-urlencode 'showFooter=0' \
		"$@" --output "$output_file"
}

validate_render_response() {
	perl -MJSON::PP -0777 -e '
	my $value = decode_json(<>);
	my @expected = qw(JWT debug flags problem_result problem_state renderedHTML resources);
	exit 1 unless ref($value) eq "HASH" && keys(%$value) == @expected;
	exit 1 if grep { !exists($value->{$_}) } @expected;
	exit 1 unless !ref($value->{renderedHTML}) && length($value->{renderedHTML});
	exit 1 unless ref($value->{JWT}) eq "HASH" && keys(%{$value->{JWT}}) == 3;
	for my $key (qw(problem session answer)) {
		my $token = $value->{JWT}{$key};
		exit 1 unless defined($token) && !ref($token) && $token =~ /^[A-Za-z0-9_.-]+$/;
		my @parts = split(/[.]/, $token, -1);
		exit 1 unless (@parts == 3 || @parts == 5) && !grep { $_ eq "" } @parts;
	}
	exit 1 unless ref($value->{problem_result}) eq "HASH"
		&& defined($value->{problem_result}{score})
		&& $value->{problem_result}{score} == 0;
	exit 1 if grep { exists($value->{$_}) } qw(answers inputs pgcore hidden_input_field body_part550);
' "$1"
}

project_public_render() {
	perl -MHTML::TreeBuilder -MJSON::PP -0777 -e '
	my $value = decode_json(<>);
	my $html = $value->{renderedHTML};
	my $tree = HTML::TreeBuilder->new;
	$tree->parse_content($html);
	my $body = $tree->look_down(id => "problem_body");
	exit 1 unless defined($body);
	my $visible_text = $body->as_text;
	$visible_text =~ s/\s+/ /g;
	$visible_text =~ s/^\s+|\s+$//g;
	my ($prompt) = $visible_text =~ /(What\s+is\s+the\s+best\s+color\?)/i;
	exit 1 unless defined($prompt);
	$prompt =~ s/\s+/ /g;
	exit 1 unless $prompt eq "What is the best color?";
	my %controls_by_id;
	for my $input ($body->look_down(_tag => "input")) {
		my $id = $input->attr("id");
		$controls_by_id{$id} = $input if defined($id) && length($id);
	}
	my @controls;
	for my $label ($body->look_down(_tag => "label")) {
		my @inputs = $label->look_down(_tag => "input");
		if (!@inputs) {
			my $for = $label->attr("for");
			@inputs = ($controls_by_id{$for}) if defined($for) && exists($controls_by_id{$for});
		}
		next unless @inputs == 1;
		my $input = $inputs[0];
		my $type = $input->attr("type");
		my $name = $input->attr("name");
		next unless defined($type) && lc($type) eq "radio" && defined($name) && $name =~ /^AnSwEr[0-9]+$/;
		my $label_text = $label->as_text;
		$label_text =~ s/\s+/ /g;
		$label_text =~ s/^\s+|\s+$//g;
		exit 1 unless length($label_text);
		push @controls, "radio:AnSwEr:$label_text";
	}
	exit 1 unless @controls;
	$tree->delete;
	print join("\x1e", $prompt, @controls);
' "$1"
}

render_request "$render_response"
validate_render_response "$render_response"
[ "${1:-}" = "--exercise" ] || exit 0

render_request "$second_response"
validate_render_response "$second_response"
first_public_projection="$(project_public_render "$render_response")"
second_public_projection="$(project_public_render "$second_response")"
[ "$first_public_projection" = "$second_public_projection" ]
answer_controls="$(perl -MJSON::PP -0777 -e '
	my $value = decode_json(<>);
	my $html = $value->{renderedHTML};
	my %controls;
	while ($html =~ m{<input\b([^>]*)>\s*([^<]+)</label>}gis) {
		my ($attributes, $label) = ($1, $2);
		my ($name) = $attributes =~ /\bname="(AnSwEr[0-9]+)"/i;
		my ($control_value) = $attributes =~ /\bvalue="([^"]+)"/i;
		$label =~ s/^\s+|\s+$//g;
		$controls{$label} = "$name\t$control_value" if defined($name) && defined($control_value);
	}
	exit 1 unless defined($controls{blue}) && defined($controls{red});
	print "$controls{blue}\n$controls{red}\n";
' "$render_response")"
correct_control="$(printf '%s\n' "$answer_controls" | sed -n '1p')"
incorrect_control="$(printf '%s\n' "$answer_controls" | sed -n '2p')"
answer_field="${correct_control%%$'\t'*}"
correct_value="${correct_control#*$'\t'}"
incorrect_field="${incorrect_control%%$'\t'*}"
incorrect_value="${incorrect_control#*$'\t'}"
[ -n "$answer_field" ] && [ "$answer_field" = "$incorrect_field" ]
render_request "$correct_response" --data-urlencode 'submitAnswers=1' --data-urlencode "${answer_field}=${correct_value}"
perl -MJSON::PP -0777 -e 'my $value = decode_json(<>); exit !(defined($value->{problem_result}{score}) && $value->{problem_result}{score} == 1);' "$correct_response"
render_request "$incorrect_response" --data-urlencode 'submitAnswers=1' --data-urlencode "${answer_field}=${incorrect_value}"
perl -MJSON::PP -0777 -e 'my $value = decode_json(<>); exit !(defined($value->{problem_result}{score}) && $value->{problem_result}{score} == 0);' "$incorrect_response"
