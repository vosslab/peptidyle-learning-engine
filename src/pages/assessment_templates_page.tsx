// assessment_templates_page.tsx - Instructor-owned reusable Assessment settings.

import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssessmentTemplate } from "../../generated/api/AssessmentTemplate";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type {
  AssessmentTemplateClient,
  AssessmentTemplateResponse,
} from "../api/assessment_template";
import { useApplicationApi } from "../api/application_api";
import { ApiRequestError } from "../api/http_client/error";
import { UnsavedChangesGuard } from "../components/unsaved_changes_guard";
import {
  ASSESSMENT_TYPE_OPTIONS,
  assessmentTypePresentation,
  isAssessmentType,
} from "../assessment_type_presentation";
import { AssessmentTemplateSettingsEditor } from "./assessment_template_settings_editor";
import {
  assessmentTemplateDraft,
  assessmentTemplateNameError,
  assessmentTemplateSettings,
  assessmentTypeHasOneAttempt,
  type AssessmentTemplateDraft,
  type AssessmentTemplateDraftPatch,
} from "./assessment_template_settings_model";
import "./assessment_templates_page.css";

export interface AssessmentTemplatesSurfaceProps {
  readonly client: AssessmentTemplateClient;
}

function replaceTemplate(
  templates: ReadonlyArray<AssessmentTemplate>,
  replacement: AssessmentTemplate,
): ReadonlyArray<AssessmentTemplate> {
  const index = templates.findIndex((template) => template.id === replacement.id);
  if (index < 0) return [replacement, ...templates];
  return templates.map((template) => (template.id === replacement.id ? replacement : template));
}

function isConflict(error: unknown): boolean {
  return error instanceof ApiRequestError && error.status === 409;
}

