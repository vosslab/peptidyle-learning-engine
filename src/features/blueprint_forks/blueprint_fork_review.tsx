// Lazy, ordinary-visibility Blueprint fork discovery and current Revision review.

import { A } from "@solidjs/router";
import {
  For,
  Show,
  createResource,
  createSignal,
  createUniqueId,
  onMount,
  type JSX,
} from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintComparisonSide } from "../../../generated/api/BlueprintComparisonSide";
import type { BlueprintRevision } from "../../../generated/api/BlueprintRevision";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import { normalizeHumanEnteredPublicId } from "../../question_id";
import { BlueprintForkApply } from "./blueprint_fork_apply";
import {
  assessmentDifferenceLabels,
  forkModuleLabel,
  readableSettingName,
  sameForkSnapshot,
  settingLines,
} from "./blueprint_fork_model";
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
    <Show when={props.view.fork_source}>
      {(source) => (
        <section class="blueprint-forks">
          <h2>Fork source</h2>
          <p>
            Forked from{" "}
            <A href={coursePath(source().blueprint_course_id)}>source Blueprint Course</A>, Revision{" "}
            {source().revision}. This fork develops independently.
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
              leftBlueprintCourseId={source().blueprint_course_id}
              hasUnsavedChanges={props.hasUnsavedChanges}
              onApplied={props.onApplied}
            />
          </Show>
        </section>
      )}
    </Show>
  );
}

function coursePath(reference: string): string {
  // ASVS 1.2.2: only a local fixed route with an encoded public ID is constructed.
  return `/blueprint-courses/${encodeURIComponent(reference)}`;
}

export function Settings(props: { readonly value: unknown }): JSX.Element {
  return (
    <dl class="blueprint-fork-settings">
      <For each={settingLines(props.value)}>
        {(line) => (
          <>
            <dt>{line.label}</dt>
            <dd>{line.value}</dd>
          </>
        )}
      </For>
    </dl>
  );
}

export function AssessmentSnapshot(props: {
  readonly snapshot: BlueprintComparisonSide["assessments"][number] | undefined;
  readonly side: BlueprintComparisonSide;
}): JSX.Element {
  return (
    <Show when={props.snapshot} fallback={<p>Not present in this Revision.</p>}>
      {(snapshot) => (
        <>
          <h4>{snapshot().content.title}</h4>
          <p>{assessmentTypePresentation(snapshot().content.assessment_type).label}</p>
          <p>
            Module: {forkModuleLabel(props.side, snapshot().blueprintModuleId)}; Assessment
            position {snapshot().position + 1}.
          </p>
          <p class="blueprint-fork-instructions">
            {snapshot().content.instructions || "No instructions."}
          </p>
          <p>Question IDs in this Assessment: {snapshot().questionIds.join(", ") || "None"}.</p>
          <h5>Questions and Pools in authored order</h5>
          <Show
            when={snapshot().content.entries.length > 0}
            fallback={<p>No Questions or Pools.</p>}
          >
            <ol>
              <For each={snapshot().content.entries}>
                {(entry) => (
                  <li>
                    <Show when={entry.kind === "fixed" ? entry : undefined}>
                      {(fixed) => (
                        <p>
                          Question {fixed().published_question.questionId}, Revision{" "}
                          {fixed().published_question.revisionNumber}; {fixed().points_possible}{" "}
                          points.
                        </p>
                      )}
                    </Show>
                    <Show when={entry.kind === "pool" ? entry : undefined}>
                      {(pool) => (
                        <p>
                          Question Pool {pool().question_pool_id}, Edit{" "}
                          {pool().question_pool_edit_number}; select {pool().selection_count};{" "}
                          {pool().points_per_item} points per Question.
                        </p>
                      )}
                    </Show>
                    <Settings
                      value={{
                        scoring_rule: entry.scoring_rule,
                        question_attempt_limit: entry.question_attempt_limit,
                        question_attempt_time_limit: entry.question_attempt_time_limit,
                        ...(entry.kind === "pool" ? { selection_rule: entry.selection_rule } : {}),
                      }}
                    />
                  </li>
                )}
              </For>
            </ol>
          </Show>
          <h5>Assessment Properties</h5>
          <Settings value={snapshot().content.defaults} />
        </>
      )}
    </Show>
  );
}

