// Explicit Revision Save editing for reusable Blueprint Courses.
import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createSignal, onMount, type JSX } from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import { UnsavedChangesGuard } from "../../components/unsaved_changes_guard";
import { CourseClassificationEditor } from "../../components/course_classification_editor";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import { parseBlueprintCourseReference } from "../../navigation/public_route";
import type {
  BlueprintCourseClient,
  BlueprintMetadataEtag,
  BlueprintRevisionEtag,
} from "../../api/blueprint_course";
import { BlueprintAssessmentContentEditor } from "./blueprint_assessment_content_editor";
import { BlueprintCourseLifecycleControls } from "./blueprint_course_lifecycle_controls";
import { BlueprintHistory } from "./blueprint_history";
import { BlueprintForkSource, BlueprintKnownForks } from "../blueprint_forks/blueprint_fork_review";
import { BlueprintForkCreate } from "../blueprint_forks/blueprint_fork_create";
import { ProposalTargetTools } from "../blueprint_change_proposal/proposal_workspace";
import {
  blueprintLifecyclePresentation,
  replacementContentFromBlueprintModules,
  validateReusableContent,
} from "./blueprint_course_model";
import type { BlueprintCourseDetailWorkspaceProps } from "./blueprint_course_workspace_types";
import "./blueprint_course.css";

type LoadState = "loading" | "ready" | "error";
type NoticeKind = "status" | "alert";

interface Notice {
  readonly kind: NoticeKind;
  readonly text: string;
}

interface LoadedBlueprintCourse {
  readonly view: BlueprintCourseView;
  /** The exact Revision and ETag on which this local editor state is based. */
  readonly revisionEtag: BlueprintRevisionEtag;
  /** Opaque validator for current lineage metadata, including classification. */
  readonly metadataEtag: BlueprintMetadataEtag;
  readonly content: ReplaceBlueprintCourseContentInput;
  readonly savedContent: ReplaceBlueprintCourseContentInput;
}

function metadataEtagForView(value: string): BlueprintMetadataEtag {
  return `"${value}"`;
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.entries(value)
      .sort(([first], [second]) => first.localeCompare(second))
      .map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value) ?? "null";
}

/** Local no-op detection ignores object-property construction order while retaining authored array order. */
function sameContent(
  first: ReplaceBlueprintCourseContentInput,
  second: ReplaceBlueprintCourseContentInput,
): boolean {
  return canonicalJson(first) === canonicalJson(second);
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof BlueprintCourseConflictError) {
    return "Blueprint Course content changed in another editor. Reload before saving.";
  }
  if (error instanceof ApiRequestError) {
    if (error.status === 401)
      return "Your session ended. Sign in again, then return to this Blueprint Course.";
    if (error.status === 403 || error.status === 404)
      return "This Blueprint Course is unavailable for your Account.";
  }
  return error instanceof Error && error.message.length > 0 ? error.message : fallback;
}

