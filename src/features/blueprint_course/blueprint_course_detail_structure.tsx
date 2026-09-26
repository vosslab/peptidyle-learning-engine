// Reusable Blueprint Course structure: save, names, assessment outline, and the open editor.
import { For, Show, type JSX } from "solid-js";
import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import {
  RecordOutlineItem,
  RecordOutlineList,
} from "../../components/record_list/record_outline_list";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { QuestionPickerSource, QuestionPickerSourceRepository } from "../question_picker";
import { BlueprintAssessmentContentEditor } from "./blueprint_assessment_content_editor";

export interface SelectedBlueprintAssessment {
  readonly moduleIndex: number;
  readonly assessmentIndex: number;
  /** The outline action that regains focus when this local editor closes. */
  readonly triggerId: string;
}

type BlueprintModule = ReplaceBlueprintCourseContentInput["modules"][number];
type BlueprintAssessment = BlueprintModule["assessments"][number];

export interface BlueprintCourseDetailStructureProps {
  readonly editing: boolean;
  readonly canEdit: boolean;
  readonly saving: boolean;
  readonly dirty: boolean;
  readonly conflict: boolean;
  readonly onSave: () => void;
  readonly onDiscard: () => void;
  readonly shortName: string;
  readonly longName: string;
  readonly savedShortName: string;
  readonly savedLongName: string;
  readonly onShortNameInput: (value: string) => void;
  readonly onLongNameInput: (value: string) => void;
  readonly metadataSaving: boolean;
  readonly metadataConflict: boolean;
  readonly onSaveNames: () => void;
  readonly onReloadMetadata: () => void;
  readonly modules: ReadonlyArray<BlueprintModule>;
  readonly courseLongName: string;
  readonly ownerEditing: boolean;
  readonly selectedAssessment: SelectedBlueprintAssessment | undefined;
  readonly assessmentContent: (
    moduleIndex: number,
    assessmentIndex: number,
  ) => BlueprintAssessment["content"] | undefined;
  readonly retainedAssessmentId: (
    moduleIndex: number,
    assessmentIndex: number,
  ) => string | undefined;
  readonly assessmentTriggerId: (moduleIndex: number, assessmentIndex: number) => string;
  readonly registerAssessmentTrigger: (triggerId: string, element: HTMLButtonElement) => void;
  readonly onSelectAssessment: (moduleIndex: number, assessmentIndex: number) => void;
  readonly onReturnToAssessmentList: () => void;
  readonly blueprintCourseId: string;
  readonly blueprintClient: BlueprintCourseClient;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onInvalidDraftChange: (invalid: boolean) => void;
  readonly onChangeAssessment: (
    moduleIndex: number,
    assessmentIndex: number,
    content: BlueprintAssessmentContentInput,
    text: string,
  ) => void;
}