function Comparison(props: { readonly view: BlueprintComparisonView }): JSX.Element {
  const assessment = (
    side: BlueprintComparisonSide,
    reference: string,
  ): BlueprintComparisonSide["assessments"][number] | undefined =>
    side.assessments.find((item) => item.blueprintAssessmentId === reference);
  const sideOnly = (side: "left" | "right"): BlueprintComparisonSide["assessments"] =>
    props.view[side].assessments.filter(
      (item) =>
        !props.view.assessmentRelationships.some(
          (edge) =>
            (side === "left" ? edge.leftAssessmentId : edge.rightAssessmentId) ===
            item.blueprintAssessmentId,
        ),
    );
  return (
    <div data-blueprint-fork-comparison>
      <div class="blueprint-fork-columns blueprint-fork-heads">
        <For each={["left", "right"] as const}>
          {(key) => (
            <section>
              <h3>{key === "left" ? "Left" : "Right"}: latest saved Revision</h3>
              <A href={coursePath(props.view[key].currentRevision.blueprint_course_id)}>
                {props.view[key].names.longName}
              </A>
              <p>
                {props.view[key].names.shortName};{" "}
                {props.view[key].currentRevision.blueprint_course_id}; Revision{" "}
                {props.view[key].currentRevision.revision}.
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
        and local references do not establish identity. Question Revision pins and Pool membership
        do not expose Question bodies; body differences are not determined here.
      </p>
      <h3>Question IDs</h3>
      <p>Shared: {props.view.sharedQuestionIds.join(", ") || "None"}.</p>
      <p>
        Removed from left to right (left only):{" "}
        {props.view.leftOnlyQuestionIds.join(", ") || "None"}.
      </p>
      <p>
        Added from left to right (right only):{" "}
        {props.view.rightOnlyQuestionIds.join(", ") || "None"}.
      </p>
      <h3>Assessments related by shared Questions</h3>
      <For
        each={props.view.assessmentRelationships}
        fallback={<p>No Assessments share Question IDs.</p>}
      >
        {(edge) => (
          <details class="blueprint-fork-item">
            <summary>
              {assessment(props.view.left, edge.leftAssessmentId)?.content.title} compared with{" "}
              {assessment(props.view.right, edge.rightAssessmentId)?.content.title}
            </summary>
            <p>Shared Question IDs: {edge.sharedQuestionIds.join(", ")}.</p>
            <Show when={assessment(props.view.left, edge.leftAssessmentId)}>
              {(left) => (
                <Show when={assessment(props.view.right, edge.rightAssessmentId)}>
                  {(right) => (
                    <p>
                      {assessmentDifferenceLabels(
                        left(),
                        right(),
                        props.view.left,
                        props.view.right,
                      )}
                    </p>
                  )}
                </Show>
              )}
            </Show>
            <div class="blueprint-fork-columns">
              <section>
                <h4>Left Assessment</h4>
                <AssessmentSnapshot
                  snapshot={assessment(props.view.left, edge.leftAssessmentId)}
                  side={props.view.left}
                />
              </section>
              <section>
                <h4>Right Assessment</h4>
                <AssessmentSnapshot
                  snapshot={assessment(props.view.right, edge.rightAssessmentId)}
                  side={props.view.right}
                />
              </section>
            </div>
          </details>
        )}
      </For>
      <For each={["left", "right"] as const}>
        {(key) => (
          <section>
            <h3>
              {key === "left"
                ? "Removed from left to right: left-only Assessments"
                : "Added from left to right: right-only Assessments"}
            </h3>
            <p>
              These Assessments have no shared-Question relationship on the other side; this does
              not establish their history.
            </p>
            <For each={sideOnly(key)} fallback={<p>None.</p>}>
              {(item) => (
                <details class="blueprint-fork-item">
                  <summary>{item.content.title}</summary>
                  <AssessmentSnapshot snapshot={item} side={props.view[key]} />
                </details>
              )}
            </For>
          </section>
        )}
      </For>
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

/** Authorized rows only; source head is compared to source ancestry, never fork numbering. */
export function BlueprintKnownForks(
  props: ForkProps & { readonly sourceCurrentRevision: BlueprintRevision },
): JSX.Element {
  const [selectedFork, setSelectedFork] = createSignal<string>();
  const comparisonId = createUniqueId();
  let comparisonTrigger: HTMLButtonElement | undefined;
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
      <Show when={!forks.loading && forks()}>
        {(items) => (
          <Show
            when={items().length > 0}
            fallback={<p>No known forks are visible through your current access.</p>}
          >
            <ul class="blueprint-known-forks-list">
              <For each={items()}>
                {(fork) => (
                  <li>
                    <div>
                      <h3>{fork.longName}</h3>
                      <p>
                        {fork.shortName}; {readableSettingName(fork.availability)}; fork Revision{" "}
                        {fork.currentRevision}.
                      </p>
                      <p>Created from source Revision {fork.sourceRevision}.</p>
                      <Show
                        when={BigInt(props.sourceCurrentRevision) > BigInt(fork.sourceRevision)}
                      >
                        <p>The source has Revisions since this fork was created.</p>
                      </Show>
                      <Show when={BigInt(fork.currentRevision) > 1n}>
                        <p>This fork has saved changes since it was created.</p>
                      </Show>
                      <p>Owning Instructor: {fork.ownerDisplayName}</p>
                    </div>
                    <A
                      class="quiet-link"
                      href={coursePath(fork.id)}
                      aria-label={`Open fork: ${fork.longName}`}
                    >
                      Open fork
                    </A>
                    <button
                      class="quiet-action"
                      type="button"
                      aria-label={`Compare fork with source: ${fork.longName}`}
                      aria-expanded={selectedFork() === fork.id}
                      aria-controls={selectedFork() === fork.id ? comparisonId : undefined}
                      onClick={(event) => {
                        comparisonTrigger = event.currentTarget;
                        setSelectedFork(fork.id);
                      }}
                    >
                      Compare with source
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        )}
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
  const reviewId = (): string => props.blueprintCourseId ?? uniqueId;
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
            <Comparison view={loaded.comparison} />
            <Show
              when={
                !props.hasUnsavedChanges &&
                loaded.target?.read_access === "blueprint_course_owner" &&
                loaded.target.availability !== "archived" &&
                loaded.target.fork_source?.blueprint_course_id ===
                  loaded.comparison.left.currentRevision.blueprint_course_id
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
