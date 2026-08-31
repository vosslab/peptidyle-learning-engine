// flat_metadata_fields.tsx - bounded metadata controls for a flat-question draft.

import { For, Show, type JSX } from "solid-js";

import type { FlatQuestionClassification, FlatQuestionLicense } from "./flat_question_source";

const MAXIMUM_CLASSIFICATIONS = 32;

export interface FlatMetadataFieldsProps {
  readonly tags: ReadonlyArray<string>;
  readonly classifications: ReadonlyArray<FlatQuestionClassification>;
  readonly license: FlatQuestionLicense;
  readonly language: string;
  readonly onTagsChange: (tags: ReadonlyArray<string>) => void;
  readonly onClassificationsChange: (
    classifications: ReadonlyArray<FlatQuestionClassification>,
  ) => void;
  readonly onLicenseChange: (license: FlatQuestionLicense) => void;
  readonly onLanguageChange: (language: string) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

const LICENSES: ReadonlyArray<{
  readonly kind: FlatQuestionLicense["kind"];
  readonly label: string;
}> = [
  { kind: "allRightsReserved", label: "All rights reserved" },
  { kind: "ccBy", label: "CC BY" },
  { kind: "ccBySa", label: "CC BY-SA" },
  { kind: "ccByNc", label: "CC BY-NC" },
  { kind: "cc0", label: "CC0" },
  { kind: "other", label: "Other SPDX identifier" },
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

function isLicenseKind(value: string): value is FlatQuestionLicense["kind"] {
  return LICENSES.some((license) => license.kind === value);
}

/** Metadata remains deliberate and compact: classification rows are capped before validation. */
export function FlatMetadataFields(props: FlatMetadataFieldsProps): JSX.Element {
  const classificationsError = (): string | undefined => props.fieldErrors?.["classifications"];
  const updateClassification = (
    index: number,
    patch: Partial<FlatQuestionClassification>,
  ): void => {
    props.onClassificationsChange(
      props.classifications.map((classification, classificationIndex) =>
        classificationIndex === index ? { ...classification, ...patch } : classification,
      ),
    );
  };
  const removeClassification = (index: number): void =>
    props.onClassificationsChange(
      props.classifications.filter(
        (_classification, classificationIndex) => classificationIndex !== index,
      ),
    );
  const addClassification = (): void =>
    props.onClassificationsChange([...props.classifications, { system: "", code: "", name: "" }]);
  const changeLicenseKind = (kind: FlatQuestionLicense["kind"]): void =>
    props.onLicenseChange(kind === "other" ? { kind, spdx: "" } : { kind });
  return (
    <fieldset>
      <legend>Question Library metadata</legend>
      <label class="flat-question-authoring__field">
        <span>Tags (comma-separated)</span>
        <input
          value={props.tags.join(", ")}
          disabled={props.disabled}
          onInput={(event) => props.onTagsChange(uniqueTags(event.currentTarget.value))}
        />
      </label>
      <label class="flat-question-authoring__field">
        <span>Language</span>
        <input
          value={props.language}
          disabled={props.disabled}
          onInput={(event) => props.onLanguageChange(event.currentTarget.value)}
        />
      </label>
      <label class="flat-question-authoring__field">
        <span>License</span>
        <select
          value={props.license.kind}
          disabled={props.disabled}
          onChange={(event) => {
            const kind = event.currentTarget.value;
            if (isLicenseKind(kind)) changeLicenseKind(kind);
          }}
        >
          {LICENSES.map((license) => (
            <option value={license.kind}>{license.label}</option>
          ))}
        </select>
      </label>
      <Show when={props.license.kind === "other"}>
        <label class="flat-question-authoring__field">
          <span>SPDX identifier</span>
          <input
            value={props.license.kind === "other" ? props.license.spdx : ""}
            disabled={props.disabled}
            onInput={(event) =>
              props.onLicenseChange({ kind: "other", spdx: event.currentTarget.value })
            }
          />
        </label>
      </Show>
      <section aria-labelledby="flat-classifications-heading">
        <h3 id="flat-classifications-heading">Question classification</h3>
        <p class="flat-question-authoring__help">
          Use the established classification system, code, and readable name.
        </p>
        <Show when={classificationsError() !== undefined}>
          <p class="flat-question-authoring__error" role="alert">
            {classificationsError()}
          </p>
        </Show>
        <ol class="flat-question-authoring__classification-list">
          <For each={props.classifications}>
            {(classification, index) => (
              <li class="flat-question-authoring__classification-row">
                <div class="flat-question-authoring__grid">
                  <label class="flat-question-authoring__field">
                    <span>Classification system</span>
                    <input
                      value={classification.system}
                      disabled={props.disabled}
                      onInput={(event) =>
                        updateClassification(index(), { system: event.currentTarget.value })
                      }
                    />
                  </label>
                  <label class="flat-question-authoring__field">
                    <span>Code</span>
                    <input
                      value={classification.code}
                      disabled={props.disabled}
                      onInput={(event) =>
                        updateClassification(index(), { code: event.currentTarget.value })
                      }
                    />
                  </label>
                </div>
                <label class="flat-question-authoring__field">
                  <span>Classification name</span>
                  <input
                    value={classification.name}
                    disabled={props.disabled}
                    onInput={(event) =>
                      updateClassification(index(), { name: event.currentTarget.value })
                    }
                  />
                </label>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={props.disabled}
                  onClick={() => removeClassification(index())}
                >
                  Remove classification
                </button>
              </li>
            )}
          </For>
        </ol>
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.classifications.length >= MAXIMUM_CLASSIFICATIONS}
          onClick={addClassification}
        >
          Add classification
        </button>
      </section>
    </fieldset>
  );
}
