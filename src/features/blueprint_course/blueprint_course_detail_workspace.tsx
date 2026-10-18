// Explicit Revision Save editing for reusable Blueprint Courses.
import { A } from "@solidjs/router";
import {
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import type { Theme } from "../../../generated/api/Theme";
import { useAppearance } from "../../appearance/appearance_context";
import { ThemeChooser } from "../../appearance/theme_chooser";
import { UnsavedChangesGuard, trackEditorSave } from "../../components/unsaved_changes_guard";
import { CourseClassificationEditor } from "../../components/course_classification_editor";
import { PageFrame } from "../../components/page_frame/page_frame";
import { useSelectedDisplayDateTimeFormatter } from "../../selected_display_zone";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import { parseBlueprintCourseId } from "../../navigation/public_route";
import { blueprintDetailCollectionLink } from "../../pages/blueprint_course_search_return_state";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  usePublishRouteScopeNavigation,
  useRouteScopePublication,
} from "../../ribbon/route_scope_context";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { BlueprintCourseLifecycleControls } from "./blueprint_course_lifecycle_controls";
import {
  BlueprintCourseDetailStructure,
  type SelectedBlueprintAssessment,
} from "./blueprint_course_detail_structure";
import { BlueprintCourseExport } from "./blueprint_exchange";
import { BlueprintHistory } from "./blueprint_history";
import { BlueprintStewardship } from "./blueprint_stewardship";
import { BlueprintForkSource, BlueprintKnownForks } from "../blueprint_forks/blueprint_fork_review";
import { BlueprintForkCreate } from "../blueprint_forks/blueprint_fork_create";
import { ProposalTargetTools } from "../blueprint_change_proposal/proposal_workspace";
import {
  blueprintLifecyclePresentation,
  replacementContentFromBlueprintModules,
  validateReusableContent,
} from "./blueprint_course_model";
import type { BlueprintCourseDetailWorkspaceProps } from "./blueprint_course_workspace_types";
import {
  questionNativePreviewKey,
  type QuestionNativePreview,
} from "../../components/question_native_preview";
import "./blueprint_course.css";

type LoadState = "loading" | "ready" | "error";
type NoticeKind = "status" | "alert";

interface Notice {
  readonly kind: NoticeKind;
  readonly text: string;
}

interface LoadedBlueprintCourse {
  readonly view: BlueprintCourseView;
  readonly content: ReplaceBlueprintCourseContentInput;
  readonly savedContent: ReplaceBlueprintCourseContentInput;
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
  const formatInAccountZone = useSelectedDisplayDateTimeFormatter();
  const appearance = useAppearance();
  const formatDateTime = (timestamp: number | Date): string => formatInAccountZone()(timestamp);
  const [editing, setEditing] = createSignal(false);
  const [selectedAssessment, setSelectedAssessment] = createSignal<SelectedBlueprintAssessment>();
  const [state, setState] = createSignal<LoadState>("loading");
  const [current, setCurrent] = createSignal<LoadedBlueprintCourse>();
  // Recognition metadata stays outside persisted editor inputs and survives closing the local editor.
  const [pickedQuestionPreviews, setPickedQuestionPreviews] = createSignal<
    ReadonlyMap<string, QuestionNativePreview>
  >(new Map());
  const questionPreviews = createMemo(() => {
    const previews = new Map(pickedQuestionPreviews());
    for (const module of current()?.view.modules ?? []) {
      for (const assessment of module.assessments) {
        for (const entry of assessment.content.entries) {
          if (entry.kind !== "fixed") continue;
          const tuple = entry.question.published_question_revision_tuple;
          previews.set(questionNativePreviewKey(tuple), {
            publishedQuestionRevisionTuple: tuple,
            nativePrompt: entry.question.question_library.nativePrompt,
          });
        }
      }
    }
    return previews;
  });
  function rememberQuestionPreviews(questions: ReadonlyArray<QuestionNativePreview>): void {
    setPickedQuestionPreviews((previous) => {
      const next = new Map(previous);
      for (const question of questions) {
        next.set(questionNativePreviewKey(question.publishedQuestionRevisionTuple), question);
      }
      return next;
    });
  }
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
  const [themeDraft, setThemeDraft] = createSignal<Theme>("grass");
  const [archiveConfirmation, setArchiveConfirmation] = createSignal("");
  const [invalidDraft, setInvalidDraft] = createSignal(false);
  const [refreshFailed, setRefreshFailed] = createSignal(false);
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const publishRouteScopeNavigation = usePublishRouteScopeNavigation();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let activePublication: ReturnType<typeof routeScopePublication> | undefined;
  const assessmentTriggers = new Map<string, HTMLButtonElement>();

