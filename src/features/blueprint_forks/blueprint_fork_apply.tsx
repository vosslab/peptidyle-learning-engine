import { For, Show, createMemo, createSignal, type JSX } from "solid-js";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import {
  RecordOutlineItem,
  RecordOutlineList,
} from "../../components/record_list/record_outline_list";
import type { RecordContent } from "../../components/record_list/record_list";
import { reorderedRecordListRows } from "../../components/record_list/record_list_reorder";
import { RecordSequence } from "../../components/record_list/record_sequence";
import {
  currentForkLayout,
  destinationKey,
  forkSelectionProblem,
  type Layout,
} from "./blueprint_fork_apply_model";
import "./blueprint_fork_apply.css";
type ApplyProps = {
  readonly client?: BlueprintCourseClient;
  readonly review: BlueprintComparisonView;
  readonly onApplied: () => void;
  readonly proposalSelection?: {
    readonly classificationControl: JSX.Element;
    readonly hasClassificationSelection: () => boolean;
    readonly disabled?: boolean;
    readonly onChanged?: () => void;
    readonly onReset?: () => void;
    readonly submit: (
      selection: BlueprintForkApplySelection,
      shortName: boolean,
      longName: boolean,
    ) => Promise<void>;
  };
};
/** Parent gates ownership and dirty work; authorization remains server-owned. */
export function BlueprintForkApply(props: ApplyProps): JSX.Element {
  return <BlueprintSelectionEditor {...props} />;
}

