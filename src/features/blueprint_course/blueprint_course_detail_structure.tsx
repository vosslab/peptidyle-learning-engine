// Reusable Blueprint Course structure: save, names, assessment outline, and the open editor.
import { createMemo, Show, type JSX } from "solid-js";
import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type { QuestionNativePreview } from "../../components/question_native_preview";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { QuestionPickerSource, QuestionPickerSourceRepository } from "../question_picker";
import { BlueprintAssessmentContentEditor } from "./blueprint_assessment_content_editor";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import {
  blueprintStructureAssessments,
  blueprintStructureModules,
  blueprintStructureAssessmentFields,
  blueprintStructureModuleFields,
  blueprintStructureAssessmentContent,
  blueprintStructureModuleContent,
  type BlueprintStructureAssessment,
  type BlueprintStructureModule,
} from "./blueprint_course_structure_records";

export interface SelectedBlueprintAssessment {
  readonly moduleIndex: number;
  readonly assessmentIndex: number;
  /** The record action that regains focus when this local editor closes. */
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
  readonly questionPreviews: ReadonlyMap<string, QuestionNativePreview>;
  readonly onRememberQuestionPreviews: (questions: ReadonlyArray<QuestionNativePreview>) => void;
  readonly courseLongName: string;
  readonly ownerEditing: boolean;
  readonly selectedAssessment: SelectedBlueprintAssessment | undefined;
  readonly assessmentContent: (
    moduleIndex: number,
    assessmentIndex: number,
  ) => BlueprintAssessment["content"] | undefined;
  readonly assessmentTriggerId: (moduleIndex: number, assessmentIndex: number) => string;
  readonly registerAssessmentTrigger: (triggerId: string, element: HTMLButtonElement) => void;
  readonly onSelectAssessment: (moduleIndex: number, assessmentIndex: number) => void;
  readonly onReturnToAssessmentList: () => void;
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

/** Complete Module and Assessment collections and the local editor for one Blueprint Course. */
export function BlueprintCourseDetailStructure(
  props: BlueprintCourseDetailStructureProps,
): JSX.Element {
  const selection = (): SelectedBlueprintAssessment | undefined => props.selectedAssessment;
  let assessmentOverview: HTMLElement | undefined;
  function returnToAssessmentList(): void {
    const current = selection();
    props.onReturnToAssessmentList();
    // The parent restores the original action first; edited titles can remove it from search.
    queueMicrotask(() => {
      if (current === undefined || selection() !== undefined || !assessmentOverview?.isConnected) {
        return;
      }
      if (assessmentOverview.ownerDocument.getElementById(current.triggerId) !== null) return;
      assessmentOverview.focus();
    });
  }
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
  const modules = createMemo(() => blueprintStructureModules(props.modules));
  const assessments = createMemo(() => blueprintStructureAssessments(props.modules));
  function assessmentRecordContent(row: BlueprintStructureAssessment): RecordContent {
    return blueprintStructureAssessmentContent(row, {
      id: "open_assessment",
      kind: "command",
      label: props.editing ? "Edit assessment" : "View assessment",
      onClick: () => props.onSelectAssessment(row.moduleIndex, row.assessmentIndex),
      ref: (element) => {
        const triggerId = props.assessmentTriggerId(row.moduleIndex, row.assessmentIndex);
        element.id = triggerId;
        props.registerAssessmentTrigger(triggerId, element);
      },
    });
  }
  const assessmentSearch = createLocalCompleteSearch(
    { rows: assessments },
    {
      fields: blueprintStructureAssessmentFields,
      rowId: (row) => row.id,
      keyword: (row) =>
        `${row.title} ${row.moduleContext} ${assessmentTypePresentation(row.assessmentType).label}`,
      compare: (key, left, right) =>
        key === "module" ? left.moduleIndex - right.moduleIndex : undefined,
      content: assessmentRecordContent,
    },
  );
  function moduleRecordContent(row: BlueprintStructureModule): RecordContent {
    return blueprintStructureModuleContent(row, {
      id: "view_assessments",
      kind: "command",
      label: "View Assessments",
      onClick: () => assessmentSearch.collection.query.filters?.onChange("module", [row.context]),
    });
  }
  const moduleSearch = createLocalCompleteSearch(
    { rows: modules },
    {
      fields: blueprintStructureModuleFields,
      rowId: (row) => row.id,
      keyword: (row) => row.label,
      content: moduleRecordContent,
    },
  );
  return (
    <section
      class="blueprint-course-primary-structure"
      aria-labelledby="blueprint-structure-heading"
    >
      <header class="blueprint-course-section-heading">
        <div>
          <h2 id="blueprint-structure-heading">
            {props.editing ? "Edit Blueprint Assessments" : "Blueprint Assessments"}
          </h2>
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
        <Show when={selection() !== undefined}>
          <button type="button" class="quiet-action" onClick={returnToAssessmentList}>
            Return to assessment list
          </button>
        </Show>
        <div hidden={selection() !== undefined}>
          <Show when={props.editing}>
            <p>Select an assessment to edit its Questions and defaults.</p>
          </Show>
          {/* ASVS 1.2.1: shared record fields render labels as text, including authored names. */}
          <div class="blueprint-course-structure-collections">
            <section aria-label="Blueprint Module index">
              <h3>Modules</h3>
              <SearchBlock
                layout="above"
                controls={<SearchControls state={moduleSearch.state} textLabel="Search Modules" />}
                state={moduleSearch.state}
                collection={{
                  ariaLabel: "Blueprint Modules",
                  initialMode: "compact",
                  ...moduleSearch.collection,
                  emptyState: { title: "No Modules match your search." },
                }}
              />
            </section>
            <section
              aria-label="Blueprint Assessment overview"
              tabindex="-1"
              ref={(element) => (assessmentOverview = element)}
            >
              <h3>Assessments</h3>
              <SearchBlock
                layout="above"
                controls={
                  <SearchControls state={assessmentSearch.state} textLabel="Search Assessments" />
                }
                state={assessmentSearch.state}
                collection={{
                  ariaLabel: "Blueprint Assessments",
                  initialMode: "compact",
                  ...assessmentSearch.collection,
                  emptyState: { title: "No Assessments match your search." },
                }}
              />
            </section>
          </div>
        </div>
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
                    questionPreviews={props.questionPreviews}
                    onRememberQuestionPreviews={props.onRememberQuestionPreviews}
                    blueprintClient={props.blueprintClient}
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
