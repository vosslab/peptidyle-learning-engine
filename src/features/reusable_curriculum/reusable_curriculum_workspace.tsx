// reusable_curriculum_workspace.tsx - live discovery and authoring workspace for reusable curricula.

import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createSignal, onMount, type JSX } from "solid-js";

import type { AlphaCourseDefinitionInput } from "../../../generated/api/AlphaCourseDefinitionInput";
import type { AlphaCourseSummaryView } from "../../../generated/api/AlphaCourseSummaryView";
import type { AlphaCourseView } from "../../../generated/api/AlphaCourseView";
import type { BlueprintDefinitionInput } from "../../../generated/api/BlueprintDefinitionInput";
import type { BlueprintSummaryView } from "../../../generated/api/BlueprintSummaryView";
import type { BlueprintView } from "../../../generated/api/BlueprintView";
import { ApiRequestError } from "../../api/http_client";
import type {
  ReusableCurriculumClient,
  ReusableCurriculumEtag,
} from "../../api/reusable_curriculum";
import type { ProblemPickerSource, ProblemPickerSourceRepository } from "../problem_picker";
import {
  alphaInputFromView,
  alphaProblemPickerSources,
  appendCurriculumPage,
  appendAlphaDefinition,
  appendAlphaModule,
  blueprintInputFromView,
  curriculumContinuationPresentation,
  moveAlphaDefinition,
  moveAlphaModule,
  removeAlphaDefinition,
  removeAlphaModule,
  updateAlphaDefinition,
  validateAlphaDefinition,
  validateReusableDefinition,
} from "./reusable_curriculum_model";
import { ReusableDefinitionEditor } from "./reusable_definition_editor";
import { CurriculumCreateDialog } from "./reusable_curriculum_create_dialog";
import "./reusable_curriculum.css";

type LoadState = "loading" | "ready" | "error";
type DetailKind = "blueprint" | "alpha";
type DetailState = "loading" | "ready" | "error";
type WorkspaceNoticeKind = "status" | "alert";

interface WorkspaceNotice {
  readonly kind: WorkspaceNoticeKind;
  readonly text: string;
}

interface LoadedBlueprint {
  readonly view: BlueprintView;
  readonly etag: ReusableCurriculumEtag;
  readonly draft: BlueprintDefinitionInput;
}

interface LoadedAlpha {
  readonly view: AlphaCourseView;
  readonly etag: ReusableCurriculumEtag;
  readonly draft: AlphaCourseDefinitionInput;
}

export interface CurriculumWorkspaceProps {
  readonly client: ReusableCurriculumClient;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<ProblemPickerSource>;
}

export interface CurriculumDetailWorkspaceProps extends CurriculumWorkspaceProps {
  readonly curriculumRef: string;
}

function referencePath(reference: string): string {
  return `/curriculum/${encodeURIComponent(reference)}`;
}

function creatorByline(summary: AlphaCourseSummaryView): string {
  return summary.creatorByline.names.join(", ");
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiRequestError) {
    if (error.status === 401)
      return "Your session ended. Sign in again, then return to this live workspace.";
    if (error.status === 403) return "This curriculum is unavailable for this instructor account.";
    if (error.status === 404) return "This curriculum is no longer available.";
  }
  return fallback;
}

function isConflict(error: unknown): boolean {
  return error instanceof ApiRequestError && error.status === 412;
}

function hasPrefix(reference: string, prefix: "BP-" | "AC-"): boolean {
  return reference.startsWith(prefix);
}

