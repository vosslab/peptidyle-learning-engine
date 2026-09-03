// question_json_metadata_fields.tsx - bounded metadata controls for a ple-question-json draft.

import { type JSX } from "solid-js";

import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionCitation } from "../../../generated/api/QuestionCitation";

export interface PleQuestionJsonMetadataFieldsProps {
  readonly questionDescription: string;
  readonly tags: ReadonlyArray<string>;
  readonly questionLicense: QuestionLicense | null;
  readonly questionCitation: QuestionCitation | null;
  readonly language: string;
  readonly onTagsChange: (tags: ReadonlyArray<string>) => void;
  readonly onQuestionDescriptionChange: (questionDescription: string) => void;
  readonly onQuestionLicenseChange: (questionLicense: QuestionLicense | null) => void;
  readonly onQuestionCitationChange: (questionCitation: QuestionCitation | null) => void;
  readonly onLanguageChange: (language: string) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
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

function citationWith(
  citation: QuestionCitation | null,
  field: "citationUrl" | "citationText",
  value: string,
): QuestionCitation | null {
  const next = {
    citationUrl: citation?.citationUrl ?? null,
    citationText: citation?.citationText ?? null,
    [field]: value.trim() === "" ? null : value,
  };
  return next.citationUrl === null && next.citationText === null ? null : next;
}

/** Metadata remains deliberate and compact. */
export function PleQuestionJsonMetadataFields(
  props: PleQuestionJsonMetadataFieldsProps,
): JSX.Element {
  return (
    <fieldset>
      <legend>Question Library metadata</legend>
      <label class="ple-question-json-authoring__field">
        <span>Question Description for Instructors</span>
        <textarea
          value={props.questionDescription}
          disabled={props.disabled}
          onInput={(event) => props.onQuestionDescriptionChange(event.currentTarget.value)}
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Tags (comma-separated)</span>
        <input
          value={props.tags.join(", ")}
          disabled={props.disabled}
          onInput={(event) => props.onTagsChange(uniqueTags(event.currentTarget.value))}
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Language</span>
        <input
          value={props.language}
          disabled={props.disabled}
          onInput={(event) => props.onLanguageChange(event.currentTarget.value)}
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Question License</span>
        <select
          value={props.questionLicense ?? ""}
          disabled={props.disabled}
          onChange={(event) => {
            const value = event.currentTarget.value;
            props.onQuestionLicenseChange(isQuestionLicense(value) ? value : null);
          }}
        >
          <option value="">Select a Question License before publication</option>
          {QUESTION_LICENSES.map((license) => (
            <option value={license.value}>{license.label}</option>
          ))}
        </select>
      </label>
      <label class="ple-question-json-authoring__field">
        <span>Citation URL</span>
        <input
          type="url"
          value={props.questionCitation?.citationUrl ?? ""}
          disabled={props.disabled}
          onInput={(event) =>
            props.onQuestionCitationChange(
              citationWith(props.questionCitation, "citationUrl", event.currentTarget.value),
            )
          }
        />
      </label>
      <label class="ple-question-json-authoring__field">
        <span>NLM-style Citation Text</span>
        <textarea
          value={props.questionCitation?.citationText ?? ""}
          disabled={props.disabled}
          onInput={(event) =>
            props.onQuestionCitationChange(
              citationWith(props.questionCitation, "citationText", event.currentTarget.value),
            )
          }
        />
      </label>
    </fieldset>
  );
}