/** Assessment outline and the local editor for one reusable Blueprint Course. */
export function BlueprintCourseDetailStructure(
  props: BlueprintCourseDetailStructureProps,
): JSX.Element {
  const selection = (): SelectedBlueprintAssessment | undefined => props.selectedAssessment;
  const openContent = (): BlueprintAssessment["content"] | undefined => {
    const current = selection();
    if (current === undefined) return undefined;
    return props.assessmentContent(current.moduleIndex, current.assessmentIndex);
  };
  const openModuleLabel = (): string | undefined => {
    const current = selection();
    if (current === undefined) return undefined;
    return props.modules[current.moduleIndex]?.label;
  };
  return (
    <section
      class="blueprint-course-primary-structure"
      aria-labelledby="blueprint-structure-heading"
    >
      <header class="blueprint-course-section-heading">
        <div>
          <p class="eyebrow">Reusable course structure</p>
          <h2 id="blueprint-structure-heading">
            {props.editing ? "Edit Blueprint Assessments" : "Blueprint Assessments"}
          </h2>
          <p>
            Assessments define reusable teaching settings and Questions, never Students or delivery
            dates.
          </p>
        </div>
      </header>
      <Show when={props.editing && props.canEdit}>
        <div class="blueprint-course-owner-controls">
          <aside class="blueprint-course-inspection">
            <h3>Save reusable structure</h3>
            <p>Your edits stay in this browser until Save creates the next Blueprint Revision.</p>
            <footer class="blueprint-course-save-actions blueprint-course-detail-actions">
              <button type="button" disabled={props.saving || !props.dirty} onClick={props.onSave}>
                {props.saving ? "Saving..." : "Save Blueprint Course"}
              </button>
              <Show when={props.dirty || props.conflict}>
                <button type="button" class="quiet-action" onClick={props.onDiscard}>
                  {props.conflict ? "Reload current Revision" : "Discard local changes"}
                </button>
              </Show>
            </footer>
          </aside>
          <aside class="blueprint-course-inspection">
            <h3>Blueprint Course names</h3>
            <p>Names control discovery and do not create a Blueprint Revision.</p>
            <label for="blueprint-course-detail-short-name">
              Blueprint Course short name
              <input
                id="blueprint-course-detail-short-name"
                value={props.shortName}
                maxlength="200"
                aria-describedby="blueprint-course-detail-short-name-help"
                disabled={props.metadataSaving}
                onInput={(event) => props.onShortNameInput(event.currentTarget.value)}
              />
            </label>
            <small id="blueprint-course-detail-short-name-help">
              For compact navigation; about 16 characters when practical.
            </small>
            <label>
              Blueprint Course long name
              <input
                value={props.longName}
                maxlength="200"
                disabled={props.metadataSaving}
                onInput={(event) => props.onLongNameInput(event.currentTarget.value)}
              />
            </label>
            <footer class="blueprint-course-save-actions blueprint-course-detail-actions">
              <button
                type="button"
                disabled={
                  props.metadataSaving ||
                  (props.shortName === props.savedShortName &&
                    props.longName === props.savedLongName)
                }
                onClick={props.onSaveNames}
              >
                {props.metadataSaving ? "Saving names..." : "Save Blueprint Course names"}
              </button>
              <Show when={props.metadataConflict}>
                <div class="blueprint-course-inline-actions">
                  <p class="blueprint-course-field-help" role="status">
                    Blueprint Course metadata changed elsewhere. Your typed names remain here.
                  </p>
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.metadataSaving}
                    onClick={props.onReloadMetadata}
                  >
                    Reload current metadata
                  </button>
                </div>
              </Show>
            </footer>
          </aside>
        </div>
      </Show>
      <div class="blueprint-course-editor-content">
        <Show
          when={selection() === undefined}
          fallback={
            <button type="button" class="quiet-action" onClick={props.onReturnToAssessmentList}>
              Return to assessment list
            </button>
          }
        >
          <Show when={props.editing}>
            <p>Select an assessment to edit its Questions and defaults.</p>
          </Show>
          <RecordOutlineList
            state={{ kind: "ready" }}
            isEmpty={props.modules.length === 0}
            ariaLabel="Blueprint Modules and Assessments"
            emptyState={{
              title: "No Blueprint Modules",
              message: "This Blueprint Course has no reusable Assessments.",
            }}
          >
            <For each={props.modules}>
              {(module, moduleIndex) => (
                <RecordOutlineItem recordId={`module-${moduleIndex()}`}>
                  <section class="blueprint-course-module">
                    <h3>{module.label}</h3>
                    <RecordOutlineList
                      state={{ kind: "ready" }}
                      isEmpty={module.assessments.length === 0}
                      ariaLabel={`${module.label} Assessments`}
                      emptyState={{
                        title: "No Assessments",
                        message: "This Module has no reusable Assessments.",
                      }}
                    >
                      <For each={module.assessments}>
                        {(assessment, assessmentIndex) => (
                          <RecordOutlineItem
                            recordId={`assessment-${moduleIndex()}-${assessmentIndex()}`}
                          >
                            <span>{assessment.content.title}</span>
                            <button
                              id={props.assessmentTriggerId(moduleIndex(), assessmentIndex())}
                              type="button"
                              class="quiet-action"
                              ref={(element) =>
                                props.registerAssessmentTrigger(
                                  props.assessmentTriggerId(moduleIndex(), assessmentIndex()),
                                  element,
                                )
                              }
                              onClick={() =>
                                props.onSelectAssessment(moduleIndex(), assessmentIndex())
                              }
                            >
                              {props.editing ? "Edit assessment" : "View assessment"}
                            </button>
                          </RecordOutlineItem>
                        )}
                      </For>
                    </RecordOutlineList>
                  </section>
                </RecordOutlineItem>
              )}
            </For>
          </RecordOutlineList>
        </Show>
        <Show when={selection()} keyed>
          {(currentSelection) => (
            <Show when={openContent()}>
              {(assessmentContent) => (
                <section class="blueprint-course-content-card">
                  <nav aria-label="Blueprint Assessment location">
                    <ol>
                      <li>{props.courseLongName}</li>
                      <li>{openModuleLabel()}</li>
                      <li aria-current="page">{assessmentContent().title}</li>
                    </ol>
                  </nav>
                  <BlueprintAssessmentContentEditor
                    content={assessmentContent()}
                    blueprintCourseId={props.blueprintCourseId}
                    blueprintClient={props.blueprintClient}
                    retainedAssessmentId={props.retainedAssessmentId(
                      currentSelection.moduleIndex,
                      currentSelection.assessmentIndex,
                    )}
                    editable={props.ownerEditing}
                    pickerRepository={props.pickerRepository}
                    pickerSources={props.pickerSources}
                    onInvalidDraftChange={props.onInvalidDraftChange}
                    onChange={(nextContent, text) =>
                      props.onChangeAssessment(
                        currentSelection.moduleIndex,
                        currentSelection.assessmentIndex,
                        nextContent,
                        text,
                      )
                    }
                  />
                </section>
              )}
            </Show>
          )}
        </Show>
      </div>
    </section>
  );
}