/** Lists live personal blueprints alongside shareable Alpha curricula. */
export function CurriculumWorkspace(props: CurriculumWorkspaceProps): JSX.Element {
  const [state, setState] = createSignal<LoadState>("loading");
  const [blueprints, setBlueprints] = createSignal<ReadonlyArray<BlueprintSummaryView>>([]);
  const [alphaCourses, setAlphaCourses] = createSignal<ReadonlyArray<AlphaCourseSummaryView>>([]);
  const [blueprintCursor, setBlueprintCursor] = createSignal<string | null>(null);
  const [alphaCursor, setAlphaCursor] = createSignal<string | null>(null);
  const [loadingMoreBlueprints, setLoadingMoreBlueprints] = createSignal(false);
  const [loadingMoreAlpha, setLoadingMoreAlpha] = createSignal(false);
  const [continuationFailure, setContinuationFailure] = createSignal<DetailKind>();
  const [notice, setNotice] = createSignal<WorkspaceNotice>({
    kind: "status",
    text: "Loading your reusable curriculum workspace.",
  });
  const [createKind, setCreateKind] = createSignal<DetailKind>();
  let createTrigger: HTMLButtonElement | undefined;

  async function load(): Promise<void> {
    setState("loading");
    setNotice({ kind: "status", text: "Loading blueprints and public Alpha curricula." });
    try {
      const [blueprintPage, alphaPage] = await Promise.all([
        props.client.listBlueprints(undefined, 50),
        props.client.listAlphaCourses(undefined, 50),
      ]);
      setBlueprints(blueprintPage.items);
      setAlphaCourses(alphaPage.items);
      setBlueprintCursor(blueprintPage.nextCursor);
      setAlphaCursor(alphaPage.nextCursor);
      setContinuationFailure(undefined);
      setState("ready");
      setNotice({
        kind: "status",
        text: "Choose a curriculum to inspect, or create a reusable starting point for your next course.",
      });
    } catch (error: unknown) {
      setState("error");
      setNotice({
        kind: "alert",
        text: errorMessage(error, "Curricula could not load. Try again."),
      });
    }
  }

  async function loadMoreBlueprints(): Promise<void> {
    const cursor = blueprintCursor();
    if (cursor === null || loadingMoreBlueprints()) return;
    setLoadingMoreBlueprints(true);
    setContinuationFailure(undefined);
    setNotice({ kind: "status", text: "Loading more of your private blueprints." });
    try {
      const page = await props.client.listBlueprints(cursor, 50);
      setBlueprints((current) => appendCurriculumPage(current, page.items));
      setBlueprintCursor(page.nextCursor);
      setNotice({ kind: "status", text: "More blueprints are ready to inspect." });
    } catch (error: unknown) {
      setContinuationFailure("blueprint");
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "More blueprints could not load. The visible list is still available; try loading more again.",
        ),
      });
    } finally {
      setLoadingMoreBlueprints(false);
    }
  }

  async function loadMoreAlpha(): Promise<void> {
    const cursor = alphaCursor();
    if (cursor === null || loadingMoreAlpha()) return;
    setLoadingMoreAlpha(true);
    setContinuationFailure(undefined);
    setNotice({ kind: "status", text: "Loading more public Alpha curricula." });
    try {
      const page = await props.client.listAlphaCourses(cursor, 50);
      setAlphaCourses((current) => appendCurriculumPage(current, page.items));
      setAlphaCursor(page.nextCursor);
      setNotice({ kind: "status", text: "More Alpha curricula are ready to inspect and reuse." });
    } catch (error: unknown) {
      setContinuationFailure("alpha");
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "More Alpha curricula could not load. The visible list is still available; try loading more again.",
        ),
      });
    } finally {
      setLoadingMoreAlpha(false);
    }
  }

  function beginCreate(kind: DetailKind, trigger: HTMLButtonElement): void {
    createTrigger = trigger;
    setCreateKind(kind);
    setNotice({
      kind: "status",
      text: "Name the curriculum and choose its first published question. The live server then creates a complete reusable definition.",
    });
  }

  onMount(() => void load());

  return (
    <main class="page curriculum-workspace" data-route-surface="curriculum">
      <header class="curriculum-page-heading">
        <p class="eyebrow">Reusable curriculum</p>
        <h1>Build once, adapt for each course</h1>
        <p class="page-lede">
          Blueprints are your private reusable assignments. Alpha curricula are public, answer-free
          instructor resources with no students, runs, or grades.
        </p>
      </header>
      <p class="curriculum-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading live curriculum records.</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => void load()}>
            Retry loading curricula
          </button>
        </Match>
        <Match when={state() === "ready"}>
          <section class="curriculum-grid" aria-label="Curriculum choices">
            <CurriculumList
              title="My blueprints"
              description="Private reusable assignments you can revise and later adapt to a teaching course."
              items={blueprints()}
              empty="Create a personal blueprint to save a reusable assignment definition."
              onCreate={(trigger) => beginCreate("blueprint", trigger)}
              hasMore={blueprintCursor() !== null}
              loadingMore={loadingMoreBlueprints()}
              continuationFailed={continuationFailure() === "blueprint"}
              onLoadMore={() => void loadMoreBlueprints()}
            />
            <AlphaList
              items={alphaCourses()}
              onCreate={(trigger) => beginCreate("alpha", trigger)}
              hasMore={alphaCursor() !== null}
              loadingMore={loadingMoreAlpha()}
              continuationFailed={continuationFailure() === "alpha"}
              onLoadMore={() => void loadMoreAlpha()}
            />
          </section>
        </Match>
      </Switch>
      <Show when={createKind()} keyed>
        {(kind) => (
          <CurriculumCreateDialog
            kind={kind}
            client={props.client}
            pickerRepository={props.pickerRepository}
            pickerSources={props.pickerSources}
            onClose={() => {
              setCreateKind(undefined);
              queueMicrotask(() => createTrigger?.focus());
            }}
            onFailure={(text) => setNotice({ kind: "alert", text })}
          />
        )}
      </Show>
    </main>
  );
}

