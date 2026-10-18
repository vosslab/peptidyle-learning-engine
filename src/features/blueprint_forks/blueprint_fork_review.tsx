// Lazy, ordinary-visibility Blueprint fork discovery and current Revision review.

import { A } from "@solidjs/router";
import {
  createMemo,
  For,
  Show,
  createEffect,
  createResource,
  createSignal,
  createUniqueId,
  onMount,
  type JSX,
} from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintKnownForkView } from "../../../generated/api/BlueprintKnownForkView";
import type { BlueprintRevisionNumber } from "../../../generated/api/BlueprintRevisionNumber";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { comparisonRecognitionIds, recognizedIdList } from "../recognition_label";
import { recognitionTitlesResource } from "../recognition_titles_load";
import { normalizeHumanEnteredPublicId } from "../../question_id";
import type { RecordContent, RecordFact } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/record_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { BlueprintForkApply } from "./blueprint_fork_apply";
import { ComparisonMembershipSummary } from "./blueprint_comparison_membership";
import { readableSettingName, sameForkSnapshot } from "./blueprint_fork_model";
import "./blueprint_fork_review.css";

interface ForkProps {
  readonly client: BlueprintCourseClient;
  readonly blueprintCourseId: string;
  readonly onApplied?: () => void;
  readonly hasUnsavedChanges?: boolean;
}

/** Ancestry is shown only when the ordinary detail reader authorized its source. */
export function BlueprintForkSource(props: {
  readonly client: BlueprintCourseClient;
  readonly view: BlueprintCourseView;
  readonly hasUnsavedChanges: boolean;
  readonly onApplied: () => void;
}): JSX.Element {
  const [comparing, setComparing] = createSignal(false);
  return (
    <Show when={props.view.fork_source_tuple}>
      {(source) => (
        <section class="blueprint-forks">
          <h2>Fork source</h2>
          <p>
            Forked from <A href={coursePath(source().blueprintCourseId)}>source Blueprint Course</A>
            , Revision {source().revisionNumber}. This fork develops independently.
          </p>
          <button
            type="button"
            class="quiet-action"
            aria-expanded={comparing()}
            onClick={() => setComparing(!comparing())}
          >
            {comparing() ? "Close comparison" : "Compare with source"}
          </button>
          <Show when={comparing()}>
            <BlueprintForkReview
              client={props.client}
              blueprintCourseId={props.view.id}
              leftBlueprintCourseId={source().blueprintCourseId}
              hasUnsavedChanges={props.hasUnsavedChanges}
              onApplied={props.onApplied}
            />
          </Show>
        </section>
      )}
    </Show>
  );
}

function coursePath(blueprintCourseId: string): string {
  // ASVS 1.2.2: only a local fixed route with an encoded public ID is constructed.
  return `/blueprint-courses/${encodeURIComponent(blueprintCourseId)}`;
}

export { Settings, AssessmentSnapshot } from "./blueprint_frozen_collections";
import {
  AssessmentSnapshot,
  frozenComparisonRows,
  type FrozenComparisonRow,
  type FrozenComparisonKind,
} from "./blueprint_frozen_collections";