  function assessmentTriggerId(moduleIndex: number, assessmentIndex: number): string {
    return `blueprint-assessment-${moduleIndex}-${assessmentIndex}-action`;
  }

  function selectAssessment(moduleIndex: number, assessmentIndex: number): void {
    setSelectedAssessment({
      moduleIndex,
      assessmentIndex,
      triggerId: assessmentTriggerId(moduleIndex, assessmentIndex),
    });
  }

  function returnToAssessmentList(): void {
    const selection = selectedAssessment();
    if (selection === undefined) return;
    setSelectedAssessment(undefined);
    queueMicrotask(() => assessmentTriggers.get(selection.triggerId)?.focus());
  }

  function publishBlueprintLabels(view: BlueprintCourseView): void {
    if (activePublication === undefined) return;
    publishRouteScopeLabels(activePublication, {
      blueprintCourseTitle: view.long_name,
      blueprintBreadcrumbParent:
        view.read_access === "blueprint_course_owner"
          ? "myBlueprintCourses"
          : "publicBlueprintSearch",
    });
  }
  function publishBlueprintNavigation(): void {
    if (activePublication === undefined) return;
    publishRouteScopeNavigation(activePublication, {});
  }
  const blueprintCollection = (): { readonly href: string; readonly label: string } =>
    blueprintDetailCollectionLink(current()?.view.read_access);
  createEffect(() => {
    const view = current()?.view;
    if (view !== undefined) publishBlueprintNavigation();
  });
  const dirty = (): boolean => {
    const loaded = current();
    return (
      invalidDraft() || (loaded !== undefined && !sameContent(loaded.content, loaded.savedContent))
    );
  };
  const namesDirty = (): boolean => {
    const loaded = current();
    return (
      loaded !== undefined &&
      (shortName() !== loaded.view.short_name || longName() !== loaded.view.long_name)
    );
  };
  const themeDirty = (): boolean => {
    const loaded = current();
    return loaded !== undefined && themeDraft() !== loaded.view.theme;
  };
  const hasUnsavedForkChanges = (): boolean => {
    return dirty() || namesDirty() || themeDirty();
  };
  /** Refresh server baselines while retaining each independent unsaved metadata draft. */
  function mergeServerView(
    view: BlueprintCourseView,
    content?: Pick<LoadedBlueprintCourse, "content" | "savedContent">,
  ): void {
    const prior = current();
    const preserveShortName = prior !== undefined && shortName() !== prior.view.short_name;
    const preserveLongName = prior !== undefined && longName() !== prior.view.long_name;
    const preserveTheme = themeDirty();
    setCurrent({
      view,
      content:
        content?.content ?? prior?.content ?? replacementContentFromBlueprintModules(view.modules),
      savedContent:
        content?.savedContent ??
        prior?.savedContent ??
        replacementContentFromBlueprintModules(view.modules),
    });
    if (!preserveShortName) setShortName(view.short_name);
    if (!preserveLongName) setLongName(view.long_name);
    if (!preserveTheme) setThemeDraft(view.theme);
    publishBlueprintLabels(view);
  }