/** Input-driven inventory/layout controls; proposal callers supply their own review action. */
export function BlueprintSelectionEditor(props: ApplyProps): JSX.Element {
  const [shortName, setShortName] = createSignal(false),
    [longName, setLongName] = createSignal(false);
  const [labels, setLabels] = createSignal<BlueprintForkApplySelection["sourceModuleLabels"]>([]);
  const [contents, setContents] = createSignal<BlueprintForkApplySelection["sourceAssessments"]>(
    [],
  );
  const [layout, setLayout] = createSignal(currentForkLayout(props.review)),
    [edited, setEdited] = createSignal(false);
  const [busy, setBusy] = createSignal(false),
    [locked, setLocked] = createSignal(false),
    [message, setMessage] = createSignal("");
  const problem = createMemo(() =>
    forkSelectionProblem(props.review, layout(), labels(), contents()),
  );
  const selected = (): boolean =>
    shortName() ||
    longName() ||
    labels().length > 0 ||
    contents().length > 0 ||
    edited() ||
    Boolean(props.proposalSelection?.hasClassificationSelection());
  const moduleLabel = (module: Layout["module"]): string => {
    if (module.kind === "newFromSource")
      return (
        props.review.left.modules.find((m) => m.blueprintModuleId === module.sourceModuleId)
          ?.label ?? "New module"
      );
    const copy = labels().find((c) => c.targetModuleId === module.targetModuleId);
    return (
      (copy
        ? props.review.left.modules.find((m) => m.blueprintModuleId === copy.sourceModuleId)?.label
        : props.review.right.modules.find((m) => m.blueprintModuleId === module.targetModuleId)
            ?.label) ?? "Module"
    );
  };
  const title = (entry: Layout["assessments"][number]): string => {
    if (entry.kind === "newFromSource")
      return (
        props.review.left.assessments.find(
          (a) => a.blueprintAssessmentId === entry.sourceAssessmentId,
        )?.content.title ?? "New Assessment"
      );
    const copy = contents().find((c) => c.targetAssessmentId === entry.targetAssessmentId);
    return (
      (copy
        ? props.review.left.assessments.find(
            (a) => a.blueprintAssessmentId === copy.sourceAssessmentId,
          )?.content.title
        : props.review.right.assessments.find(
            (a) => a.blueprintAssessmentId === entry.targetAssessmentId,
          )?.content.title) ?? "Assessment"
    );
  };
  function edit(next: Layout[]): void {
    props.proposalSelection?.onChanged?.();
    setLayout(next);
    setEdited(true);
    setMessage("");
  }
  function reorderDestinationAssessments(
    moduleKey: string,
    sourceIndex: number,
    destinationIndex: number,
  ): void {
    edit(
      layout().map((row) =>
        destinationKey(row.module) === moduleKey
          ? {
              ...row,
              assessments: reorderedRecordListRows(row.assessments, sourceIndex, destinationIndex),
            }
          : row,
      ),
    );
  }
  function place(entry: Layout["assessments"][number], key: string): void {
    edit(
      layout().map((row) => ({
        ...row,
        assessments: [
          ...row.assessments.filter((a) => destinationKey(a) !== destinationKey(entry)),
          ...(destinationKey(row.module) === key ? [entry] : []),
        ],
      })),
    );
  }
  function chooseModule(source: string, value: string): void {
    props.proposalSelection?.onChanged?.();
    setLabels([
      ...labels().filter((c) => c.sourceModuleId !== source),
      ...(value
        ? [{ sourceModuleId: source, targetModuleId: value === "new" ? null : value }]
        : []),
    ]);
    const next = layout().filter(
      (row) => !(row.module.kind === "newFromSource" && row.module.sourceModuleId === source),
    );
    if (value === "new")
      next.push({
        module: { kind: "newFromSource", sourceModuleId: source },
        assessments: [],
      });
    if (next.length !== layout().length || value === "new") edit(next);
  }
  function chooseAssessment(source: string, value: string): void {
    props.proposalSelection?.onChanged?.();
    setContents([
      ...contents().filter((c) => c.sourceAssessmentId !== source),
      ...(value
        ? [
            {
              sourceAssessmentId: source,
              targetAssessmentId: value === "new" ? null : value,
            },
          ]
        : []),
    ]);
    if (
      value !== "new" &&
      layout().some((row) =>
        row.assessments.some((a) => a.kind === "newFromSource" && a.sourceAssessmentId === source),
      )
    )
      place({ kind: "newFromSource", sourceAssessmentId: source }, "");
  }
  function reset(): void {
    props.proposalSelection?.onChanged?.();
    props.proposalSelection?.onReset?.();
    setShortName(false);
    setLongName(false);
    setLabels([]);
    setContents([]);
    setLayout(currentForkLayout(props.review));
    setEdited(false);
    setMessage("Selections cancelled. Nothing was sent.");
  }
  async function save(): Promise<void> {
    if (busy() || locked() || problem() || !selected()) return;
    setBusy(true);
    setMessage("");
    try {
      if (props.proposalSelection) {
        await props.proposalSelection.submit(
          {
            sourceModuleLabels: labels(),
            sourceAssessments: contents(),
            layout: edited() ? layout() : null,
          },
          shortName(),
          longName(),
        );
        return;
      }
      if (!props.client) throw new Error("Fork client is unavailable.");
      await props.client.applyBlueprintFork(
        props.review.right.currentRevisionTuple.blueprintCourseId,
        {
          expectedSourceRevisionTuple: props.review.left.currentRevisionTuple,
          expectedForkRevisionTuple: props.review.right.currentRevisionTuple,
          expectedSourceBlueprintEditNumber: props.review.left.blueprintEditNumber,
          expectedForkBlueprintEditNumber: props.review.right.blueprintEditNumber,
          sourceShortName: shortName(),
          sourceLongName: longName(),
          selection: {
            sourceModuleLabels: labels(),
            sourceAssessments: contents(),
            layout: edited() ? layout() : null,
          },
        },
      );
      setLocked(true);
      setMessage("Selected changes saved. Reloading the fork and comparison...");
      props.onApplied();
    } catch (error: unknown) {
      if (error instanceof BlueprintCourseConflictError) {
        setLocked(true);
        setMessage(
          "The source or fork changed. Refresh the comparison and review your choices again.",
        );
      } else if (error instanceof ApiRequestError && (error.status === 400 || error.status === 422))
        setMessage(
          "These choices could not be saved. Correct the destinations and content selections before saving again.",
        );
      else {
        setLocked(true);
        setMessage(
          "The result was not confirmed. Refresh the comparison before trying again; do not repeat this request automatically.",
        );
      }
    } finally {
      setBusy(false);
    }
  }
  const available = createMemo(() =>
    [
      ...props.review.right.assessments.map((a) => ({
        kind: "existing" as const,
        targetAssessmentId: a.blueprintAssessmentId,
      })),
      ...contents()
        .filter((c) => c.targetAssessmentId === null)
        .map((c) => ({
          kind: "newFromSource" as const,
          sourceAssessmentId: c.sourceAssessmentId,
        })),
    ].filter(
      (a) =>
        !layout().some((row) =>
          row.assessments.some((entry) => destinationKey(entry) === destinationKey(a)),
        ),
    ),
  );
  return (
    <section
      class="blueprint-fork-apply"
      aria-label={
        props.proposalSelection ? "Choose proposal changes" : "Apply reviewed changes to your fork"
      }
      aria-busy={busy()}
    >
      <h3>
        {props.proposalSelection
          ? "Choose changes for the receiving Blueprint"
          : "Choose changes for your fork"}
      </h3>
      <p>
        Nothing is selected initially. Choose an existing target explicitly or create a new copy.
        Unselected target work stays in place unless you explicitly remove it from the destination.
      </p>
      <fieldset disabled={busy() || locked() || props.proposalSelection?.disabled}>
        <legend>Source names and complete content</legend>
        {props.proposalSelection?.classificationControl}
        <label>
          <input
            type="checkbox"
            checked={shortName()}
            onChange={(e) => {
              props.proposalSelection?.onChanged?.();
              setShortName(e.currentTarget.checked);
            }}
          />{" "}
          Use source short name: {props.review.left.names.shortName}
        </label>
        <label>
          <input
            type="checkbox"
            checked={longName()}
            onChange={(e) => {
              props.proposalSelection?.onChanged?.();
              setLongName(e.currentTarget.checked);
            }}
          />{" "}
          Use source long name: {props.review.left.names.longName}
        </label>
        <For each={props.review.left.modules}>
          {(source) => (
            <label>
              Source module label: {source.label}
              <select
                value={
                  labels().find((c) => c.sourceModuleId === source.blueprintModuleId)
                    ?.targetModuleId ??
                  (labels().some((c) => c.sourceModuleId === source.blueprintModuleId) ? "new" : "")
                }
                onChange={(e) => chooseModule(source.blueprintModuleId, e.currentTarget.value)}
              >
                <option value="">Do not copy</option>
                <option value="new">Create new module</option>
                <For each={props.review.right.modules}>
                  {(target) => (
                    <option value={target.blueprintModuleId}>Replace label: {target.label}</option>
                  )}
                </For>
              </select>
            </label>
          )}
        </For>
        <For each={props.review.left.assessments}>
          {(source) => (
            <label>
              Complete source Assessment: {source.content.title} (settings, instructions, Questions,
              Pools, pins and scoring)
              <select
                value={
                  contents().find((c) => c.sourceAssessmentId === source.blueprintAssessmentId)
                    ?.targetAssessmentId ??
                  (contents().some((c) => c.sourceAssessmentId === source.blueprintAssessmentId)
                    ? "new"
                    : "")
                }
                onChange={(e) =>
                  chooseAssessment(source.blueprintAssessmentId, e.currentTarget.value)
                }
              >
                <option value="">Do not copy</option>
                <option value="new">Create new Assessment copy</option>
                <For each={props.review.right.assessments}>
                  {(target) => {
                    const shared = (): number =>
                      props.review.assessmentRelationships.find(
                        (r) =>
                          r.leftAssessmentId === source.blueprintAssessmentId &&
                          r.rightAssessmentId === target.blueprintAssessmentId,
                      )?.sharedQuestionIds.length ?? 0;
                    return (
                      <option value={target.blueprintAssessmentId}>
                        Replace content: {target.content.title}
                        {shared() ? ` (${shared()} shared Questions; not identity)` : ""}
                      </option>
                    );
                  }}
                </For>
              </select>
            </label>
          )}
        </For>
        <p>
          Complete content copies Questions and Pools according to the server's copy rules; new
          destination IDs are assigned by the server.
        </p>
        <h4>Complete destination layout</h4>
        <p>
          Move related changes together. Removing a module also excludes its Assessments unless you
          move them first.
        </p>
        <div class="blueprint-fork-destination">
          <RecordOutlineList
            state={{ kind: "ready" }}
            isEmpty={layout().length === 0}
            ariaLabel="Complete destination Module layout"
            emptyState={{
              title: "No destination Modules",
              message: "Restore or create a destination Module before placing an Assessment.",
            }}
            reorder={{
              recordIds: () => layout().map((row) => destinationKey(row.module)),
              onMove: (sourceIndex, destinationIndex): void =>
                edit(reorderedRecordListRows(layout(), sourceIndex, destinationIndex)),
              recordLabel: (moduleKey): string => {
                const row = layout().find(
                  (candidate) => destinationKey(candidate.module) === moduleKey,
                );
                return row === undefined ? "Module" : moduleLabel(row.module);
              },
              isDisabled: (): boolean =>
                busy() || locked() || props.proposalSelection?.disabled === true,
            }}
          >
            <For each={layout().map((row) => destinationKey(row.module))}>
              {(moduleKey) => {
                const row = (): Layout =>
                  layout().find((candidate) => destinationKey(candidate.module) === moduleKey)!;
                return (
                  <RecordOutlineItem recordId={moduleKey}>
                    <h5>{moduleLabel(row().module)}</h5>
                    <div class="blueprint-fork-layout-actions">
                      <button
                        type="button"
                        onClick={() =>
                          edit(
                            layout().filter(
                              (candidate) => destinationKey(candidate.module) !== moduleKey,
                            ),
                          )
                        }
                      >
                        Remove module: {moduleLabel(row().module)}
                      </button>
                    </div>
                    <RecordSequence
                      rows={row().assessments}
                      content={(entry): RecordContent => ({
                        title: title(entry),
                        details: [],
                        actions: [
                          {
                            id: "remove-assessment",
                            kind: "command",
                            label: `Remove Assessment: ${title(entry)}`,
                            disabled:
                              busy() || locked() || props.proposalSelection?.disabled === true,
                            onClick: (): void => place(entry, ""),
                          },
                        ],
                      })}
                      renderBody={(entry) => (
                        <label>
                          Destination for {title(entry())}
                          <select
                            value={moduleKey}
                            onChange={(event) => place(entry(), event.currentTarget.value)}
                          >
                            <For each={layout()}>
                              {(candidateRow) => (
                                <option value={destinationKey(candidateRow.module)}>
                                  {moduleLabel(candidateRow.module)}
                                </option>
                              )}
                            </For>
                          </select>
                        </label>
                      )}
                      recordId={destinationKey}
                      reorder={{
                        onMove: (sourceIndex, destinationIndex): void =>
                          reorderDestinationAssessments(moduleKey, sourceIndex, destinationIndex),
                        recordLabel: (entry): string => title(entry),
                        isDisabled: (): boolean =>
                          busy() || locked() || props.proposalSelection?.disabled === true,
                      }}
                      state={{ kind: "ready" }}
                      ariaLabel={`${moduleLabel(row().module)} destination Assessments`}
                      emptyState={{
                        title: "No Assessments",
                        message:
                          "Place an Assessment in this Module to include it in the destination layout.",
                      }}
                    />
                  </RecordOutlineItem>
                );
              }}
            </For>
          </RecordOutlineList>
        </div>
        <For
          each={props.review.right.modules.filter(
            (m) =>
              !layout().some(
                (r) =>
                  r.module.kind === "existing" && r.module.targetModuleId === m.blueprintModuleId,
              ),
          )}
        >
          {(m) => (
            <button
              type="button"
              onClick={() =>
                edit([
                  ...layout(),
                  {
                    module: { kind: "existing", targetModuleId: m.blueprintModuleId },
                    assessments: [],
                  },
                ])
              }
            >
              Restore module: {m.label}
            </button>
          )}
        </For>
        <For each={available()}>
          {(entry) => (
            <label>
              Add destination for {title(entry)}
              <select
                value=""
                onChange={(e) => {
                  if (e.currentTarget.value) place(entry, e.currentTarget.value);
                }}
              >
                <option value="">Choose module</option>
                <For each={layout()}>
                  {(r) => <option value={destinationKey(r.module)}>{moduleLabel(r.module)}</option>}
                </For>
              </select>
            </label>
          )}
        </For>
      </fieldset>
      <Show when={problem()}>
        <p role="alert">{problem()}</p>
      </Show>
      <Show when={message()}>
        <p role="status">{message()}</p>
      </Show>
      <div class="blueprint-fork-layout-actions">
        <button
          type="button"
          disabled={
            busy() ||
            locked() ||
            Boolean(problem()) ||
            !selected() ||
            props.proposalSelection?.disabled
          }
          onClick={() => void save()}
        >
          {busy()
            ? "Preparing..."
            : props.proposalSelection
              ? "Review chosen result"
              : "Save selected changes to fork"}
        </button>
        <button
          type="button"
          disabled={busy() || props.proposalSelection?.disabled}
          onClick={reset}
        >
          Cancel selections
        </button>
      </div>
    </section>
  );
}