interface CurriculumListProps {
  readonly title: string;
  readonly description: string;
  readonly items: ReadonlyArray<BlueprintSummaryView>;
  readonly empty: string;
  readonly onCreate: (trigger: HTMLButtonElement) => void;
  readonly hasMore: boolean;
  readonly loadingMore: boolean;
  readonly continuationFailed: boolean;
  readonly onLoadMore: () => void;
}

function CurriculumList(props: CurriculumListProps): JSX.Element {
  const continuation = (): ReturnType<typeof curriculumContinuationPresentation> =>
    curriculumContinuationPresentation("blueprint", props.hasMore, props.continuationFailed);
  return (
    <section class="curriculum-card" aria-labelledby="blueprint-heading">
      <div class="curriculum-section-heading">
        <div>
          <h2 id="blueprint-heading">{props.title}</h2>
          <p>{props.description}</p>
        </div>
        <button type="button" onClick={(event) => props.onCreate(event.currentTarget)}>
          Create blueprint
        </button>
      </div>
      <Show
        when={props.items.length > 0}
        fallback={<p class="curriculum-empty-copy">{props.empty}</p>}
      >
        <ul class="curriculum-summary-list">
          <For each={props.items}>
            {(item) => (
              <li>
                <A href={referencePath(item.reference)}>
                  <strong>{item.title}</strong>
                  <span>Private blueprint, revision {item.revision}</span>
                </A>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={continuation().visible}>
        <div class="curriculum-continuation">
          <p role={props.continuationFailed ? "alert" : "status"}>
            {props.continuationFailed
              ? "More blueprints are available. Retry when you are ready."
              : "More blueprints are available."}
          </p>
          <button type="button" disabled={props.loadingMore} onClick={props.onLoadMore}>
            {props.loadingMore ? "Loading more blueprints..." : continuation().action}
          </button>
        </div>
      </Show>
    </section>
  );
}

interface AlphaListProps {
  readonly items: ReadonlyArray<AlphaCourseSummaryView>;
  readonly onCreate: (trigger: HTMLButtonElement) => void;
  readonly hasMore: boolean;
  readonly loadingMore: boolean;
  readonly continuationFailed: boolean;
  readonly onLoadMore: () => void;
}

function AlphaList(props: AlphaListProps): JSX.Element {
  const continuation = (): ReturnType<typeof curriculumContinuationPresentation> =>
    curriculumContinuationPresentation("alpha", props.hasMore, props.continuationFailed);
  return (
    <section class="curriculum-card" aria-labelledby="alpha-heading">
      <div class="curriculum-section-heading">
        <div>
          <h2 id="alpha-heading">Public Alpha curricula</h2>
          <p>
            Shared reusable instructor curricula. Readers inspect and reuse question sets; creators
            manage the aggregate.
          </p>
        </div>
        <button type="button" onClick={(event) => props.onCreate(event.currentTarget)}>
          Create Alpha curriculum
        </button>
      </div>
      <Show
        when={props.items.length > 0}
        fallback={
          <p class="curriculum-empty-copy">
            Public Alpha curricula will appear here when approved instructors publish them.
          </p>
        }
      >
        <ul class="curriculum-summary-list">
          <For each={props.items}>
            {(item) => (
              <li>
                <A href={referencePath(item.reference)}>
                  <strong>{item.title}</strong>
                  <span>
                    Created by {creatorByline(item)}.{" "}
                    {item.access === "creator"
                      ? "You can edit this curriculum."
                      : "Inspect and reuse its questions."}
                  </span>
                </A>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={continuation().visible}>
        <div class="curriculum-continuation">
          <p role={props.continuationFailed ? "alert" : "status"}>
            {props.continuationFailed
              ? "More Alpha curricula are available. Retry when you are ready."
              : "More Alpha curricula are available."}
          </p>
          <button type="button" disabled={props.loadingMore} onClick={props.onLoadMore}>
            {props.loadingMore ? "Loading more Alpha curricula..." : continuation().action}
          </button>
        </div>
      </Show>
    </section>
  );
}

/** Loads one BP-* or AC-* curriculum and preserves a local draft through a conflict response. */
export function CurriculumDetailWorkspace(props: CurriculumDetailWorkspaceProps): JSX.Element {
  const kind = (): DetailKind | undefined =>
    hasPrefix(props.curriculumRef, "BP-")
      ? "blueprint"
      : hasPrefix(props.curriculumRef, "AC-")
        ? "alpha"
        : undefined;
  const [state, setState] = createSignal<DetailState>("loading");
  const [blueprint, setBlueprint] = createSignal<LoadedBlueprint>();
  const [alpha, setAlpha] = createSignal<LoadedAlpha>();
  const [notice, setNotice] = createSignal<WorkspaceNotice>({
    kind: "status",
    text: "Loading the current curriculum.",
  });
  const [saving, setSaving] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);

  async function load(keepDraft: boolean): Promise<void> {
    const currentKind = kind();
    if (currentKind === undefined) {
      setState("error");
      setNotice({ kind: "alert", text: "Curriculum references begin with BP- or AC-." });
      return;
    }
    setState("loading");
    setNotice({ kind: "status", text: "Loading the current curriculum version." });
    try {
      if (currentKind === "blueprint") {
        const current = await props.client.getBlueprint(props.curriculumRef);
        const prior = blueprint();
        setBlueprint({
          view: current.blueprint,
          etag: current.etag,
          draft:
            keepDraft && prior !== undefined
              ? prior.draft
              : blueprintInputFromView(current.blueprint),
        });
      } else {
        const current = await props.client.getAlphaCourse(props.curriculumRef);
        const prior = alpha();
        setAlpha({
          view: current.alpha,
          etag: current.etag,
          draft: keepDraft && prior !== undefined ? prior.draft : alphaInputFromView(current.alpha),
        });
      }
      setConflict(false);
      if (!keepDraft) setDirty(false);
      setState("ready");
      setNotice({
        kind: "status",
        text: keepDraft
          ? "Current version reloaded. Your local draft is still visible for comparison."
          : "Current curriculum loaded. Review its definition, then make the next useful edit.",
      });
    } catch (error: unknown) {
      setState("error");
      setNotice({
        kind: "alert",
        text: errorMessage(error, "The curriculum could not load. Try again."),
      });
    }
  }

  function changeBlueprint(draft: BlueprintDefinitionInput, text: string): void {
    const current = blueprint();
    if (current === undefined) return;
    setBlueprint({ ...current, draft });
    setDirty(true);
    setNotice({ kind: "status", text });
  }

  function changeAlpha(draft: AlphaCourseDefinitionInput, text: string): void {
    const current = alpha();
    if (current === undefined) return;
    setAlpha({ ...current, draft });
    setDirty(true);
    setNotice({ kind: "status", text });
  }

  async function save(): Promise<void> {
    const currentKind = kind();
    if (currentKind === "blueprint") {
      const current = blueprint();
      if (current === undefined) return;
      const validation = validateReusableDefinition(current.draft.definition);
      if (!validation.valid) {
        setNotice({
          kind: "alert",
          text: validation.message ?? "Review this blueprint before saving.",
        });
        return;
      }
      setSaving(true);
      try {
        const saved = await props.client.replaceBlueprint(
          current.view.reference,
          current.draft,
          current.etag,
        );
        setBlueprint({
          view: saved.blueprint,
          etag: saved.etag,
          draft: blueprintInputFromView(saved.blueprint),
        });
        setConflict(false);
        setDirty(false);
        setNotice({
          kind: "status",
          text: "Blueprint saved. It is ready to adapt into a future teaching course.",
        });
      } catch (error: unknown) {
        setConflict(isConflict(error));
        setNotice({
          kind: "alert",
          text: isConflict(error)
            ? "A newer curriculum version exists. Your complete local draft remains here; reload current version or keep editing this draft."
            : errorMessage(error, "Blueprint could not save. Your local draft remains here."),
        });
      } finally {
        setSaving(false);
      }
      return;
    }
    const current = alpha();
    if (current === undefined) return;
    const validation = validateAlphaDefinition(current.draft);
    if (!validation.valid) {
      setNotice({
        kind: "alert",
        text: validation.message ?? "Review this Alpha curriculum before saving.",
      });
      return;
    }
    setSaving(true);
    try {
      const saved = await props.client.replaceAlphaCourse(
        current.view.reference,
        current.draft,
        current.etag,
      );
      setAlpha({ view: saved.alpha, etag: saved.etag, draft: alphaInputFromView(saved.alpha) });
      setConflict(false);
      setDirty(false);
      setNotice({
        kind: "status",
        text: "Alpha curriculum saved. Approved instructors can inspect and reuse its answer-free question set.",
      });
    } catch (error: unknown) {
      setConflict(isConflict(error));
      setNotice({
        kind: "alert",
        text: isConflict(error)
          ? "A newer curriculum version exists. Your complete local draft remains here; reload current version or keep editing this draft."
          : errorMessage(error, "Alpha curriculum could not save. Your local draft remains here."),
      });
    } finally {
      setSaving(false);
    }
  }

  onMount(() => void load(false));

  return (
    <main class="page curriculum-workspace" data-route-surface="curriculumDetail">
      <A class="quiet-link" href="/curriculum">
        Return to all curricula
      </A>
      <p class="curriculum-notice" role={notice().kind === "alert" ? "alert" : "status"}>
        {notice().text}
      </p>
      <Switch>
        <Match when={state() === "loading"}>
          <p>Loading live curriculum record.</p>
        </Match>
        <Match when={state() === "error"}>
          <button type="button" onClick={() => void load(false)}>
            Retry loading this curriculum
          </button>
        </Match>
        <Match when={state() === "ready" && blueprint()} keyed>
          {(current) => (
            <BlueprintDetail
              current={current}
              editable
              pickerRepository={props.pickerRepository}
              pickerSources={props.pickerSources}
              onChange={changeBlueprint}
              onSave={() => void save()}
              saving={saving()}
              conflict={conflict()}
              dirty={dirty()}
              onReload={() => void load(false)}
              onKeep={() => {
                setConflict(false);
                setNotice({
                  kind: "status",
                  text: "Keeping your local draft. Review it, then save when it is ready.",
                });
              }}
            />
          )}
        </Match>
        <Match when={state() === "ready" && alpha()} keyed>
          {(current) => (
            <AlphaDetail
              current={current}
              pickerRepository={props.pickerRepository}
              onChange={changeAlpha}
              onSave={() => void save()}
              saving={saving()}
              conflict={conflict()}
              dirty={dirty()}
              onReload={() => void load(false)}
              onKeep={() => {
                setConflict(false);
                setNotice({
                  kind: "status",
                  text: "Keeping your local draft. Review it, then save when it is ready.",
                });
              }}
            />
          )}
        </Match>
      </Switch>
    </main>
  );
}

interface DetailActionsProps {
  readonly saving: boolean;
  readonly conflict: boolean;
  readonly dirty: boolean;
  readonly onSave: () => void;
  readonly onReload: () => void;
  readonly onKeep: () => void;
}

function DetailActions(props: DetailActionsProps): JSX.Element {
  return (
    <footer class="curriculum-save-actions curriculum-detail-actions">
      <button type="button" disabled={props.saving} onClick={props.onSave}>
        {props.saving ? "Saving..." : "Save curriculum"}
      </button>
      <Show when={props.dirty || props.conflict}>
        <button type="button" class="quiet-action" onClick={props.onReload}>
          {props.conflict ? "Reload current version" : "Discard local changes"}
        </button>
      </Show>
      <Show when={props.conflict}>
        <button type="button" class="quiet-action" onClick={props.onKeep}>
          Keep my draft
        </button>
      </Show>
    </footer>
  );
}

interface BlueprintDetailProps extends DetailActionsProps {
  readonly current: LoadedBlueprint;
  readonly editable: boolean;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<ProblemPickerSource>;
  readonly onChange: (draft: BlueprintDefinitionInput, text: string) => void;
}

function BlueprintDetail(props: BlueprintDetailProps): JSX.Element {
  return (
    <section class="curriculum-detail-editor">
      <header class="curriculum-page-heading">
        <p class="eyebrow">Private blueprint</p>
        <h1>{props.current.draft.definition.title}</h1>
        <p class="page-lede">
          This is a reusable assignment definition. It has no students, assignment runs, or grades.
        </p>
      </header>
      <div class="curriculum-editor-content">
        <ReusableDefinitionEditor
          definition={props.current.draft.definition}
          editable={props.editable}
          pickerRepository={props.pickerRepository}
          pickerSources={props.pickerSources}
          onChange={(definition, text) => props.onChange({ definition }, text)}
        />
      </div>
      <DetailActions {...props} />
    </section>
  );
}

interface AlphaDetailProps extends DetailActionsProps {
  readonly current: LoadedAlpha;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly onChange: (draft: AlphaCourseDefinitionInput, text: string) => void;
}

function AlphaDetail(props: AlphaDetailProps): JSX.Element {
  const editable = (): boolean => props.current.view.access === "creator";
  return (
    <section class="curriculum-detail-editor">
      <header class="curriculum-page-heading">
        <p class="eyebrow">Public Alpha curriculum</p>
        <h1>{props.current.draft.title}</h1>
        <p class="page-lede">
          Reusable instructor curriculum only: no students, learner runs, or grades.{" "}
          {editable()
            ? "You created this Alpha and can revise it."
            : "You can inspect and reuse its answer-free question set."}
        </p>
      </header>
      <div class="curriculum-editor-content">
        <Show when={editable()} fallback={<AlphaInspection draft={props.current.draft} />}>
          <AlphaEditor
            draft={props.current.draft}
            pickerRepository={props.pickerRepository}
            pickerSources={alphaProblemPickerSources()}
            onChange={props.onChange}
          />
        </Show>
      </div>
      <Show when={editable()}>
        <DetailActions {...props} />
      </Show>
    </section>
  );
}

interface AlphaEditorProps {
  readonly draft: AlphaCourseDefinitionInput;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<ProblemPickerSource>;
  readonly onChange: (draft: AlphaCourseDefinitionInput, text: string) => void;
}

function AlphaEditor(props: AlphaEditorProps): JSX.Element {
  function updateModuleLabel(moduleIndex: number, label: string): void {
    const module = props.draft.modules[moduleIndex];
    if (module === undefined) return;
    const modules = [...props.draft.modules];
    modules[moduleIndex] = { ...module, label };
    props.onChange(
      { ...props.draft, modules },
      "Module label updated. Edit its reusable assignments or save the curriculum.",
    );
  }

  return (
    <section class="curriculum-alpha-editor">
      <label>
        Curriculum title
        <input
          value={props.draft.title}
          maxlength="200"
          onInput={(event) =>
            props.onChange(
              { ...props.draft, title: event.currentTarget.value },
              "Alpha curriculum title updated. Review its modules next.",
            )
          }
        />
      </label>
      <div class="curriculum-inline-actions">
        <button
          type="button"
          onClick={() =>
            props.onChange(
              appendAlphaModule(props.draft),
              "Added a module. Give it a meaningful label and define its first reusable assignment.",
            )
          }
        >
          Add module
        </button>
      </div>
      <For each={props.draft.modules}>
        {(module, moduleIndex) => (
          <section class="curriculum-module">
            <div class="curriculum-section-heading">
              <label>
                Module {moduleIndex() + 1} label
                <input
                  value={module.label}
                  maxlength="200"
                  onInput={(event) => updateModuleLabel(moduleIndex(), event.currentTarget.value)}
                />
              </label>
              <div class="curriculum-reorder-actions" aria-label={`Actions for ${module.label}`}>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={moduleIndex() === 0}
                  onClick={() =>
                    props.onChange(
                      moveAlphaModule(props.draft, moduleIndex(), -1),
                      "Module order updated. Continue arranging or save the curriculum.",
                    )
                  }
                >
                  Move earlier
                </button>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={moduleIndex() === props.draft.modules.length - 1}
                  onClick={() =>
                    props.onChange(
                      moveAlphaModule(props.draft, moduleIndex(), 1),
                      "Module order updated. Continue arranging or save the curriculum.",
                    )
                  }
                >
                  Move later
                </button>
                <button
                  type="button"
                  class="danger-action"
                  onClick={() =>
                    props.onChange(
                      removeAlphaModule(props.draft, moduleIndex()),
                      "Module removed. Add another labelled module or save the revised curriculum.",
                    )
                  }
                >
                  Remove module
                </button>
              </div>
            </div>
            <For each={module.definitions}>
              {(definition, definitionIndex) => (
                <section class="curriculum-definition-card">
                  <div
                    class="curriculum-reorder-actions"
                    aria-label={`Actions for ${definition.title}`}
                  >
                    <strong>Reusable assignment {definitionIndex() + 1}</strong>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={definitionIndex() === 0}
                      onClick={() =>
                        props.onChange(
                          moveAlphaDefinition(props.draft, moduleIndex(), definitionIndex(), -1),
                          "Assignment order updated. Continue arranging or save the curriculum.",
                        )
                      }
                    >
                      Move earlier
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={definitionIndex() === module.definitions.length - 1}
                      onClick={() =>
                        props.onChange(
                          moveAlphaDefinition(props.draft, moduleIndex(), definitionIndex(), 1),
                          "Assignment order updated. Continue arranging or save the curriculum.",
                        )
                      }
                    >
                      Move later
                    </button>
                    <button
                      type="button"
                      class="danger-action"
                      onClick={() =>
                        props.onChange(
                          removeAlphaDefinition(props.draft, moduleIndex(), definitionIndex()),
                          "Reusable assignment removed. Add another definition or save the revised curriculum.",
                        )
                      }
                    >
                      Remove assignment
                    </button>
                  </div>
                  <ReusableDefinitionEditor
                    definition={definition}
                    editable
                    pickerRepository={props.pickerRepository}
                    pickerSources={props.pickerSources}
                    onChange={(next, text) =>
                      props.onChange(
                        updateAlphaDefinition(props.draft, moduleIndex(), definitionIndex(), next),
                        text,
                      )
                    }
                  />
                </section>
              )}
            </For>
            <button
              type="button"
              class="quiet-action"
              onClick={() =>
                props.onChange(
                  appendAlphaDefinition(props.draft, moduleIndex()),
                  "Added a reusable assignment. Give it a title and select published questions next.",
                )
              }
            >
              Add reusable assignment
            </button>
          </section>
        )}
      </For>
    </section>
  );
}

function AlphaInspection(props: { readonly draft: AlphaCourseDefinitionInput }): JSX.Element {
  return (
    <section class="curriculum-inspection" aria-labelledby="alpha-inspection-heading">
      <h2 id="alpha-inspection-heading">Inspect and reuse this question set</h2>
      <p>
        Review the ordered modules and Question IDs below, then use the Library or an assignment
        editor to choose these published questions for your own course.
      </p>
      <For each={props.draft.modules}>
        {(module) => (
          <section>
            <h3>{module.label}</h3>
            <For each={module.definitions}>
              {(definition) => (
                <article>
                  <h4>{definition.title}</h4>
                  <ol>
                    {definition.entries.map((entry) => (
                      <li>
                        {entry.kind === "fixed"
                          ? entry.questionId
                          : `Pool: ${entry.candidates.join(", ")}`}
                      </li>
                    ))}
                  </ol>
                </article>
              )}
            </For>
          </section>
        )}
      </For>
    </section>
  );
}