function Comparison(props: {
  readonly client: BlueprintCourseClient;
  readonly view: BlueprintComparisonView;
}): JSX.Element {
  const recognition = recognitionTitlesResource(
    () => props.client,
    () => comparisonRecognitionIds(props.view),
  );
  const rows = createMemo(() => frozenComparisonRows(props.view, recognition.titles()));
  const [selected, setSelected] = createSignal<{
    readonly kind: FrozenComparisonKind;
    readonly id: string;
  }>();
  const detailId = createUniqueId();
  let detailHeading: HTMLHeadingElement | undefined;
  let inspectTrigger: HTMLButtonElement | undefined;
  const containers: Partial<Record<FrozenComparisonKind, HTMLDivElement>> = {};
  const selectedRow = (): FrozenComparisonRow | undefined => {
    const current = selected();
    return current ? rows()?.[current.kind].find((row) => row.id === current.id) : undefined;
  };
  function closeInspection(): void {
    const current = selected();
    setSelected(undefined);
    if (inspectTrigger?.isConnected) inspectTrigger.focus();
    else if (current) containers[current.kind]?.focus();
  }
  createEffect(() => {
    if (selected() && !selectedRow()) closeInspection();
  });
  const fields: ReadonlyArray<
    RecordField<FrozenComparisonRow, "title" | "placement", "differences">
  > = [
    {
      id: "title",
      label: "Assessment",
      role: "identity",
      sortKey: "title",
      value: (row) => row.title,
      defaultColumn: { priority: 0, width: "22rem" },
    },
    {
      id: "placement",
      label: "Saved placement",
      sortKey: "placement",
      value: (row) => row.placement,
      defaultColumn: { priority: 1, width: "20rem" },
    },
    {
      id: "shared",
      label: "Shared Questions",
      value: (row) => row.sharedQuestions,
      defaultColumn: { priority: 2, width: "18rem" },
    },
    {
      id: "differences",
      label: "Comparison",
      filterKey: "differences",
      value: (row) => row.differences,
      defaultColumn: { priority: 3, width: "22rem" },
    },
  ];
  function content(row: FrozenComparisonRow): RecordContent {
    return {
      title: row.title,
      details: [
        { kind: "text", label: "Saved placement", value: row.placement },
        { kind: "text", label: "Shared Questions", value: row.sharedQuestions },
        { kind: "text", label: "Comparison", value: row.differences },
      ],
      actions: [
        {
          id: "inspect",
          kind: "command",
          label: "Inspect",
          title: `Inspect ${row.title}`,
          expanded: selected()?.id === row.id,
          controls: selected()?.id === row.id ? detailId : undefined,
          onClick: (event): void => {
            inspectTrigger = event.currentTarget;
            setSelected({ kind: row.kind, id: row.id });
            queueMicrotask(() => detailHeading?.focus());
          },
        },
      ],
    };
  }
  const options = {
    fields,
    rowId: (row: FrozenComparisonRow): string => row.id,
    content,
    keyword: (row: FrozenComparisonRow): string =>
      `${row.title} ${row.placement} ${row.sharedQuestions} ${row.differences}`,
  };
  const related = createLocalCompleteSearch({ rows: () => rows()?.related ?? [] }, options);
  const leftOnly = createLocalCompleteSearch({ rows: () => rows()?.left ?? [] }, options);
  const rightOnly = createLocalCompleteSearch({ rows: () => rows()?.right ?? [] }, options);
  return (
    <div data-blueprint-fork-comparison>
      <Show when={recognition.status()}>
        <p role="status">{recognition.status()}</p>
      </Show>
      <div class="blueprint-fork-columns blueprint-fork-heads">
        <For each={["left", "right"] as const}>
          {(key) => (
            <section>
              <h3>{key === "left" ? "Left" : "Right"}: latest saved Revision</h3>
              <A href={coursePath(props.view[key].currentRevisionTuple.blueprintCourseId)}>
                {props.view[key].names.longName}
              </A>
              <p>
                {props.view[key].names.shortName};{" "}
                {props.view[key].currentRevisionTuple.blueprintCourseId}; Revision{" "}
                {props.view[key].currentRevisionTuple.revisionNumber}.
              </p>
              <h4>Module structure in authored order</h4>
              <For each={props.view[key].modules} fallback={<p>No Modules.</p>}>
                {(module) => (
                  <p>
                    {module.position + 1}. {module.label} (
                    {
                      props.view[key].assessments.filter(
                        (item) => item.blueprintModuleId === module.blueprintModuleId,
                      ).length
                    }{" "}
                    Assessments)
                  </p>
                )}
              </For>
            </section>
          )}
        </For>
      </div>
      <p>
        {sameForkSnapshot(props.view.left.names, props.view.right.names)
          ? "Current course names match."
          : "Current course names differ."}{" "}
        Names are current metadata, not historical Revision names.
      </p>
      <p>
        This compares the latest saved Revisions, not a creation baseline or unsaved editor changes.
        Nothing is applied automatically.
      </p>
      <p>
        Shared Question IDs relate Assessments, including splits and combinations. Assessment names
        and local IDs do not establish identity. Question Revision pins and Pool membership do not
        expose Question bodies; body differences are not determined here.
      </p>
      <ComparisonMembershipSummary view={props.view} />
      <h3>Question IDs</h3>
      <p>
        Shared:{" "}
        {recognizedIdList(
          props.view.sharedQuestionIds,
          recognition.titles().questions,
          "Question",
          "Question ID",
        )}
        .
      </p>
      <p>
        Removed from left to right (left only):{" "}
        {recognizedIdList(
          props.view.leftOnlyQuestionIds,
          recognition.titles().questions,
          "Question",
          "Question ID",
        )}
        .
      </p>
      <p>
        Added from left to right (right only):{" "}
        {recognizedIdList(
          props.view.rightOnlyQuestionIds,
          recognition.titles().questions,
          "Question",
          "Question ID",
        )}
        .
      </p>
      <div
        tabindex="-1"
        ref={(element) => {
          containers.related = element;
        }}
      >
        <h3>Assessments related by shared Questions</h3>
        <SearchBlock
          layout="above"
          controls={
            <SearchControls
              state={related.state}
              textLabel="Related Assessment or shared Question"
            />
          }
          state={related.state}
          collection={{
            ariaLabel: "Assessments related by shared Questions",
            fields,
            query: related.query,
            emptyState: { title: "No Assessments share Question IDs or match your search." },
          }}
        />
      </div>
      <p>
        Left-only and right-only Assessments have no shared-Question relationship on the other side;
        this does not establish their history.
      </p>
      <div
        tabindex="-1"
        ref={(element) => {
          containers.left = element;
        }}
      >
        <h3>Removed from left to right: left-only Assessments</h3>
        <SearchBlock
          layout="above"
          controls={
            <SearchControls state={leftOnly.state} textLabel="Left-only Assessment or Module" />
          }
          state={leftOnly.state}
          collection={{
            ariaLabel: "Left-only Assessments",
            fields,
            query: leftOnly.query,
            emptyState: { title: "No left-only Assessments match." },
          }}
        />
      </div>
      <div
        tabindex="-1"
        ref={(element) => {
          containers.right = element;
        }}
      >
        <h3>Added from left to right: right-only Assessments</h3>
        <SearchBlock
          layout="above"
          controls={
            <SearchControls state={rightOnly.state} textLabel="Right-only Assessment or Module" />
          }
          state={rightOnly.state}
          collection={{
            ariaLabel: "Right-only Assessments",
            fields,
            query: rightOnly.query,
            emptyState: { title: "No right-only Assessments match." },
          }}
        />
      </div>
      <Show when={selectedRow()}>
        {(row) => (
          <section id={detailId} aria-labelledby={`${detailId}-heading`}>
            <h3
              id={`${detailId}-heading`}
              tabindex="-1"
              ref={(element) => {
                detailHeading = element;
              }}
            >
              {row().title}
            </h3>
            <button type="button" class="quiet-action" onClick={closeInspection}>
              Close inspection
            </button>
            <p>{row().differences}</p>
            <Show when={row().kind === "related"}>
              <p>Shared Question IDs: {row().sharedQuestions}.</p>
            </Show>
            <div class="blueprint-fork-columns">
              <Show when={row().left}>
                <section>
                  <h4>Left Assessment</h4>
                  <AssessmentSnapshot
                    snapshot={row().left}
                    side={props.view.left}
                    titles={recognition.titles()}
                  />
                </section>
              </Show>
              <Show when={row().right}>
                <section>
                  <h4>Right Assessment</h4>
                  <AssessmentSnapshot
                    snapshot={row().right}
                    side={props.view.right}
                    titles={recognition.titles()}
                  />
                </section>
              </Show>
            </div>
          </section>
        )}
      </Show>
    </div>
  );
}