/** Actual Template surface, injectable for one-time actual-component browser proof. */
export function AssessmentTemplatesSurface(props: AssessmentTemplatesSurfaceProps): JSX.Element {
  const [templates, setTemplates] = createSignal<ReadonlyArray<AssessmentTemplate>>([]);
  const [listState, setListState] = createSignal<"loading" | "ready" | "error">("loading");
  const [selected, setSelected] = createSignal<AssessmentTemplateResponse>();
  const [draft, setDraft] = createSignal<AssessmentTemplateDraft>();
  const [detailBusy, setDetailBusy] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
  const [replacementRequested, setReplacementRequested] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [status, setStatus] = createSignal("");
  const [statusIsError, setStatusIsError] = createSignal(false);
  const [createName, setCreateName] = createSignal("");
  const [createType, setCreateType] = createSignal<AssessmentType>("regular_assignment");
  const [creating, setCreating] = createSignal(false);
  const [createDisclosure, setCreateDisclosure] = createSignal<boolean>();
  let pendingReplacement: (() => void) | undefined;

  const isCreateExpanded = createMemo(
    () => createDisclosure() ?? (listState() === "ready" && templates().length === 0),
  );

  async function loadTemplates(): Promise<void> {
    setListState("loading");
    try {
      setTemplates(await props.client.listAssessmentTemplates());
      setListState("ready");
    } catch {
      setListState("error");
    }
  }

  function openResponse(response: AssessmentTemplateResponse): void {
    setSelected(response);
    setDraft(assessmentTemplateDraft(response.template));
    setDirty(false);
    setConflict(false);
  }

  function requestReplacement(action: () => void): void {
    if (!dirty()) {
      action();
      return;
    }
    pendingReplacement = action;
    setReplacementRequested(true);
  }

  function continueReplacement(): void {
    const action = pendingReplacement;
    pendingReplacement = undefined;
    action?.();
  }

  function openTemplate(template: AssessmentTemplate): void {
    if (template.id === selected()?.template.id || detailBusy() || creating()) return;
    requestReplacement(() => void loadTemplate(template));
  }

  async function loadTemplate(template: AssessmentTemplate): Promise<void> {
    setDetailBusy(true);
    setStatus("");
    try {
      openResponse(await props.client.getAssessmentTemplate(template.id));
    } catch {
      setStatusIsError(true);
      setStatus("This Template could not be loaded. Choose it again to retry.");
    } finally {
      setDetailBusy(false);
    }
  }

  function createTemplate(): void {
    const nameError = assessmentTemplateNameError(createName());
    if (nameError !== undefined) {
      setStatusIsError(true);
      setStatus(nameError);
      return;
    }
    if (detailBusy() || creating()) return;
    requestReplacement(() => void performCreateTemplate());
  }

  async function performCreateTemplate(): Promise<void> {
    setCreating(true);
    setStatus("");
    try {
      const response = await props.client.createAssessmentTemplate({
        name: createName(),
        assessmentType: createType(),
      });
      setTemplates((current) => replaceTemplate(current, response.template));
      setCreateName("");
      setCreateDisclosure(false);
      openResponse(response);
      setStatusIsError(false);
      setStatus("Template created with the canonical settings for its Assessment Type.");
    } catch {
      setStatusIsError(true);
      setStatus("The Template could not be created. Review its name and try again.");
    } finally {
      setCreating(false);
    }
  }

  function patchDraft(patch: AssessmentTemplateDraftPatch): void {
    setDraft((current) => (current === undefined ? current : { ...current, ...patch }));
    setDirty(true);
    setStatus("");
  }

  async function saveTemplate(): Promise<boolean> {
    const response = selected();
    const currentDraft = draft();
    if (response === undefined || currentDraft === undefined) return false;
    const nameError = assessmentTemplateNameError(currentDraft.name);
    const settingsResult = assessmentTemplateSettings(currentDraft);
    const error = nameError ?? settingsResult.error;
    if (error !== undefined || settingsResult.settings === undefined) {
      setStatusIsError(true);
      setStatus(error ?? "Review the Template settings.");
      return false;
    }
    setDetailBusy(true);
    setStatus("");
    try {
      const saved = await props.client.saveAssessmentTemplate(
        response.template.id,
        {
          name: currentDraft.name,
          assessmentType: currentDraft.assessmentType,
          settings: settingsResult.settings,
        },
        response.template.assessmentTemplateEditNumber,
      );
      setTemplates((current) => replaceTemplate(current, saved.template));
      openResponse(saved);
      setStatusIsError(false);
      setStatus("Template saved.");
      return true;
    } catch (error: unknown) {
      setStatusIsError(true);
      if (isConflict(error)) {
        setConflict(true);
        setStatus(
          "This Template changed elsewhere. Reload the latest version while keeping your typed changes, or use the server version.",
        );
      } else {
        setStatus("The Template was not saved. Your changes remain here; try again.");
      }
      return false;
    } finally {
      setDetailBusy(false);
    }
  }

  async function reloadAfterConflict(keepDraft: boolean): Promise<void> {
    const response = selected();
    if (response === undefined) return;
    setDetailBusy(true);
    try {
      const latest = await props.client.getAssessmentTemplate(response.template.id);
      setSelected(latest);
      setTemplates((current) => replaceTemplate(current, latest.template));
      if (!keepDraft) {
        setDraft(assessmentTemplateDraft(latest.template));
        setDirty(false);
      }
      setConflict(false);
      setStatusIsError(false);
      setStatus(
        keepDraft
          ? "Latest version loaded. Your typed changes remain and can now be saved again."
          : "Latest server version loaded. Your previous typed changes were cleared.",
      );
    } catch {
      setStatusIsError(true);
      setStatus("The latest Template could not be loaded. Your typed changes remain here.");
    } finally {
      setDetailBusy(false);
    }
  }

  onMount(() => void loadTemplates());

  return (
    <section class="page assessment-templates" data-route-surface="assessmentTemplates">
      <header class="assessment-templates-header">
        <p class="eyebrow">Assessments</p>
        <h1>My Assessment Templates</h1>
        <p class="page-lede">
          Reuse your own Assessment settings. Templates do not contain Questions, Question Pools,
          points, or Course dates.
        </p>
      </header>

      <Show when={status() !== ""}>
        <p
          class="assessment-template-status"
          role={statusIsError() ? "alert" : "status"}
          aria-live="polite"
        >
          {status()}
        </p>
      </Show>

      <div class="assessment-templates-layout">
        <aside class="assessment-template-overview" aria-labelledby="template-overview-heading">
          <h2 id="template-overview-heading">Your Templates</h2>
          <Show when={listState() === "loading"}>
            <p class="assessment-template-list-state" role="status">
              Loading your Templates...
            </p>
          </Show>
          <Show when={listState() === "error"}>
            <div class="assessment-template-list-state" role="alert">
              <p>Your Templates could not be loaded.</p>
              <button type="button" onClick={() => void loadTemplates()}>
                Try again
              </button>
            </div>
          </Show>
          <Show when={listState() === "ready" && templates().length === 0}>
            <div class="empty-state assessment-template-list-state">
              <h3>No Templates yet</h3>
              <p>Create one below to save settings you use often.</p>
            </div>
          </Show>
          <Show when={listState() === "ready" && templates().length > 0}>
            <ul class="assessment-template-list" aria-label="My Assessment Templates">
              <For each={templates()}>
                {(template) => {
                  const presentation = assessmentTypePresentation(template.assessmentType);
                  return (
                    <li>
                      <button
                        type="button"
                        class="assessment-template-list-button"
                        classList={{
                          "assessment-template-list-button--selected":
                            selected()?.template.id === template.id,
                        }}
                        aria-pressed={selected()?.template.id === template.id}
                        disabled={detailBusy() || creating()}
                        onClick={() => openTemplate(template)}
                      >
                        <strong>{template.name}</strong>
                        <span>{presentation.label}</span>
                      </button>
                    </li>
                  );
                }}
              </For>
            </ul>
          </Show>

          <button
            class="quiet-action assessment-template-create-disclosure"
            type="button"
            aria-expanded={isCreateExpanded()}
            aria-controls="create-assessment-template"
            disabled={creating() || detailBusy()}
            onClick={() => setCreateDisclosure(!isCreateExpanded())}
          >
            <span aria-hidden="true">{isCreateExpanded() ? "\u25be" : "\u25b8"}</span>
            Create a Template
          </button>
          <form
            id="create-assessment-template"
            class="assessment-template-create"
            hidden={!isCreateExpanded()}
            aria-busy={creating()}
            onSubmit={(event) => {
              event.preventDefault();
              createTemplate();
            }}
          >
            <label class="assessment-template-field">
              Template name
              <input
                maxlength="200"
                value={createName()}
                disabled={creating() || detailBusy()}
                onInput={(event) => setCreateName(event.currentTarget.value)}
              />
            </label>
            <label class="assessment-template-field">
              Assessment Type
              <select
                value={createType()}
                disabled={creating() || detailBusy()}
                onChange={(event) => {
                  if (isAssessmentType(event.currentTarget.value)) {
                    setCreateType(event.currentTarget.value);
                  }
                }}
              >
                <For each={ASSESSMENT_TYPE_OPTIONS}>
                  {(option) => <option value={option.value}>{option.label}</option>}
                </For>
              </select>
            </label>
            <button class="primary-action" type="submit" disabled={creating() || detailBusy()}>
              {creating() ? "Creating Template..." : "Create Template"}
            </button>
          </form>
        </aside>

        <section class="assessment-template-editor" aria-labelledby="template-editor-heading">
          <Show
            when={draft()}
            fallback={
              <div class="empty-state assessment-template-editor-empty">
                <h2 id="template-editor-heading">Choose a Template</h2>
                <p>Select one from the overview, or create a new Template.</p>
              </div>
            }
          >
            {(currentDraft) => (
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  void saveTemplate();
                }}
              >
                <header class="assessment-template-editor-header">
                  <div>
                    <p class="eyebrow">Focused editor</p>
                    <h2 id="template-editor-heading">Edit Template</h2>
                  </div>
                  <button
                    class="primary-action"
                    type="submit"
                    disabled={detailBusy() || creating()}
                  >
                    {detailBusy() ? "Working..." : "Save Template"}
                  </button>
                </header>
                <div class="assessment-template-identity">
                  <label class="assessment-template-field">
                    Template name
                    <input
                      maxlength="200"
                      value={currentDraft().name}
                      disabled={detailBusy() || creating()}
                      onInput={(event) => patchDraft({ name: event.currentTarget.value })}
                    />
                  </label>
                  <label class="assessment-template-field">
                    Assessment Type
                    <select
                      value={currentDraft().assessmentType}
                      disabled={detailBusy() || creating()}
                      onChange={(event) => {
                        if (isAssessmentType(event.currentTarget.value)) {
                          const assessmentType = event.currentTarget.value;
                          patchDraft({
                            assessmentType,
                            ...(assessmentTypeHasOneAttempt(assessmentType)
                              ? { attemptLimit: "1" }
                              : {}),
                          });
                        }
                      }}
                    >
                      <For each={ASSESSMENT_TYPE_OPTIONS}>
                        {(option) => <option value={option.value}>{option.label}</option>}
                      </For>
                    </select>
                    <small>
                      Quiz and Exam use one Assessment Attempt. Other settings stay unchanged.
                    </small>
                  </label>
                </div>
                <AssessmentTemplateSettingsEditor
                  draft={currentDraft()}
                  disabled={detailBusy() || creating()}
                  onPatch={patchDraft}
                />
                <div class="assessment-template-actions">
                  <button
                    class="primary-action"
                    type="submit"
                    disabled={detailBusy() || creating()}
                  >
                    {detailBusy() ? "Working..." : "Save Template"}
                  </button>
                  <Show when={conflict()}>
                    <button
                      type="button"
                      disabled={detailBusy()}
                      onClick={() => void reloadAfterConflict(true)}
                    >
                      Reload latest and keep my changes
                    </button>
                    <button
                      type="button"
                      disabled={detailBusy()}
                      onClick={() => void reloadAfterConflict(false)}
                    >
                      Use latest server version
                    </button>
                  </Show>
                </div>
              </form>
            )}
          </Show>
        </section>
      </div>
      <UnsavedChangesGuard
        dirty={dirty}
        save={saveTemplate}
        manualLeave={{
          requested: replacementRequested,
          clearRequest: () => setReplacementRequested(false),
          continue: continueReplacement,
        }}
        copy={{
          heading: "Save Template changes?",
          description: "Your changes to this Assessment Template have not been saved.",
          saveActionLabel: "Save and continue",
          savingActionLabel: "Saving Template...",
          saveFailureMessage:
            "Template changes were not saved. Resolve the page error, then try again or stay here.",
        }}
      />
    </section>
  );
}

/** Production route consumes only the app-owned same-origin client. */
export function AssessmentTemplatesPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  return <AssessmentTemplatesSurface client={applicationApi.client} />;
}
