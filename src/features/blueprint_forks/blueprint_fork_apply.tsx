import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type { RecordReorderBinding } from "../../components/data_sheet/record_reorder";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import {
  destinationAssessmentContent,
  destinationAssessmentFields,
  destinationAssessmentTitle,
  destinationModuleContent,
  destinationModuleFields,
  destinationModuleLabel,
  destinationRecords,
  moveDestinationAssessment,
  moveDestinationModule,
  type DestinationAssessmentRecord,
  type DestinationModuleRecord,
} from "./blueprint_fork_destination_records";
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
  const disabled = (): boolean => busy() || locked() || props.proposalSelection?.disabled === true;
  const moduleLabel = (module: Layout["module"]): string =>
    destinationModuleLabel(props.review, labels(), module);
  const title = (entry: Layout["assessments"][number]): string =>
    destinationAssessmentTitle(props.review, contents(), entry);
  function edit(next: Layout[]): void {
    if (disabled() || next === layout()) return;
    props.proposalSelection?.onChanged?.();
    setLayout(next);
    setEdited(true);
    setMessage("");
  }
  function assessmentLayoutModule(id: string): Layout | undefined {
    return layout().find((module) =>
      module.assessments.some((entry) => destinationKey(entry) === id),
    );
  }
  function reorderDestinationModule(sourceId: string, targetId: string): void {
    if (disabled() || sourceId === targetId) return;
    const current = layout();
    const sourceIndex = current.findIndex((module) => destinationKey(module.module) === sourceId);
    const targetIndex = current.findIndex((module) => destinationKey(module.module) === targetId);
    if (sourceIndex < 0 || targetIndex < 0) return;
    edit(moveDestinationModule(current, sourceId, targetIndex - sourceIndex));
  }
  function reorderDestinationAssessment(sourceId: string, targetId: string): void {
    if (disabled() || sourceId === targetId) return;
    const module = assessmentLayoutModule(sourceId);
    if (module === undefined) return;
    const sourceIndex = module.assessments.findIndex((entry) => destinationKey(entry) === sourceId);
    const targetIndex = module.assessments.findIndex((entry) => destinationKey(entry) === targetId);
    if (sourceIndex < 0 || targetIndex < 0) return;
    edit(
      moveDestinationAssessment(
        layout(),
        destinationKey(module.module),
        sourceId,
        targetIndex - sourceIndex,
      ),
    );
  }
  const moduleReorder: RecordReorderBinding = {
    orderForRecord: (id) => {
      const currentFullAuthoredIds = layout().map((module) => destinationKey(module.module));
      return currentFullAuthoredIds.includes(id) ? currentFullAuthoredIds : [];
    },
    onMove: reorderDestinationModule,
    recordLabel: (id) => {
      const module = layout().find((entry) => destinationKey(entry.module) === id);
      return module === undefined ? "Module" : moduleLabel(module.module);
    },
    isDisabled: disabled,
  };
  const assessmentReorder: RecordReorderBinding = {
    orderForRecord: (id) =>
      assessmentLayoutModule(id)?.assessments.map((entry) => destinationKey(entry)) ?? [],
    onMove: reorderDestinationAssessment,
    recordLabel: (id) => {
      const entry = assessmentLayoutModule(id)?.assessments.find(
        (assessment) => destinationKey(assessment) === id,
      );
      return entry === undefined ? "Assessment" : title(entry);
    },
    isDisabled: disabled,
  };
  function moveDestinationByOffset(
    reorder: RecordReorderBinding,
    id: string,
    offset: -1 | 1,
  ): void {
    const order = reorder.orderForRecord(id);
    const index = order.indexOf(id);
    if (index < 0) return;
    const targetId = order[index + offset];
    if (targetId !== undefined) void reorder.onMove(id, targetId);
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
  const records = createMemo(() =>
    destinationRecords(props.review, layout(), labels(), contents()),
  );
  const [selectedDestination, setSelectedDestination] = createSignal<string>();
  const placementDetailId = createUniqueId();
  let destinationOverview: HTMLDivElement | undefined;
  let destinationHeading: HTMLHeadingElement | undefined;
  let destinationTrigger: HTMLButtonElement | undefined;
  const selectedDestinationRow = (): DestinationAssessmentRecord | undefined =>
    records()?.assessments.find((row) => row.id === selectedDestination());
  function closeDestination(): void {
    setSelectedDestination(undefined);
    if (destinationTrigger?.isConnected) destinationTrigger.focus();
    else destinationOverview?.focus();
  }
  createEffect(() => {
    if (selectedDestination() && !selectedDestinationRow()) closeDestination();
  });
  function assessmentRecordContent(row: DestinationAssessmentRecord): RecordContent {
    const siblings = layout().find(
      (candidate) => destinationKey(candidate.module) === row.moduleId,
    )?.assessments;
    const index = siblings?.findIndex((entry) => destinationKey(entry) === row.id) ?? -1;
    return destinationAssessmentContent(row, [
      {
        id: "place_assessment",
        kind: "command",
        label: "Place Assessment",
        title: `Choose destination for ${row.title}`,
        disabled: disabled(),
        expanded: selectedDestination() === row.id,
        controls: selectedDestination() === row.id ? placementDetailId : undefined,
        onClick: (event): void => {
          destinationTrigger = event.currentTarget;
          setSelectedDestination(row.id);
          queueMicrotask(() => destinationHeading?.focus());
        },
      },
      {
        id: "move_earlier",
        kind: "command",
        label: "Move earlier",
        title: `Move ${row.title} earlier within its Module`,
        disabled: disabled() || index <= 0,
        onClick: (): void => moveDestinationByOffset(assessmentReorder, row.id, -1),
      },
      {
        id: "move_later",
        kind: "command",
        label: "Move later",
        title: `Move ${row.title} later within its Module`,
        disabled: disabled() || index < 0 || index >= (siblings?.length ?? 0) - 1,
        onClick: (): void => moveDestinationByOffset(assessmentReorder, row.id, 1),
      },
      {
        id: "remove_assessment",
        kind: "command",
        label: "Remove Assessment",
        title: `Remove Assessment: ${row.title}`,
        disabled: disabled(),
        onClick: (): void => place(row.entry, ""),
      },
    ]);
  }
  const assessmentSearch = createLocalCompleteSearch(
    { rows: () => records().assessments },
    {
      fields: destinationAssessmentFields,
      rowId: (row) => row.id,
      keyword: (row) =>
        `${row.title} ${row.moduleContext} ${row.assessmentIndex + 1} ${row.sourceContext}`,
      compare: (key, left, right) =>
        key === "module" ? left.moduleIndex - right.moduleIndex : undefined,
      content: assessmentRecordContent,
    },
  );
  function moduleRecordContent(row: DestinationModuleRecord): RecordContent {
    const index = layout().findIndex((candidate) => destinationKey(candidate.module) === row.id);
    return destinationModuleContent(row, [
      {
        id: "view_assessments",
        kind: "command",
        label: "View Assessments",
        onClick: (): void => assessmentSearch.query.filters?.onChange("module", [row.context]),
      },
      {
        id: "move_earlier",
        kind: "command",
        label: "Move earlier",
        title: `Move Module ${row.label} earlier`,
        disabled: disabled() || index <= 0,
        onClick: (): void => moveDestinationByOffset(moduleReorder, row.id, -1),
      },
      {
        id: "move_later",
        kind: "command",
        label: "Move later",
        title: `Move Module ${row.label} later`,
        disabled: disabled() || index < 0 || index >= layout().length - 1,
        onClick: (): void => moveDestinationByOffset(moduleReorder, row.id, 1),
      },
      {
        id: "remove_module",
        kind: "command",
        label: "Remove Module",
        title: `Remove Module: ${row.label}`,
        disabled: disabled(),
        onClick: (): void =>
          edit(layout().filter((candidate) => destinationKey(candidate.module) !== row.id)),
      },
    ]);
  }
  const moduleSearch = createLocalCompleteSearch(
    { rows: () => records().modules },
    {
      fields: destinationModuleFields,
      rowId: (row) => row.id,
      keyword: (row) => `${row.label} ${row.moduleIndex + 1} ${row.sourceContext}`,
      content: moduleRecordContent,
    },
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
        {/* ASVS 1.2.1: authored labels remain escaped text in shared records and selectors. */}
        <div
          class="blueprint-fork-destination"
          tabindex="-1"
          ref={(element) => (destinationOverview = element)}
        >
          <section aria-label="Destination Module index">
            <h5>Modules</h5>
            <SearchBlock
              layout="above"
              controls={
                <SearchControls state={moduleSearch.state} textLabel="Search destination Modules" />
              }
              state={moduleSearch.state}
              collection={{
                ariaLabel: "Complete destination Modules",
                initialMode: "compact",
                fields: destinationModuleFields,
                query: moduleSearch.query,
                disabled,
                reorder: moduleReorder,
                emptyState: {
                  title: "No destination Modules match.",
                  message: "Restore or create a destination Module before placing an Assessment.",
                },
              }}
            />
          </section>
          <section aria-label="Destination Assessment overview">
            <h5>Assessments</h5>
            <SearchBlock
              layout="above"
              controls={
                <SearchControls
                  state={assessmentSearch.state}
                  textLabel="Search destination Assessments"
                />
              }
              state={assessmentSearch.state}
              collection={{
                ariaLabel: "All placed destination Assessments",
                initialMode: "compact",
                fields: destinationAssessmentFields,
                query: assessmentSearch.query,
                disabled,
                reorder: assessmentReorder,
                emptyState: {
                  title: "No placed Assessments match.",
                  message:
                    "Use an unplaced Assessment's destination selector to place it in a Module.",
                },
              }}
            />
          </section>
          <Show when={selectedDestinationRow()}>
            {(row) => (
              <section
                class="blueprint-fork-destination-detail"
                id={placementDetailId}
                aria-labelledby={`${placementDetailId}-heading`}
              >
                <h5
                  id={`${placementDetailId}-heading`}
                  tabindex="-1"
                  ref={(element) => (destinationHeading = element)}
                >
                  Place Assessment: {row().title}
                </h5>
                <button type="button" class="quiet-action" onClick={closeDestination}>
                  Close destination selection
                </button>
                <p>{row().sourceContext}</p>
                <label>
                  Destination for {row().title}
                  <select
                    value={row().moduleId}
                    onChange={(event) => place(row().entry, event.currentTarget.value)}
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
              </section>
            )}
          </Show>
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
