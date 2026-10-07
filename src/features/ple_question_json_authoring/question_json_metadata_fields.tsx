// question_json_metadata_fields.tsx - bounded metadata controls for a Native Question draft.

import { type JSX } from "solid-js";

import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { PleQuestionJsonRecordMetadata } from "./question_json_defaults";

export interface PleQuestionJsonMetadataFieldsProps {
  readonly metadata: PleQuestionJsonRecordMetadata;
  readonly onMetadataChange: (metadata: PleQuestionJsonRecordMetadata) => void;
  readonly disabled?: boolean;
}

const QUESTION_LICENSES: ReadonlyArray<{
  readonly value: QuestionLicense;
  readonly label: string;
}> = [
  { value: "CC0-1.0", label: "CC0 1.0" },
  { value: "CC-BY-4.0", label: "CC BY 4.0" },
  { value: "CC-BY-SA-4.0", label: "CC BY-SA 4.0" },
];

function uniqueTags(value: string): ReadonlyArray<string> {
  return [
    ...new Set(
      value
        .split(",")
        .map((tag) => tag.trim())
        .filter((tag) => tag.length > 0),
    ),
  ];
}

function isQuestionLicense(value: string): value is QuestionLicense {
  return QUESTION_LICENSES.some((license) => license.value === value);
}

/** Metadata remains deliberate and compact. */
export function PleQuestionJsonMetadataFields(
  props: PleQuestionJsonMetadataFieldsProps,
): JSX.Element {
  const update = (change: Partial<PleQuestionJsonRecordMetadata>): void =>
    props.onMetadataChange({ ...props.metadata, ...change });

  return (
    <fieldset>
      <legend>Question record metadata</legend>
      <label class="ple-question-json-authoring__field">
        <span>Question Title</span>
        <input
          value={props.metadata.questionTitle}
          disabled={props.disabled}
          onInput={(event) => update({ questionTitle: event.currentTarget.value })}
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Question Description for Instructors</span>
        <textarea
          value={props.metadata.questionDescription}
          disabled={props.disabled}
          onInput={(event) => update({ questionDescription: event.currentTarget.value })}
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Tags (comma-separated)</span>
        <input
          value={props.metadata.tags.join(", ")}
          disabled={props.disabled}
          onInput={(event) => update({ tags: uniqueTags(event.currentTarget.value) })}
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Language</span>
        <input
          value={props.metadata.language ?? ""}
          disabled={props.disabled}
          onInput={(event) =>
            update({
              language: event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            })
          }
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Question License</span>
        <select
          value={props.metadata.questionLicense ?? ""}
          disabled={props.disabled}
          onChange={(event) => {
            const value = event.currentTarget.value;
            update({ questionLicense: isQuestionLicense(value) ? value : null });
          }}
        >
          <option value="">Select a Question License before publication</option>
          {QUESTION_LICENSES.map((license) => (
            <option value={license.value}>{license.label}</option>
          ))}
        </select>
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Citation</span>
        <textarea
          value={props.metadata.questionCitation ?? ""}
          disabled={props.disabled}
          onInput={(event) => {
            const value = event.currentTarget.value;
            update({ questionCitation: value === "" ? null : value });
          }}
        />
      </label>
    </fieldset>
  );
}