function RelatedComparison(props: ForkProps): JSX.Element {
  const inputId = createUniqueId();
  const comparisonId = createUniqueId();
  const [relatedBlueprintCourseId, setRelatedBlueprintCourseId] = createSignal("");
  const [selected, setSelected] = createSignal<string>();
  const [invalid, setInvalid] = createSignal(false);
  let input: HTMLInputElement | undefined;
  return (
    <section class="blueprint-forks">
      <h2>Compare a related Blueprint Course</h2>
      <p>
        Enter a visible Blueprint Course ID in the same fork lineage. This Course is left; the
        entered Course is right.
      </p>
      <form
        class="blueprint-related-comparison-form"
        onSubmit={(event) => {
          event.preventDefault();
          const candidate = normalizeHumanEnteredPublicId(
            "blueprintCourse",
            relatedBlueprintCourseId(),
          );
          if (candidate === null || candidate === props.blueprintCourseId) {
            setInvalid(true);
            input?.focus();
            return;
          }
          setInvalid(false);
          setSelected(candidate);
        }}
      >
        <label for={inputId}>Related Blueprint Course ID</label>
        <input
          id={inputId}
          ref={(element) => {
            input = element;
          }}
          value={relatedBlueprintCourseId()}
          maxlength={8}
          required
          aria-invalid={invalid()}
          aria-describedby={invalid() ? `${inputId}-error` : undefined}
          onInput={(event) => setRelatedBlueprintCourseId(event.currentTarget.value)}
        />
        <button type="submit" aria-controls={selected() ? comparisonId : undefined}>
          Compare latest Revisions
        </button>
      </form>
      <Show when={invalid()}>
        <p id={`${inputId}-error`} role="alert">
          Enter a different Blueprint Course ID in BPXXXXXXXZ format.
        </p>
      </Show>
      <Show when={selected()} keyed>
        {(right) => (
          <>
            <button
              type="button"
              class="quiet-action"
              onClick={() => {
                setSelected(undefined);
                input?.focus();
              }}
            >
              Close comparison
            </button>
            <BlueprintForkReview
              client={props.client}
              leftBlueprintCourseId={props.blueprintCourseId}
              blueprintCourseId={right}
              id={comparisonId}
              hasUnsavedChanges={props.hasUnsavedChanges}
              onApplied={props.onApplied}
            />
          </>
        )}
      </Show>
    </section>
  );
}

