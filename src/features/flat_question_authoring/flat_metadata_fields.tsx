// flat_metadata_fields.tsx - bounded metadata controls for a flat-question draft.

import { For, Show, type JSX } from "solid-js";

import type { FlatQuestionLicense, FlatQuestionTaxonomyTerm } from "./flat_question_source";

const MAXIMUM_TAXONOMY_TERMS = 32;

export interface FlatMetadataFieldsProps {
  readonly tags: ReadonlyArray<string>;
  readonly taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>;
  readonly license: FlatQuestionLicense;
  readonly language: string;
  readonly onTagsChange: (tags: ReadonlyArray<string>) => void;
  readonly onTaxonomyChange: (taxonomy: ReadonlyArray<FlatQuestionTaxonomyTerm>) => void;
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

/** Metadata remains deliberate and compact: taxonomy rows are capped before schema validation. */
export function FlatMetadataFields(props: FlatMetadataFieldsProps): JSX.Element {
  const taxonomyError = (): string | undefined => props.fieldErrors?.["taxonomy"];
  const updateTerm = (index: number, patch: Partial<FlatQuestionTaxonomyTerm>): void => {
    props.onTaxonomyChange(
      props.taxonomy.map((term, termIndex) => (termIndex === index ? { ...term, ...patch } : term)),
    );
  };
  const removeTerm = (index: number): void =>
    props.onTaxonomyChange(props.taxonomy.filter((_term, termIndex) => termIndex !== index));
  const addTerm = (): void =>
    props.onTaxonomyChange([...props.taxonomy, { scheme: "", code: "", label: "" }]);
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
      <section aria-labelledby="flat-taxonomy-heading">
        <h3 id="flat-taxonomy-heading">Taxonomy</h3>
        <p class="flat-question-authoring__help">
          Use an established scheme, code, and student-readable label.
        </p>
        <Show when={taxonomyError() !== undefined}>
          <p class="flat-question-authoring__error" role="alert">
            {taxonomyError()}
          </p>
        </Show>
        <ol class="flat-question-authoring__taxonomy-list">
          <For each={props.taxonomy}>
            {(term, index) => (
              <li class="flat-question-authoring__taxonomy-row">
                <div class="flat-question-authoring__grid">
                  <label class="flat-question-authoring__field">
                    <span>Scheme</span>
                    <input
                      value={term.scheme}
                      disabled={props.disabled}
                      onInput={(event) =>
                        updateTerm(index(), { scheme: event.currentTarget.value })
                      }
                    />
                  </label>
                  <label class="flat-question-authoring__field">
                    <span>Code</span>
                    <input
                      value={term.code}
                      disabled={props.disabled}
                      onInput={(event) => updateTerm(index(), { code: event.currentTarget.value })}
                    />
                  </label>
                </div>
                <label class="flat-question-authoring__field">
                  <span>Label</span>
                  <input
                    value={term.label}
                    disabled={props.disabled}
                    onInput={(event) => updateTerm(index(), { label: event.currentTarget.value })}
                  />
                </label>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={props.disabled}
                  onClick={() => removeTerm(index())}
                >
                  Remove taxonomy term
                </button>
              </li>
            )}
          </For>
        </ol>
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.taxonomy.length >= MAXIMUM_TAXONOMY_TERMS}
          onClick={addTerm}
        >
          Add taxonomy term
        </button>
      </section>
    </fieldset>
  );
}