  async function load(scope: "initial" | "refresh" | "discardStructure"): Promise<void> {
    const publication = routeScopePublication();
    activePublication = publication;
    clearRouteScopeLabels(publication);
    if (parseBlueprintCourseId(props.blueprintCourseId) === null) {
      setState("error");
      setNotice({ kind: "alert", text: "This Blueprint Course ID is invalid." });
      return;
    }
    setRefreshFailed(false);
    if (scope === "initial") {
      setState("loading");
      setPickedQuestionPreviews(new Map());
    }
    try {
      const result = await props.client.getBlueprintCourse(props.blueprintCourseId);
      const prior = current();
      const savedContent = replacementContentFromBlueprintModules(result.blueprintCourse.modules);
      const content =
        scope === "refresh" && prior !== undefined && dirty() ? prior.content : savedContent;
      mergeServerView(result.blueprintCourse, {
        content,
        savedContent,
      });
      setConflict(false);
      setState("ready");
      setNotice({
        kind: "status",
        text:
          result.blueprintCourse.read_access === "blueprint_course_owner"
            ? "Blueprint Course loaded. Save creates one immutable Blueprint Revision."
            : "Blueprint Course loaded. Inspect its answer-free reusable structure.",
      });
    } catch (error: unknown) {
      clearRouteScopeLabels(publication);
      if (scope !== "initial" && current() !== undefined) setRefreshFailed(true);
      else setState("error");
      setNotice({
        kind: "alert",
        text:
          scope !== "initial"
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
        loaded.view.id,
        loaded.content,
        loaded.view.current_revision_tuple.revisionNumber,
        crypto.randomUUID(),
      );
      const savedContent = replacementContentFromBlueprintModules(saved.blueprintCourse.modules);
      mergeServerView(saved.blueprintCourse, {
        content: sameContent(current()?.content ?? loaded.content, loaded.content)
          ? savedContent
          : current()!.content,
        savedContent,
      });
      setConflict(false);
      setNotice({
        kind: "status",
        text: saved.changed
          ? `Saved Blueprint Revision ${saved.blueprintCourse.current_revision_tuple.revisionNumber}.`
          : `No content changed. Blueprint Revision ${saved.blueprintCourse.current_revision_tuple.revisionNumber} remains current.`,
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
  const trackedSave = trackEditorSave(save);

  function applySavedNames(
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
    const view = {
      ...loaded.view,
      short_name: metadata.metadata.short_name,
      long_name: metadata.metadata.long_name,
      availability: metadata.metadata.availability,
      classification: metadata.metadata.classification,
      theme: metadata.metadata.theme,
      blueprint_edit_number: metadata.metadata.blueprint_edit_number,
    };
    mergeServerView(view);
    if (metadata.metadata.availability === "archived") setEditing(false);
    setMetadataConflict(false);
  }

  function refreshMetadataView(latest: BlueprintCourseView): void {
    const prior = current();
    if (prior === undefined || prior.view.id !== latest.id) return;
    const view = {
      ...prior.view,
      short_name: latest.short_name,
      long_name: latest.long_name,
      availability: latest.availability,
      classification: latest.classification,
      theme: latest.theme,
      blueprint_edit_number: latest.blueprint_edit_number,
    };
    mergeServerView(view);
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

  async function saveNames(): Promise<boolean> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canEdit ||
      !validNames()
    ) {
      return false;
    }
    setMetadataSaving(true);
    try {
      const metadata = await props.client.renameBlueprintCourse(
        loaded.view.id,
        { short_name: shortName(), long_name: longName() },
        loaded.view.blueprint_edit_number,
      );
      applySavedNames(metadata);
      setNotice({
        kind: "status",
        text: "Blueprint Course names saved. Revision content is unchanged.",
      });
      return true;
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
      return false;
    } finally {
      setMetadataSaving(false);
    }
  }
  const trackedNamesSave = trackEditorSave(saveNames);

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
        loaded.view.id,
        archiveConfirmation(),
        loaded.view.blueprint_edit_number,
      );
      applyMetadataState(metadata);
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
        loaded.view.id,
        loaded.view.blueprint_edit_number,
      );
      applyMetadataState(metadata);
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

  async function reloadMetadata(): Promise<void> {
    const loaded = current();
    if (loaded === undefined) return;
    setMetadataSaving(true);
    try {
      const result = await props.client.getBlueprintCourse(loaded.view.id);
      refreshMetadataView(result.blueprintCourse);
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

  async function saveTheme(): Promise<boolean> {
    const loaded = current();
    if (
      loaded === undefined ||
      !blueprintLifecyclePresentation(loaded.view.availability, loaded.view.read_access).canEdit
    )
      return false;
    setMetadataSaving(true);
    try {
      const transition = await props.client.updateBlueprintCourseTheme(
        loaded.view.id,
        { theme: themeDraft() },
        loaded.view.blueprint_edit_number,
      );
      applyMetadataState(transition);
      setThemeDraft(transition.metadata.theme);
      setNotice({ kind: "status", text: "Blueprint Theme saved." });
      return true;
    } catch (error: unknown) {
      setMetadataConflict(error instanceof BlueprintCourseConflictError);
      setNotice({
        kind: "alert",
        text: errorMessage(error, "Blueprint Theme could not be saved."),
      });
      return false;
    } finally {
      setMetadataSaving(false);
    }
  }
  const trackedThemeSave = trackEditorSave(saveTheme);

  onMount(() => void load("initial"));
  onCleanup(() => {
    if (activePublication !== undefined) clearRouteScopeLabels(activePublication);
  });
  return (
    <PageFrame
      routeSurface="blueprintCourseDetail"
      actions={
        <A class="quiet-link" href={blueprintCollection().href}>
          {blueprintCollection().label}
        </A>
      }
    >
      <p class="blueprint-course-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <Show when={refreshFailed()}>
        <button type="button" onClick={() => void load("refresh")}>
          Retry refreshing Blueprint Course
        </button>
      </Show>
      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading Blueprint Course.</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => void load("initial")}>
            Retry loading Blueprint Course
          </button>
        </Match>
        <Match when={state() === "ready" && current()}>
          {(loaded) => (
            <section class="blueprint-course-detail-editor">
              <BlueprintCourseLifecycleControls
                view={loaded().view}
                placement="primary"
                editing={editing()}
                metadataSaving={metadataSaving()}
                hasUnsavedContent={dirty()}
                archiveConfirmation={archiveConfirmation()}
                onArchiveConfirmationInput={setArchiveConfirmation}
                onToggleEditor={() => {
                  setEditing(!editing());
                  setSelectedAssessment(undefined);
                }}
                onArchive={() => void archive()}
                onRestore={() => void restore()}
              />
              <BlueprintCourseDetailStructure
                editing={editing()}
                canEdit={
                  blueprintLifecyclePresentation(
                    loaded().view.availability,
                    loaded().view.read_access,
                  ).canEdit
                }
                saving={saving()}
                dirty={dirty()}
                conflict={conflict()}
                onSave={() => void trackedSave.save()}
                onDiscard={() => void load("discardStructure")}
                shortName={shortName()}
                longName={longName()}
                savedShortName={loaded().view.short_name}
                savedLongName={loaded().view.long_name}
                onShortNameInput={setShortName}
                onLongNameInput={setLongName}
                metadataSaving={metadataSaving()}
                metadataConflict={metadataConflict()}
                onSaveNames={() => void trackedNamesSave.save()}
                onReloadMetadata={() => void reloadMetadata()}
                modules={loaded().content.modules}
                questionPreviews={questionPreviews()}
                onRememberQuestionPreviews={rememberQuestionPreviews}
                courseLongName={loaded().view.long_name}
                ownerEditing={editing() && loaded().view.read_access === "blueprint_course_owner"}
                selectedAssessment={selectedAssessment()}
                assessmentContent={(moduleIndex, assessmentIndex) =>
                  current()?.content.modules[moduleIndex]?.assessments[assessmentIndex]?.content
                }
                assessmentTriggerId={assessmentTriggerId}
                registerAssessmentTrigger={(triggerId, element) =>
                  assessmentTriggers.set(triggerId, element)
                }
                onSelectAssessment={selectAssessment}
                onReturnToAssessmentList={returnToAssessmentList}
                blueprintClient={props.client}
                pickerRepository={props.pickerRepository}
                pickerSources={props.pickerSources}
                onInvalidDraftChange={setInvalidDraft}
                onChangeAssessment={changeAssessment}
              />
              <Show
                when={
                  loaded().view.availability === "public" ||
                  loaded().view.availability === "archived"
                }
              >
                <BlueprintStewardship
                  client={props.client}
                  blueprintCourseId={loaded().view.id}
                  formatDateTime={formatDateTime}
                />
              </Show>
              <CourseClassificationEditor
                value={loaded().view.classification}
                editNumber={loaded().view.blueprint_edit_number}
                canEdit={
                  blueprintLifecyclePresentation(
                    loaded().view.availability,
                    loaded().view.read_access,
                  ).canEdit && !metadataSaving()
                }
                save={async (classification, editNumber) => {
                  const transition = await props.client.updateBlueprintCourseClassification(
                    loaded().view.id,
                    classification,
                    editNumber,
                  );
                  // Classification saves update metadata without consuming unrelated name drafts.
                  applyMetadataState(transition);
                }}
                reload={async () => {
                  const latest = await props.client.getBlueprintCourse(loaded().view.id);
                  refreshMetadataView(latest.blueprintCourse);
                  return {
                    classification: latest.blueprintCourse.classification,
                    editNumber: latest.blueprintCourse.blueprint_edit_number,
                  };
                }}
              />
              <fieldset
                class="blueprint-course-theme-editor"
                disabled={
                  metadataSaving() ||
                  loaded().view.availability === "archived" ||
                  loaded().view.read_access !== "blueprint_course_owner"
                }
              >
                <legend>Blueprint Theme</legend>
                <p>Courses created from this Blueprint start with this Theme.</p>
                <ThemeChooser
                  name="blueprint-theme"
                  selectedTheme={themeDraft}
                  mode={() => appearance.appearance().mode}
                  disabled={() => metadataSaving()}
                  onSelect={setThemeDraft}
                />
                <button
                  type="button"
                  disabled={themeDraft() === loaded().view.theme}
                  onClick={() => {
                    void trackedThemeSave.save();
                  }}
                >
                  Save Blueprint Theme
                </button>
              </fieldset>
              <BlueprintHistory
                client={props.client}
                view={loaded().view}
                formatDateTime={formatDateTime}
              />
              <BlueprintCourseExport client={props.client} blueprintCourseId={loaded().view.id} />
              <BlueprintForkCreate client={props.client} source={loaded().view} />
              <Show when={props.proposalClient}>
                {(client) => (
                  <ProposalTargetTools
                    client={client()}
                    classificationClient={props.client}
                    target={loaded().view}
                    formatDateTime={formatDateTime}
                  />
                )}
              </Show>
              <BlueprintForkSource
                client={props.client}
                view={loaded().view}
                hasUnsavedChanges={hasUnsavedForkChanges()}
                onApplied={() => void load("refresh")}
              />
              <BlueprintKnownForks
                client={props.client}
                blueprintCourseId={loaded().view.id}
                sourceCurrentRevision={loaded().view.current_revision_tuple.revisionNumber}
                hasUnsavedChanges={hasUnsavedForkChanges()}
                onApplied={() => void load("refresh")}
              />
              <BlueprintCourseLifecycleControls
                view={loaded().view}
                placement="administration"
                editing={editing()}
                metadataSaving={metadataSaving()}
                hasUnsavedContent={dirty()}
                archiveConfirmation={archiveConfirmation()}
                onArchiveConfirmationInput={setArchiveConfirmation}
                onToggleEditor={() => {
                  setEditing(!editing());
                  setSelectedAssessment(undefined);
                }}
                onArchive={() => void archive()}
                onRestore={() => void restore()}
              />
            </section>
          )}
        </Match>
      </Switch>
      <UnsavedChangesGuard
        dirty={dirty}
        save={trackedSave.save}
        inFlightSave={trackedSave.inFlightSave}
        copy={{
          heading: "Save Blueprint Course changes?",
          description: "Your reusable Blueprint Course changes have not been saved as a Revision.",
          saveActionLabel: "Save and continue",
          savingActionLabel: "Saving Blueprint Course...",
          saveFailureMessage:
            "Blueprint Course changes were not saved. Resolve the save error, then try again or stay here.",
        }}
      />
      <UnsavedChangesGuard
        dirty={namesDirty}
        save={trackedNamesSave.save}
        inFlightSave={trackedNamesSave.inFlightSave}
        copy={{
          heading: "Save Blueprint Course names?",
          description: "Your Blueprint Course names have not been saved.",
          saveActionLabel: "Save names and continue",
          savingActionLabel: "Saving names...",
          saveFailureMessage:
            "Blueprint Course names were not saved. Correct the names or resolve the metadata error before leaving.",
        }}
      />
      <UnsavedChangesGuard
        dirty={themeDirty}
        save={trackedThemeSave.save}
        inFlightSave={trackedThemeSave.inFlightSave}
        copy={{
          heading: "Save Blueprint Theme?",
          description: "Your Blueprint Theme selection has not been saved.",
          saveActionLabel: "Save Theme and continue",
          savingActionLabel: "Saving Theme...",
          saveFailureMessage:
            "Blueprint Theme was not saved. Resolve the metadata error before leaving.",
        }}
      />
    </PageFrame>
  );
}