function knownForkContent(
  fork: BlueprintKnownForkView,
  sourceCurrentRevision: BlueprintRevisionNumber,
  comparisonId: string,
  selectedForkId: string | undefined,
  setComparisonTrigger: (element: HTMLButtonElement) => void,
  selectFork: (forkId: string) => void,
): RecordContent {
  const sourceHasChanged =
    BigInt(sourceCurrentRevision) > BigInt(fork.sourceRevisionTuple.revisionNumber);
  const forkHasChanged = BigInt(fork.currentRevisionTuple.revisionNumber) > 1n;
  const details: Array<RecordFact> = [
    { kind: "text", label: "Short name", value: fork.shortName },
    { kind: "text", label: "Availability", value: readableSettingName(fork.availability) },
    {
      kind: "text",
      label: "Fork Revision",
      value: String(fork.currentRevisionTuple.revisionNumber),
    },
    {
      kind: "text",
      label: "Source Revision",
      value: String(fork.sourceRevisionTuple.revisionNumber),
    },
    { kind: "text", label: "Owning Instructor", value: fork.ownerDisplayName },
  ];
  if (sourceHasChanged) {
    details.push({
      kind: "text",
      label: "Source changes",
      value: "The source has Revisions since this fork was created.",
    });
  }
  if (forkHasChanged) {
    details.push({
      kind: "text",
      label: "Fork changes",
      value: "This fork has saved changes since it was created.",
    });
  }
  return {
    title: fork.longName,
    details,
    actions: [
      {
        id: "open-fork",
        kind: "link",
        label: "Open",
        href: coursePath(fork.id),
        title: `Open fork: ${fork.longName}`,
        primary: true,
      },
      {
        id: "compare-with-source",
        kind: "command",
        label: "Compare with source",
        title: `Compare fork with source: ${fork.longName}`,
        expanded: selectedForkId === fork.id,
        controls: selectedForkId === fork.id ? comparisonId : undefined,
        onClick: (event: MouseEvent & { readonly currentTarget: HTMLButtonElement }): void => {
          setComparisonTrigger(event.currentTarget);
          selectFork(fork.id);
        },
      },
    ],
  };
}

