import { For, Show, createMemo, createSignal, type JSX } from "solid-js";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { ApiRequestError, BlueprintCourseConflictError } from "../../api/http_client";
import {
  currentForkLayout,
  destinationKey,
  forkSelectionProblem,
  reordered,
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
        props.review.left.modules.find(
          (m) => m.blueprintModuleReference === module.sourceModuleReference,
        )?.label ?? "New module"
      );
    const copy = labels().find((c) => c.targetModuleReference === module.targetModuleReference);
    return (
      (copy
        ? props.review.left.modules.find(
            (m) => m.blueprintModuleReference === copy.sourceModuleReference,
          )?.label
        : props.review.right.modules.find(
            (m) => m.blueprintModuleReference === module.targetModuleReference,
          )?.label) ?? "Module"
    );
  };
  const title = (entry: Layout["assessments"][number]): string => {
    if (entry.kind === "newFromSource")
      return (
        props.review.left.assessments.find(
          (a) => a.blueprintAssessmentReference === entry.sourceAssessmentReference,
        )?.content.title ?? "New Assessment"
      );
    const copy = contents().find(
      (c) => c.targetAssessmentReference === entry.targetAssessmentReference,
    );
    return (
      (copy
        ? props.review.left.assessments.find(
            (a) => a.blueprintAssessmentReference === copy.sourceAssessmentReference,
          )?.content.title
        : props.review.right.assessments.find(
            (a) => a.blueprintAssessmentReference === entry.targetAssessmentReference,
          )?.content.title) ?? "Assessment"
    );
  };
  function edit(next: Layout[]): void {
    props.proposalSelection?.onChanged?.();
    setLayout(next);
    setEdited(true);
    setMessage("");
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
      ...labels().filter((c) => c.sourceModuleReference !== source),
      ...(value
        ? [{ sourceModuleReference: source, targetModuleReference: value === "new" ? null : value }]
        : []),
    ]);
    const next = layout().filter(
      (row) =>
        !(row.module.kind === "newFromSource" && row.module.sourceModuleReference === source),
    );
    if (value === "new")
      next.push({
        module: { kind: "newFromSource", sourceModuleReference: source },
        assessments: [],
      });
    if (next.length !== layout().length || value === "new") edit(next);
  }
  function chooseAssessment(source: string, value: string): void {
    props.proposalSelection?.onChanged?.();
    setContents([
      ...contents().filter((c) => c.sourceAssessmentReference !== source),
      ...(value
        ? [
            {
              sourceAssessmentReference: source,
              targetAssessmentReference: value === "new" ? null : value,
            },
          ]
        : []),
    ]);
    if (
      value !== "new" &&
      layout().some((row) =>
        row.assessments.some(
          (a) => a.kind === "newFromSource" && a.sourceAssessmentReference === source,
        ),
      )
    )
      place({ kind: "newFromSource", sourceAssessmentReference: source }, "");
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
      await props.client.applyBlueprintFork(props.review.right.currentRevision.reference, {
        expectedSource: props.review.left.currentRevision,
        expectedFork: props.review.right.currentRevision,
        expectedSourceMetadataEtag: props.review.left.metadataEtag,
        expectedForkMetadataEtag: props.review.right.metadataEtag,
        sourceShortName: shortName(),
        sourceLongName: longName(),
        selection: {
          sourceModuleLabels: labels(),
          sourceAssessments: contents(),
          layout: edited() ? layout() : null,
        },
      });
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
        targetAssessmentReference: a.blueprintAssessmentReference,
      })),
      ...contents()
        .filter((c) => c.targetAssessmentReference === null)
        .map((c) => ({
          kind: "newFromSource" as const,
          sourceAssessmentReference: c.sourceAssessmentReference,
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
                  labels().find((c) => c.sourceModuleReference === source.blueprintModuleReference)
                    ?.targetModuleReference ??
                  (labels().some((c) => c.sourceModuleReference === source.blueprintModuleReference)
                    ? "new"
                    : "")
                }
                onChange={(e) =>
                  chooseModule(source.blueprintModuleReference, e.currentTarget.value)
                }
              >
                <option value="">Do not copy</option>
                <option value="new">Create new module</option>
                <For each={props.review.right.modules}>
                  {(target) => (
                    <option value={target.blueprintModuleReference}>
                      Replace label: {target.label}
                    </option>
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
                  contents().find(
                    (c) => c.sourceAssessmentReference === source.blueprintAssessmentReference,
                  )?.targetAssessmentReference ??
                  (contents().some(
                    (c) => c.sourceAssessmentReference === source.blueprintAssessmentReference,
                  )
                    ? "new"
                    : "")
                }
                onChange={(e) =>
                  chooseAssessment(source.blueprintAssessmentReference, e.currentTarget.value)
                }
              >
                <option value="">Do not copy</option>
                <option value="new">Create new Assessment copy</option>
                <For each={props.review.right.assessments}>
                  {(target) => {
                    const shared = (): number =>
                      props.review.assessmentRelationships.find(
                        (r) =>
                          r.leftAssessmentReference === source.blueprintAssessmentReference &&
                          r.rightAssessmentReference === target.blueprintAssessmentReference,
                      )?.sharedQuestionIds.length ?? 0;
                    return (
                      <option value={target.blueprintAssessmentReference}>
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
          destination references are assigned by the server.
        </p>
        <h4>Complete destination layout</h4>
        <p>
          Move related changes together. Removing a module also excludes its Assessments unless you
          move them first.
        </p>
        <ol class="blueprint-fork-destination">
          <For each={layout()}>
            {(row, index) => (
              <li>
                <h5>{moduleLabel(row.module)}</h5>
                <div class="blueprint-fork-layout-actions">
                  <button
                    type="button"
                    disabled={index() === 0}
                    onClick={() => edit(reordered(layout(), index(), -1))}
                  >
                    Move module up
                  </button>
                  <button
                    type="button"
                    disabled={index() === layout().length - 1}
                    onClick={() => edit(reordered(layout(), index(), 1))}
                  >
                    Move module down
                  </button>
                  <button type="button" onClick={() => edit(layout().filter((r) => r !== row))}>
                    Remove module: {moduleLabel(row.module)}
                  </button>
                </div>
                <ol>
                  <For each={row.assessments}>
                    {(entry, position) => (
                      <li>
                        {title(entry)}
                        <div class="blueprint-fork-layout-actions">
                          <button
                            type="button"
                            disabled={position() === 0}
                            onClick={() =>
                              edit(
                                layout().map((r) =>
                                  r === row
                                    ? {
                                        ...r,
                                        assessments: reordered(r.assessments, position(), -1),
                                      }
                                    : r,
                                ),
                              )
                            }
                          >
                            Move Assessment up
                          </button>
                          <button
                            type="button"
                            disabled={position() === row.assessments.length - 1}
                            onClick={() =>
                              edit(
                                layout().map((r) =>
                                  r === row
                                    ? { ...r, assessments: reordered(r.assessments, position(), 1) }
                                    : r,
                                ),
                              )
                            }
                          >
                            Move Assessment down
                          </button>
                          <label>
                            Destination for {title(entry)}
                            <select
                              value={destinationKey(row.module)}
                              onChange={(e) => place(entry, e.currentTarget.value)}
                            >
                              <For each={layout()}>
                                {(r) => (
                                  <option value={destinationKey(r.module)}>
                                    {moduleLabel(r.module)}
                                  </option>
                                )}
                              </For>
                            </select>
                          </label>
                          <button type="button" onClick={() => place(entry, "")}>
                            Remove Assessment: {title(entry)}
                          </button>
                        </div>
                      </li>
                    )}
                  </For>
                </ol>
              </li>
            )}
          </For>
        </ol>
        <For
          each={props.review.right.modules.filter(
            (m) =>
              !layout().some(
                (r) =>
                  r.module.kind === "existing" &&
                  r.module.targetModuleReference === m.blueprintModuleReference,
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
                    module: { kind: "existing", targetModuleReference: m.blueprintModuleReference },
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