export function BlueprintCourseDetailWorkspace(
  props: BlueprintCourseDetailWorkspaceProps,
): JSX.Element {
  const [editing, setEditing] = createSignal(false);
  const [selectedAssessment, setSelectedAssessment] = createSignal<{
    readonly moduleIndex: number;
    readonly assessmentIndex: number;
  }>();
  const [state, setState] = createSignal<LoadState>("loading");
  const [current, setCurrent] = createSignal<LoadedBlueprintCourse>();
  const [notice, setNotice] = createSignal<Notice>({
    kind: "status",
    text: "Loading Blueprint Course.",
  });
  const [saving, setSaving] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [metadataSaving, setMetadataSaving] = createSignal(false);
  const [metadataConflict, setMetadataConflict] = createSignal(false);
  const [shortName, setShortName] = createSignal("");
  const [longName, setLongName] = createSignal("");
  const [archiveConfirmation, setArchiveConfirmation] = createSignal("");
  const [invalidDraft, setInvalidDraft] = createSignal(false);
  const [refreshFailed, setRefreshFailed] = createSignal(false);
  const dirty = (): boolean => {
    const loaded = current();
    return (
      invalidDraft() || (loaded !== undefined && !sameContent(loaded.content, loaded.savedContent))
    );
  };
  const hasUnsavedForkChanges = (): boolean => {
    const loaded = current();
    return (
      dirty() ||
      (loaded !== undefined &&
        (shortName() !== loaded.view.short_name || longName() !== loaded.view.long_name))
    );
  };
  async function load(keepLocalContent: boolean, keepVisible = false): Promise<void> {
    if (parseBlueprintCourseReference(props.blueprintCourseRef) === null) {
      setState("error");
      setNotice({ kind: "alert", text: "This Blueprint Course reference is invalid." });
      return;
    }
    setRefreshFailed(false);
    if (!keepVisible) setState("loading");
    try {
      const result = await props.client.getBlueprintCourse(props.blueprintCourseRef);
      const prior = current();
      const savedContent = replacementContentFromBlueprintModules(result.blueprintCourse.modules);
      const preserveContent = keepLocalContent || (keepVisible && dirty());
      const preserveNames =
        preserveContent ||
        (keepVisible &&
          prior !== undefined &&
          (shortName() !== prior.view.short_name || longName() !== prior.view.long_name));
      const content = preserveContent && prior !== undefined ? prior.content : savedContent;
      setCurrent({
        view: result.blueprintCourse,
        revisionEtag: result.revisionEtag,
        metadataEtag: metadataEtagForView(result.blueprintCourse.metadata_etag),
        content,
        savedContent,
      });
      if (!preserveNames || prior === undefined) {
        setShortName(result.blueprintCourse.short_name);
        setLongName(result.blueprintCourse.long_name);
      }
      if (!keepLocalContent) setConflict(false);
      setState("ready");
      setNotice({
        kind: "status",
        text:
          result.blueprintCourse.read_access === "blueprint_course_owner"
            ? "Blueprint Course loaded. Save creates one immutable Blueprint Revision."
            : "Blueprint Course loaded. Inspect its answer-free reusable structure.",
      });
    } catch (error: unknown) {
      if (keepVisible && current() !== undefined) setRefreshFailed(true);
      else setState("error");
      setNotice({
        kind: "alert",
        text: keepVisible
          ? `The Blueprint Course refresh failed. Your local edits remain available. ${errorMessage(error, "Try again.")}`
          : errorMessage(error, "This Blueprint Course could not load. Try again."),
      });
    }
  }
  function changeContent(next: ReplaceBlueprintCourseContentInput, text: string): void {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canEdit
    )
      return;
    setCurrent({ ...loaded, content: next });
    setNotice({ kind: "status", text });
  }
  function changeAssessment(
    moduleIndex: number,
    assessmentIndex: number,
    content: import("../../../generated/api/BlueprintAssessmentContentInput").BlueprintAssessmentContentInput,
    text: string,
  ): void {
    const loaded = current();
    const module = loaded?.content.modules[moduleIndex];
    const assessment = module?.assessments[assessmentIndex];
    if (loaded === undefined || module === undefined || assessment === undefined) return;
    const modules = [...loaded.content.modules];
    const assessments = [...module.assessments];
    assessments[assessmentIndex] = { ...assessment, content };
    modules[moduleIndex] = { ...module, assessments };
    changeContent({ ...loaded.content, modules }, text);
  }

  async function save(): Promise<boolean> {
    const loaded = current();
    if (invalidDraft()) {
      setNotice({ kind: "alert", text: "Correct the invalid numeric draft before saving." });
      return false;
    }
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canEdit
    )
      return false;
    for (const module of loaded.content.modules) {
      for (const assessment of module.assessments) {
        const validation = validateReusableContent(assessment.content);
        if (!validation.valid) {
          setNotice({
            kind: "alert",
            text: validation.message ?? "Review this Blueprint Course before saving.",
          });
          return false;
        }
      }
    }
    setSaving(true);
    try {
      const saved = await props.client.saveBlueprintCourse(
        loaded.view.reference,
        loaded.content,
        loaded.revisionEtag,
        crypto.randomUUID(),
      );
      const savedContent = replacementContentFromBlueprintModules(saved.blueprintCourse.modules);
      setCurrent({
        view: saved.blueprintCourse,
        revisionEtag: saved.revisionEtag,
        metadataEtag: metadataEtagForView(saved.blueprintCourse.metadata_etag),
        content: savedContent,
        savedContent,
      });
      setConflict(false);
      setNotice({
        kind: "status",
        text: saved.changed
          ? `Saved Blueprint Revision ${saved.blueprintCourse.current_revision.revision}.`
          : `No content changed. Blueprint Revision ${saved.blueprintCourse.current_revision.revision} remains current.`,
      });
      return true;
    } catch (error: unknown) {
      setConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "Blueprint Course could not save. Your local changes remain available.",
        ),
      });
      return false;
    } finally {
      setSaving(false);
    }
  }

  function applyMetadata(
    metadata: Awaited<ReturnType<BlueprintCourseClient["renameBlueprintCourse"]>>,
  ): void {
    if (current() === undefined) return;
    applyMetadataState(metadata);
    setShortName(metadata.metadata.short_name);
    setLongName(metadata.metadata.long_name);
  }

  function applyMetadataState(
    metadata: Awaited<ReturnType<BlueprintCourseClient["renameBlueprintCourse"]>>,
  ): void {
    const loaded = current();
    if (loaded === undefined) return;
    setCurrent({
      ...loaded,
      metadataEtag: metadata.metadataEtag,
      view: {
        ...loaded.view,
        short_name: metadata.metadata.short_name,
        long_name: metadata.metadata.long_name,
        availability: metadata.metadata.availability,
        classification: metadata.metadata.classification,
        metadata_etag: metadata.metadata.metadata_etag,
      },
    });
    if (metadata.metadata.availability !== "private") setEditing(false);
    setMetadataConflict(false);
  }

  function validNames(): boolean {
    if (
      shortName().trim() !== shortName() ||
      shortName().length === 0 ||
      longName().trim() !== longName() ||
      longName().length === 0
    ) {
      setNotice({ kind: "alert", text: "Enter trimmed Blueprint Course short and long names." });
      return false;
    }
    return true;
  }

  async function saveNames(): Promise<void> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canEdit ||
      !validNames()
    ) {
      return;
    }
    setMetadataSaving(true);
    try {
      const metadata = await props.client.renameBlueprintCourse(
        loaded.view.reference,
        { short_name: shortName(), long_name: longName() },
        loaded.metadataEtag,
      );
      applyMetadata(metadata);
      setNotice({
        kind: "status",
        text: "Blueprint Course names saved. Revision content is unchanged.",
      });
    } catch (error: unknown) {
      setMetadataConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text:
          error instanceof BlueprintCourseConflictError
            ? "Blueprint Course metadata changed elsewhere. Your typed names remain available."
            : errorMessage(
                error,
                "Blueprint Course names could not save. Your typed names remain available.",
              ),
      });
    } finally {
      setMetadataSaving(false);
    }
  }

  async function archive(): Promise<void> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canArchive
    )
      return;
    if (archiveConfirmation() !== loaded.view.long_name) {
      setNotice({
        kind: "alert",
        text: "Enter the current Blueprint Course long name to archive it.",
      });
      return;
    }
    setMetadataSaving(true);
    try {
      const metadata = await props.client.archiveBlueprintCourse(
        loaded.view.reference,
        archiveConfirmation(),
        loaded.metadataEtag,
      );
      applyMetadata(metadata);
      setArchiveConfirmation("");
      setNotice({
        kind: "status",
        text: "Blueprint Course archived. Its saved Revisions are unchanged.",
      });
    } catch (error: unknown) {
      setMetadataConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text:
          error instanceof BlueprintCourseConflictError
            ? "Blueprint Course metadata changed elsewhere. Your confirmation remains available."
            : errorMessage(
                error,
                "Blueprint Course could not archive. Your confirmation remains available.",
              ),
      });
    } finally {
      setMetadataSaving(false);
    }
  }

  async function restore(): Promise<void> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canRestore
    )
      return;
    setMetadataSaving(true);
    try {
      const metadata = await props.client.restoreBlueprintCourse(
        loaded.view.reference,
        loaded.metadataEtag,
      );
      applyMetadata(metadata);
      setNotice({
        kind: "status",
        text: "Blueprint Course restored. Its saved Revisions are unchanged.",
      });
    } catch (error: unknown) {
      setMetadataConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text:
          error instanceof BlueprintCourseConflictError
            ? "Blueprint Course metadata changed elsewhere. Reload before restoring."
            : errorMessage(error, "Blueprint Course could not restore."),
      });
    } finally {
      setMetadataSaving(false);
    }
  }

  async function publish(): Promise<void> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canPublish
    )
      return;
    if (dirty()) {
      setNotice({
        kind: "alert",
        text: "Save your Private Blueprint Course edits before publishing its current Revision.",
      });
      return;
    }
    setMetadataSaving(true);
    try {
      applyMetadata(
        await props.client.publishBlueprintCourse(loaded.view.reference, loaded.metadataEtag),
      );
      setNotice({
        kind: "status",
        text: "Blueprint Course published. Instructors can now browse and adopt its current Revision.",
      });
    } catch (error: unknown) {
      setMetadataConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text: errorMessage(error, "Blueprint Course could not publish."),
      });
    } finally {
      setMetadataSaving(false);
    }
  }

  async function returnToPrivate(): Promise<void> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access)
        .canReturnToPrivate
    )
      return;
    setMetadataSaving(true);
    try {
      applyMetadata(
        await props.client.returnBlueprintCourseToPrivate(
          loaded.view.reference,
          loaded.metadataEtag,
        ),
      );
      setNotice({ kind: "status", text: "Blueprint Course returned to Private." });
    } catch (error: unknown) {
      setMetadataConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "Blueprint Course could not return to Private. It may already have been adopted.",
        ),
      });
    } finally {
      setMetadataSaving(false);
    }
  }

  async function reloadMetadata(): Promise<void> {
    const loaded = current();
    if (loaded === undefined) return;
    setMetadataSaving(true);
    try {
      const result = await props.client.getBlueprintCourse(loaded.view.reference);
      const prior = current();
      if (prior === undefined || prior.view.reference !== result.blueprintCourse.reference) return;
      setCurrent({
        ...prior,
        metadataEtag: metadataEtagForView(result.blueprintCourse.metadata_etag),
        view: {
          ...prior.view,
          short_name: result.blueprintCourse.short_name,
          long_name: result.blueprintCourse.long_name,
          availability: result.blueprintCourse.availability,
          classification: result.blueprintCourse.classification,
          metadata_etag: result.blueprintCourse.metadata_etag,
        },
      });
      setMetadataConflict(false);
      setNotice({
        kind: "status",
        text: "Current Blueprint Course metadata loaded. Your typed names remain available.",
      });
    } catch (error: unknown) {
      setNotice({
        kind: "alert",
        text: errorMessage(error, "Blueprint Course metadata could not load."),
      });
    } finally {
      setMetadataSaving(false);
    }
  }

  onMount(() => void load(false));
  return (
    <section class="page blueprint-course-workspace" data-route-surface="blueprintCourseDetail">
      <A class="quiet-link" href="/blueprint-courses">
        Return to Blueprint Courses
      </A>
      <p class="blueprint-course-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <Show when={refreshFailed()}>
        <button type="button" onClick={() => void load(hasUnsavedForkChanges(), true)}>
          Retry refreshing Blueprint Course
        </button>
      </Show>
      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading Blueprint Course.</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => void load(false)}>
            Retry loading Blueprint Course
          </button>
        </Match>
        <Match when={state() === "ready" && current()}>
          {(loaded) => (
            <section class="blueprint-course-detail-editor">
              <header class="blueprint-course-page-heading">
                <p class="eyebrow">Blueprint Course</p>
                <h1>{loaded().view.long_name}</h1>
                <p class="page-lede">
                  Reusable course structure without Students, deadlines, or course delivery
                  settings. Current Revision {loaded().view.current_revision.revision}.
                </p>
              </header>
              <CourseClassificationEditor
                value={loaded().view.classification}
                metadataEtag={loaded().metadataEtag}
                canEdit={
                  blueprintLifecyclePresentation(
                    loaded().view.availability,
                    loaded().view.read_access,
                  ).canEdit && !metadataSaving()
                }
                save={async (classification, etag) => {
                  const transition = await props.client.updateBlueprintCourseClassification(
                    loaded().view.reference,
                    classification,
                    etag,
                  );
                  // Classification saves update metadata without consuming unrelated name drafts.
                  applyMetadataState(transition);
                }}
                reload={async () => {
                  const latest = await props.client.getBlueprintCourse(loaded().view.reference);
                  const prior = current();
                  if (prior !== undefined)
                    setCurrent({
                      ...prior,
                      metadataEtag: metadataEtagForView(latest.blueprintCourse.metadata_etag),
                      view: {
                        ...prior.view,
                        short_name: latest.blueprintCourse.short_name,
                        long_name: latest.blueprintCourse.long_name,
                        availability: latest.blueprintCourse.availability,
                        classification: latest.blueprintCourse.classification,
                        metadata_etag: latest.blueprintCourse.metadata_etag,
                      },
                    });
                  return {
                    classification: latest.blueprintCourse.classification,
                    metadataEtag: metadataEtagForView(latest.blueprintCourse.metadata_etag),
                  };
                }}
              />
              <BlueprintHistory client={props.client} view={loaded().view} />
              <BlueprintForkCreate client={props.client} source={loaded().view} />
              <Show when={props.proposalClient}>
                {(client) => <ProposalTargetTools client={client()} target={loaded().view} />}
              </Show>
              <BlueprintForkSource
                client={props.client}
                view={loaded().view}
                hasUnsavedChanges={hasUnsavedForkChanges()}
                onApplied={() => void load(hasUnsavedForkChanges(), true)}
              />
              <BlueprintKnownForks
                client={props.client}
                reference={loaded().view.reference}
                sourceCurrentRevision={loaded().view.current_revision.revision}
                hasUnsavedChanges={hasUnsavedForkChanges()}
                onApplied={() => void load(hasUnsavedForkChanges(), true)}
              />
              <BlueprintCourseLifecycleControls
                view={loaded().view}
                editing={editing()}
                metadataSaving={metadataSaving()}
                hasUnsavedContent={dirty()}
                archiveConfirmation={archiveConfirmation()}
                onArchiveConfirmationInput={setArchiveConfirmation}
                onToggleEditor={() => {
                  setEditing(!editing());
                  setSelectedAssessment(undefined);
                }}
                onPublish={() => void publish()}
                onReturnToPrivate={() => void returnToPrivate()}
                onArchive={() => void archive()}
                onRestore={() => void restore()}
              />
              <Show
                when={
                  editing() &&
                  blueprintLifecyclePresentation(
                    loaded().view.availability,
                    loaded().view.read_access,
                  ).canEdit
                }
              >
                <div class="blueprint-course-owner-controls">
                  <aside class="blueprint-course-inspection">
                    <h2>Save reusable structure</h2>
                    <p>
                      Your edits stay in this browser until Save creates the next Blueprint
                      Revision.
                    </p>
                    <footer class="blueprint-course-save-actions blueprint-course-detail-actions">
                      <button
                        type="button"
                        disabled={saving() || !dirty()}
                        onClick={() => void save()}
                      >
                        {saving() ? "Saving..." : "Save Blueprint Course"}
                      </button>
                      <Show when={dirty() || conflict()}>
                        <button type="button" class="quiet-action" onClick={() => void load(false)}>
                          {conflict() ? "Reload current Revision" : "Discard local changes"}
                        </button>
                      </Show>
                    </footer>
                  </aside>
                  <aside class="blueprint-course-inspection">
                    <h2>Blueprint Course names</h2>
                    <p>Names control discovery and do not create a Blueprint Revision.</p>
                    <label>
                      Blueprint Course short name
                      <input
                        value={shortName()}
                        maxlength="200"
                        disabled={metadataSaving()}
                        onInput={(event) => setShortName(event.currentTarget.value)}
                      />
                    </label>
                    <label>
                      Blueprint Course long name
                      <input
                        value={longName()}
                        maxlength="200"
                        disabled={metadataSaving()}
                        onInput={(event) => setLongName(event.currentTarget.value)}
                      />
                    </label>
                    <footer class="blueprint-course-save-actions blueprint-course-detail-actions">
                      <button
                        type="button"
                        disabled={
                          metadataSaving() ||
                          (shortName() === loaded().view.short_name &&
                            longName() === loaded().view.long_name)
                        }
                        onClick={() => void saveNames()}
                      >
                        {metadataSaving() ? "Saving names..." : "Save Blueprint Course names"}
                      </button>
                      <Show when={metadataConflict()}>
                        <div class="blueprint-course-inline-actions">
                          <p class="blueprint-course-field-help" role="status">
                            Blueprint Course metadata changed elsewhere. Your typed names remain
                            here.
                          </p>
                          <button
                            type="button"
                            class="quiet-action"
                            disabled={metadataSaving()}
                            onClick={() => void reloadMetadata()}
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
                <h2>{editing() ? "Course Editor" : "Assessments"}</h2>
                <Show
                  when={selectedAssessment() === undefined}
                  fallback={
                    <button
                      type="button"
                      class="quiet-action"
                      onClick={() => setSelectedAssessment(undefined)}
                    >
                      Return to assessment list
                    </button>
                  }
                >
                  <Show when={editing()}>
                    <p>Select an assessment to edit its Questions and defaults.</p>
                  </Show>
                  <For each={loaded().content.modules}>
                    {(module, moduleIndex) => (
                      <section class="blueprint-course-module">
                        <h3>{module.label}</h3>
                        <ul class="blueprint-course-assessment-list">
                          <For each={module.assessments}>
                            {(assessment, assessmentIndex) => (
                              <li>
                                <span>{assessment.content.title}</span>
                                <button
                                  type="button"
                                  class="quiet-action"
                                  onClick={() =>
                                    setSelectedAssessment({
                                      moduleIndex: moduleIndex(),
                                      assessmentIndex: assessmentIndex(),
                                    })
                                  }
                                >
                                  {editing() ? "Edit assessment" : "View assessment"}
                                </button>
                              </li>
                            )}
                          </For>
                        </ul>
                      </section>
                    )}
                  </For>
                </Show>
                <Show when={selectedAssessment()} keyed>
                  {(selection) => {
                    const content = ():
                      | ReplaceBlueprintCourseContentInput["modules"][number]["assessments"][number]["content"]
                      | undefined =>
                      current()?.content.modules[selection.moduleIndex]?.assessments[
                        selection.assessmentIndex
                      ]?.content;
                    return (
                      <Show when={content()}>
                        {(assessmentContent) => (
                          <section class="blueprint-course-content-card">
                            <BlueprintAssessmentContentEditor
                              content={assessmentContent()}
                              blueprintRef={props.blueprintCourseRef}
                              blueprintClient={props.client}
                              retainedAssessmentRef={((): string | undefined => {
                                const choice =
                                  current()?.content.modules[selection.moduleIndex]?.assessments[
                                    selection.assessmentIndex
                                  ]?.choice;
                                return choice?.kind === "retained"
                                  ? choice.blueprint_assessment_reference
                                  : undefined;
                              })()}
                              editable={
                                editing() && loaded().view.read_access === "blueprint_course_owner"
                              }
                              pickerRepository={props.pickerRepository}
                              pickerSources={props.pickerSources}
                              onInvalidDraftChange={setInvalidDraft}
                              onChange={(nextContent, text) =>
                                changeAssessment(
                                  selection.moduleIndex,
                                  selection.assessmentIndex,
                                  nextContent,
                                  text,
                                )
                              }
                            />
                          </section>
                        )}
                      </Show>
                    );
                  }}
                </Show>
              </div>
            </section>
          )}
        </Match>
      </Switch>
      <UnsavedChangesGuard
        dirty={dirty}
        save={save}
        copy={{
          heading: "Save Blueprint Course changes?",
          description: "Your reusable Blueprint Course changes have not been saved as a Revision.",
          saveActionLabel: "Save and continue",
          savingActionLabel: "Saving Blueprint Course...",
          saveFailureMessage:
            "Blueprint Course changes were not saved. Resolve the save error, then try again or stay here.",
        }}
      />
    </section>
  );
}