/** Authorized rows only; source head is compared to source ancestry, never fork numbering. */
export function BlueprintKnownForks(
  props: ForkProps & { readonly sourceCurrentRevision: BlueprintRevisionNumber },
): JSX.Element {
  const [selectedFork, setSelectedFork] = createSignal<string>();
  const comparisonId = createUniqueId();
  let comparisonTrigger: HTMLButtonElement | undefined;
  const forkContent = (fork: BlueprintKnownForkView): RecordContent =>
    knownForkContent(
      fork,
      props.sourceCurrentRevision,
      comparisonId,
      selectedFork(),
      (element: HTMLButtonElement): void => {
        comparisonTrigger = element;
      },
      setSelectedFork,
    );
  const [forks, { refetch }] = createResource(
    () => props.blueprintCourseId,
    async (blueprintCourseId) => {
      try {
        return await props.client.listKnownBlueprintForks(blueprintCourseId);
      } catch {
        return null;
      } // ASVS 16.5.1: access loss and unavailable source share a neutral state.
    },
  );
  const fields: ReadonlyArray<
    RecordField<BlueprintKnownForkView, "name" | "owner" | "revision", "availability">
  > = [
    {
      id: "name",
      label: "Name",
      role: "identity",
      sortKey: "name",
      value: (fork) => fork.longName,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "owner",
      label: "Owning Instructor",
      sortKey: "owner",
      value: (fork) => fork.ownerDisplayName,
      defaultColumn: { priority: 1, width: "14rem" },
    },
    {
      id: "availability",
      label: "Availability",
      filterKey: "availability",
      value: (fork) => fork.availability,
      defaultColumn: { priority: 2, width: "9rem" },
    },
    {
      id: "revision",
      label: "Fork Revision",
      sortKey: "revision",
      value: (fork) => fork.currentRevisionTuple.revisionNumber,
      defaultColumn: { priority: 3, width: "8rem" },
    },
  ];
  const search = createLocalCompleteSearch(
    {
      rows: () => {
        const value = forks();
        return value === undefined ? undefined : (value ?? []);
      },
      loading: () => forks.loading,
      error: (): unknown => forks.error,
      refetchSource: refetch,
    },
    {
      fields,
      rowId: (fork) => fork.id,
      keyword: (fork) => `${fork.longName} ${fork.shortName} ${fork.ownerDisplayName} ${fork.id}`,
      content: forkContent,
      compare: (key, left, right) =>
        key === "revision"
          ? BigInt(left.currentRevisionTuple.revisionNumber) <
            BigInt(right.currentRevisionTuple.revisionNumber)
            ? -1
            : BigInt(left.currentRevisionTuple.revisionNumber) >
                BigInt(right.currentRevisionTuple.revisionNumber)
              ? 1
              : 0
          : undefined,
    },
  );
  return (
    <section
      class="blueprint-forks"
      aria-labelledby="blueprint-known-forks-heading"
      aria-busy={forks.loading}
    >
      <RelatedComparison
        client={props.client}
        blueprintCourseId={props.blueprintCourseId}
        hasUnsavedChanges={props.hasUnsavedChanges}
        onApplied={props.onApplied}
      />
      <h2 id="blueprint-known-forks-heading">Known forks</h2>
      <Show when={forks.loading}>
        <p role="status">Loading visible forks...</p>
      </Show>
      <Show when={!forks.loading && forks() === null}>
        <p role="alert">Known forks are unavailable through your current access.</p>
        <button type="button" onClick={() => void refetch()}>
          Retry loading known forks
        </button>
      </Show>
      <Show when={forks() !== null}>
        <SearchBlock
          layout="above"
          controls={<SearchControls state={search.state} textLabel="Fork name or Instructor" />}
          state={search.state}
          collection={{
            ariaLabel: "Known Blueprint forks",
            fields,
            query: search.query,
            emptyState: { title: "No known forks match." },
          }}
        />
      </Show>
      <Show when={selectedFork()} keyed>
        {(forkId) => (
          <>
            <button
              class="quiet-action"
              type="button"
              onClick={() => {
                setSelectedFork(undefined);
                comparisonTrigger?.focus();
              }}
            >
              Close comparison
            </button>
            <BlueprintForkReview
              client={props.client}
              blueprintCourseId={forkId}
              leftBlueprintCourseId={props.blueprintCourseId}
              id={comparisonId}
              hasUnsavedChanges={props.hasUnsavedChanges}
              onApplied={props.onApplied}
            />
          </>
        )}
      </Show>
    </section>
  );
}

export function BlueprintForkReview(
  props: ForkProps & { readonly leftBlueprintCourseId: string; readonly id?: string },
): JSX.Element {
  let heading: HTMLHeadingElement | undefined;
  const uniqueId = createUniqueId();
  const reviewId = (): string => props.id ?? uniqueId;
  onMount(() => heading?.focus());
  // Reactive pair owns this request; pending hides stale data when either Blueprint Course ID changes.
  const [review, { refetch }] = createResource(
    () => ({ left: props.leftBlueprintCourseId, right: props.blueprintCourseId }),
    async (pair) => {
      try {
        const [comparison, target] = await Promise.all([
          props.client.getBlueprintComparison(pair.left, pair.right),
          // Ordinary right-side detail access alone supplies the ownership gate.
          props.client.getBlueprintCourse(pair.right).catch(() => null),
        ]);
        return { comparison, target: target?.blueprintCourse ?? null };
      } catch {
        return null;
      } // ASVS 16.5.1: hidden, unrelated and unavailable pairs share a neutral state.
    },
  );
  return (
    <section
      id={reviewId()}
      class="blueprint-forks"
      aria-labelledby={`${reviewId()}-heading`}
      aria-busy={review.loading}
    >
      <div class="blueprint-fork-review-heading">
        <h2
          id={`${reviewId()}-heading`}
          tabindex="-1"
          ref={(element) => {
            heading = element;
          }}
        >
          Current Blueprint Course comparison
        </h2>
        <button
          class="quiet-action"
          type="button"
          disabled={review.loading}
          onClick={() => void refetch()}
        >
          Refresh comparison
        </button>
      </div>
      <Show when={review.loading}>
        <p role="status">Loading latest saved Revisions...</p>
      </Show>
      <Show when={!review.loading && review() === null}>
        <p role="alert">Comparison is unavailable through your current access.</p>
        <button type="button" onClick={() => void refetch()}>
          Retry comparison
        </button>
      </Show>
      {/* ASVS 1.2.1: authored content is escaped JSX text, never interpreted as HTML. */}
      <Show when={!review.loading && review()} keyed>
        {(loaded) => (
          <>
            <Comparison client={props.client} view={loaded.comparison} />
            <Show
              when={
                !props.hasUnsavedChanges &&
                loaded.target?.read_access === "blueprint_course_owner" &&
                loaded.target.availability !== "archived" &&
                loaded.target.fork_source_tuple?.blueprintCourseId ===
                  loaded.comparison.left.currentRevisionTuple.blueprintCourseId
              }
              fallback={
                <p role="status">
                  Read-only comparison. Applying source changes requires a saved editor and
                  ownership of an active direct fork on the right.
                </p>
              }
            >
              <BlueprintForkApply
                client={props.client}
                review={loaded.comparison}
                onApplied={() => {
                  // Refresh both saved Revisions and ordinary target detail; keyed rendering
                  // discards choices made against the previous comparison.
                  void refetch();
                  props.onApplied?.();
                }}
              />
            </Show>
            <Show when={props.hasUnsavedChanges}>
              <p>Your unsaved editor changes are not included in this comparison.</p>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}
