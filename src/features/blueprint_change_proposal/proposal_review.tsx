import { For, Show, createSignal, type JSX } from "solid-js";
import type { BlueprintChangeProposalDetailView } from "../../../generated/api/BlueprintChangeProposalDetailView";
import type { BlueprintChangeProposalDecisionView } from "../../../generated/api/BlueprintChangeProposalDecisionView";
import type { BlueprintChangeProposalAcceptedView } from "../../../generated/api/BlueprintChangeProposalAcceptedView";
import type { BlueprintChangeProposalClient } from "../../api/blueprint_change_proposal";
import { BlueprintCourseConflictError } from "../../api/http_client";
import { CourseClassificationSummary } from "../../components/course_classification_summary";
import { AssessmentSnapshot, Settings } from "../blueprint_forks/blueprint_fork_review";
import { BlueprintSelectionEditor } from "../blueprint_forks/blueprint_fork_apply";
import { currentForkLayout, destinationKey } from "../blueprint_forks/blueprint_fork_apply_model";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import { proposalInventory } from "./proposal_model";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintChangeProposalSideView } from "../../../generated/api/BlueprintChangeProposalSideView";
import type { Layout } from "../blueprint_forks/blueprint_fork_apply_model";
import "./proposal.css";

export function ProposalReview(props: {
  readonly client: BlueprintChangeProposalClient;
  readonly detail: BlueprintChangeProposalDetailView;
  readonly refresh: () => Promise<void>;
  readonly onAccepted?: () => void;
  readonly hasUnsavedChanges?: boolean;
}): JSX.Element {
  const [mode, setMode] = createSignal<"entire" | "selected">("selected");
  const [classification, setClassification] = createSignal(false);
  const [chosen, setChosen] = createSignal<BlueprintChangeProposalDecisionView>();
  const [committed, setCommitted] = createSignal<BlueprintChangeProposalAcceptedView>();
  const [busy, setBusy] = createSignal(false),
    [locked, setLocked] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const inventory = (): BlueprintComparisonView => proposalInventory(props.detail.comparison);
  const accepted = (): BlueprintChangeProposalAcceptedView | null =>
    committed() ?? props.detail.accepted;
  const canChoose = (): boolean =>
    props.detail.canAccept &&
    !props.detail.proposal.targetIsStale &&
    !accepted() &&
    !locked() &&
    !props.hasUnsavedChanges;
  function cancel(): void {
    setChosen(undefined);
    setMessage("Confirmation cancelled. Nothing was sent.");
  }
  async function accept(): Promise<void> {
    const decision = chosen();
    if (!decision || !canChoose() || busy()) return;
    setBusy(true);
    try {
      // ASVS 2.2.2/8.2.2: UI capability is advisory; acceptance rechecks authorization and both pins on the server.
      const result = await props.client.acceptBlueprintChangeProposal(
        props.detail.proposal.proposalId,
        {
          expectedTarget: props.detail.proposal.target,
          expectedTargetBlueprintEditNumber: props.detail.proposal.targetBlueprintEditNumber,
          decision,
        },
      );
      setCommitted(result);
      setChosen(undefined);
      setMessage(
        "Proposal accepted. The committed result below is retained even if the target changes later.",
      );
      props.onAccepted?.();
      try {
        await props.refresh();
      } catch {
        setMessage(
          "Proposal accepted. Record refresh could not complete; the exact committed result remains below. Reopen the record to refresh it.",
        );
      }
    } catch (error: unknown) {
      setLocked(true);
      setChosen(undefined);
      setClassification(false);
      setMessage(
        error instanceof BlueprintCourseConflictError
          ? "The target Revision or metadata changed. Acceptance choices were cleared. The original proposal is not rebased; create a new proposal for a new basis."
          : "Acceptance was not confirmed. Choices were cleared. Refresh the record before trying again; do not repeat the request automatically.",
      );
      try {
        await props.refresh();
      } catch {
        setMessage(
          (previous) =>
            previous + " The record refresh failed; reopen the record before trying again.",
        );
      }
    } finally {
      setBusy(false);
    }
  }
  return (
    <section class="blueprint-proposal" aria-label="Change Proposal review" aria-busy={busy()}>
      <h2>Change Proposal</h2>
      <Show when={message()}>
        <p role="status">{message()}</p>
      </Show>
      <Show when={accepted()}>
        {(result) => <AcceptedResult value={result()} detail={props.detail} />}
      </Show>
      <Show when={!accepted() && props.detail.proposal.targetIsStale}>
        <p role="alert">
          Older target basis: the target Revision or metadata changed. This frozen evidence remains
          readable; acceptance is disabled. A new submission is needed for a new basis.
        </p>
      </Show>
      <p>
        Created {props.detail.proposal.createdAt}. Source and comparison target are frozen saved
        evidence, not current editors.
      </p>
      <p>
        Shared Question IDs relate Assessments, including splits and combinations. Titles and local
        references do not match content automatically. Question bodies and answers are not exposed.
      </p>
      <p>
        Shared Questions: {props.detail.comparison.sharedQuestionIds.join(", ") || "none"}. Source
        only: {props.detail.comparison.sourceOnlyQuestionIds.join(", ") || "none"}. Target only:{" "}
        {props.detail.comparison.targetOnlyQuestionIds.join(", ") || "none"}.
      </p>
      <div class="proposal-columns">
        <For each={["source", "target"] as const}>
          {(side) => {
            const value = (): BlueprintChangeProposalSideView => props.detail.comparison[side];
            return (
              <section>
                <h3>{side === "source" ? "Proposed source" : "Before: comparison target"}</h3>
                <h4>{value().names.longName}</h4>
                <p>
                  Short name: {value().names.shortName}. Blueprint{" "}
                  {value().revision.blueprint_course_id}; frozen Revision{" "}
                  {value().revision.revision}; edit {value().blueprintEditNumber}.
                </p>
                <p>Exact classification identities below use current vocabulary labels.</p>
                <CourseClassificationSummary value={value().classification} />
                <For each={[...value().modules].sort((a, b) => a.position - b.position)}>
                  {(module) => (
                    <section>
                      <h4>
                        {module.position + 1}. {module.label}
                      </h4>
                      <For
                        each={value()
                          .assessments.filter(
                            (a) => a.blueprintModuleReference === module.blueprintModuleReference,
                          )
                          .sort((a, b) => a.position - b.position)}
                      >
                        {(assessment) => (
                          <details>
                            <summary>
                              {assessment.position + 1}. {assessment.content.title}
                            </summary>
                            <AssessmentSnapshot
                              snapshot={assessment}
                              side={inventory()[side === "source" ? "left" : "right"]}
                            />
                            <p>Related {side === "source" ? "target" : "source"} Assessments:</p>
                            <For
                              each={props.detail.comparison.assessmentRelationships.filter(
                                (edge) =>
                                  (side === "source"
                                    ? edge.leftAssessmentId
                                    : edge.rightAssessmentId) === assessment.blueprintAssessmentId,
                              )}
                              fallback={<p>No shared-Question relationship (unmatched).</p>}
                            >
                              {(edge) => (
                                <p>
                                  {
                                    props.detail.comparison[
                                      side === "source" ? "target" : "source"
                                    ].assessments.find(
                                      (a) =>
                                        a.blueprintAssessmentId ===
                                        (side === "source"
                                          ? edge.rightAssessmentId
                                          : edge.leftAssessmentId),
                                    )?.content.title
                                  }
                                  : shared Questions {edge.sharedQuestionIds.join(", ")}.
                                </p>
                              )}
                            </For>
                          </details>
                        )}
                      </For>
                    </section>
                  )}
                </For>
              </section>
            );
          }}
        </For>
      </div>
      <Show when={!accepted() && !props.detail.canAccept && !props.detail.proposal.targetIsStale}>
        <p>
          This record is read-only for your Account or the target lifecycle. Only the receiving
          owner may accept it.
        </p>
      </Show>
      <Show when={props.hasUnsavedChanges && !accepted()}>
        <p>Save or cancel your target editor changes before accepting a proposal.</p>
      </Show>
      <Show when={canChoose()}>
        <fieldset disabled={busy()}>
          <legend>Acceptance scope</legend>
          <label>
            <input
              type="radio"
              name="proposal-scope"
              checked={mode() === "selected"}
              onChange={() => {
                setMode("selected");
                setChosen(undefined);
              }}
            />{" "}
            Selected changes
          </label>
          <label>
            <input
              type="radio"
              name="proposal-scope"
              checked={mode() === "entire"}
              onChange={() => {
                setMode("entire");
                setChosen(undefined);
              }}
            />{" "}
            Entire proposal
          </label>
        </fieldset>
        <Show
          when={mode() === "entire"}
          fallback={
            <BlueprintSelectionEditor
              review={inventory()}
              onApplied={() => undefined}
              proposalSelection={{
                classificationControl: (
                  <label>
                    <input
                      type="checkbox"
                      checked={classification()}
                      onChange={(e) => {
                        setChosen(undefined);
                        setClassification(e.currentTarget.checked);
                      }}
                    />{" "}
                    Use the source's complete classification tuple, including Tags
                  </label>
                ),
                hasClassificationSelection: classification,
                disabled: busy(),
                onChanged: () => setChosen(undefined),
                onReset: () => setClassification(false),
                submit: (selection, sourceShortName, sourceLongName) => {
                  setChosen({
                    kind: "selected",
                    selection,
                    sourceShortName,
                    sourceLongName,
                    sourceClassification: classification(),
                  });
                  return Promise.resolve();
                },
              }}
            />
          }
        >
          <p>
            Entire acceptance replaces the complete target module/Assessment structure and copies
            both source names and the complete classification including Tags. Target-only units are
            removed. Question Revision pins stay exact. Pool ID and Pool Edit Number remain sibling
            fields.
          </p>
          <button type="button" disabled={busy()} onClick={() => setChosen({ kind: "entire" })}>
            Review entire result
          </button>
        </Show>
        <p>
          Selected content uses complete Assessments: individual settings or Question members cannot
          be accepted separately. Unselected target units stay unless explicitly removed from the
          complete layout.
        </p>
        <Show when={chosen()}>
          {(decision) => (
            <section aria-label="Confirm proposal acceptance">
              <h3>Chosen result before acceptance</h3>
              <DecisionSummary detail={props.detail} decision={decision()} />
              <p>
                This final decision creates a target Blueprint Revision. Daughter Course Instances
                do not change.
              </p>
              <button
                type="button"
                class="primary-action"
                disabled={busy()}
                onClick={() => void accept()}
              >
                {busy() ? "Accepting..." : "Confirm acceptance"}
              </button>
              <button type="button" disabled={busy()} onClick={cancel}>
                Cancel confirmation
              </button>
            </section>
          )}
        </Show>
      </Show>
    </section>
  );
}

function DecisionSummary(props: {
  readonly detail: BlueprintChangeProposalDetailView;
  readonly decision: BlueprintChangeProposalDecisionView;
}): JSX.Element {
  const review = (): BlueprintComparisonView => proposalInventory(props.detail.comparison);
  const source = (): BlueprintChangeProposalSideView => props.detail.comparison.source,
    target = (): BlueprintChangeProposalSideView => props.detail.comparison.target;
  const uses = (key: "sourceShortName" | "sourceLongName" | "sourceClassification"): boolean =>
    props.decision.kind === "entire" || props.decision[key];
  const layout = (): Layout[] =>
    props.decision.kind === "entire"
      ? []
      : (props.decision.selection.layout ?? currentForkLayout(review()));
  function moduleName(key: string): string {
    if (props.decision.kind === "selected" && key.startsWith("existing:")) {
      const copy = props.decision.selection.sourceModuleLabels.find(
        (item) => key === "existing:" + item.targetModuleReference,
      );
      if (copy)
        return (
          source().modules.find(
            (module) => module.blueprintModuleReference === copy.sourceModuleReference,
          )?.label ?? key
        );
    }
    return (
      [...source().modules, ...target().modules].find((m) =>
        key.endsWith(":" + m.blueprintModuleReference),
      )?.label ?? key
    );
  }
  function assessmentName(key: string): string {
    if (props.decision.kind === "selected" && key.startsWith("existing:")) {
      const copy = props.decision.selection.sourceAssessments.find(
        (item) => key === "existing:" + item.targetAssessmentId,
      );
      if (copy)
        return (
          source().assessments.find(
            (assessment) => assessment.blueprintAssessmentId === copy.sourceAssessmentId,
          )?.content.title ?? key
        );
    }
    return (
      [...source().assessments, ...target().assessments].find((a) =>
        key.endsWith(":" + a.blueprintAssessmentId),
      )?.content.title ?? key
    );
  }
  return (
    <>
      <p>
        Decision: {props.decision.kind}. Short name:{" "}
        {(uses("sourceShortName") ? source() : target()).names.shortName}. Long name:{" "}
        {(uses("sourceLongName") ? source() : target()).names.longName}.
      </p>
      <CourseClassificationSummary
        value={(uses("sourceClassification") ? source() : target()).classification}
      />
      <Show when={props.decision.kind === "selected" ? props.decision : undefined}>
        {(selected) => (
          <>
            <For each={selected().selection.sourceModuleLabels}>
              {(copy) => (
                <p>
                  Copy source module label{" "}
                  {
                    source().modules.find(
                      (m) => m.blueprintModuleReference === copy.sourceModuleReference,
                    )?.label
                  }{" "}
                  to{" "}
                  {copy.targetModuleReference === null
                    ? "a new module"
                    : target().modules.find(
                        (m) => m.blueprintModuleReference === copy.targetModuleReference,
                      )?.label}
                  .
                </p>
              )}
            </For>
            <For each={selected().selection.sourceAssessments}>
              {(copy) => (
                <p>
                  Copy complete source Assessment{" "}
                  {
                    source().assessments.find(
                      (a) => a.blueprintAssessmentId === copy.sourceAssessmentId,
                    )?.content.title
                  }{" "}
                  to{" "}
                  {copy.targetAssessmentId === null
                    ? "a new Assessment"
                    : target().assessments.find(
                        (a) => a.blueprintAssessmentId === copy.targetAssessmentId,
                      )?.content.title}
                  .
                </p>
              )}
            </For>
            <h4>Resulting destination order</h4>
            <ol>
              <For each={layout()}>
                {(row) => (
                  <li>
                    {moduleName(destinationKey(row.module))}
                    <ol>
                      <For each={row.assessments}>
                        {(entry) => <li>{assessmentName(destinationKey(entry))}</li>}
                      </For>
                    </ol>
                  </li>
                )}
              </For>
            </ol>
          </>
        )}
      </Show>
      <Show when={props.decision.kind === "entire"}>
        <p>The complete proposed source structure shown above becomes the target structure.</p>
      </Show>
    </>
  );
}

function AcceptedResult(props: {
  readonly value: BlueprintChangeProposalAcceptedView;
  readonly detail: BlueprintChangeProposalDetailView;
}): JSX.Element {
  function unitName(side: "source" | "target", reference: string, module: boolean): string {
    const inventory = props.detail.comparison[side];
    return module
      ? (inventory.modules.find((m) => m.blueprintModuleReference === reference)?.label ?? "Module")
      : (inventory.assessments.find((a) => a.blueprintAssessmentId === reference)?.content.title ??
          "Assessment");
  }
  return (
    <section aria-label="Committed acceptance result">
      <h3>Accepted: exact committed target</h3>
      <p>
        Blueprint {props.value.target.blueprint_course_id}, Revision {props.value.target.revision},
        edit {props.value.targetBlueprintEditNumber}; accepted {props.value.acceptedAt}. Decision:{" "}
        {props.value.decision.kind}.
      </p>
      <Show when={props.value.decision.kind === "selected" ? props.value.decision : undefined}>
        {(decision) => (
          <p>
            Source short name: {decision().sourceShortName ? "copied" : "retained target"}; source
            long name: {decision().sourceLongName ? "copied" : "retained target"}; complete
            classification: {decision().sourceClassification ? "copied" : "retained target"}.
            Complete Assessment copies: {decision().selection.sourceAssessments.length};
            module-label copies: {decision().selection.sourceModuleLabels.length}; layout:{" "}
            {decision().selection.layout === null ? "retained" : "explicit replacement"}.
          </p>
        )}
      </Show>
      <p>
        {props.value.resultingJson.metadata.long_name}; short name{" "}
        {props.value.resultingJson.metadata.short_name}.
      </p>
      <CourseClassificationSummary value={props.value.resultingJson.metadata.classification} />
      <h4>Exact applied copies and destinations</h4>
      <For
        each={props.value.appliedSelection.sourceModuleLabels}
        fallback={<p>No module-label copies.</p>}
      >
        {(copy) => (
          <p>
            Source label {unitName("source", copy.sourceModuleReference, true)} (
            {copy.sourceModuleReference}) to{" "}
            {copy.targetModuleReference === null
              ? `new module (${props.value.newModules[copy.sourceModuleReference] ?? "server-assigned"})`
              : `${unitName("target", copy.targetModuleReference, true)} (${copy.targetModuleReference})`}
            .
          </p>
        )}
      </For>
      <For
        each={props.value.appliedSelection.sourceAssessments}
        fallback={<p>No Assessment copies.</p>}
      >
        {(copy) => (
          <p>
            Complete source Assessment {unitName("source", copy.sourceAssessmentId, false)} (
            {copy.sourceAssessmentId}) to{" "}
            {copy.targetAssessmentId === null
              ? `new Assessment (${props.value.newAssessments[copy.sourceAssessmentId] ?? "server-assigned"})`
              : `${unitName("target", copy.targetAssessmentId, false)} (${copy.targetAssessmentId})`}
            .
          </p>
        )}
      </For>
      <Show when={props.value.appliedSelection.layout} fallback={<p>Target layout retained.</p>}>
        {(layout) => (
          <>
            <h4>Exact applied destination layout</h4>
            <ol>
              <For each={layout()}>
                {(row) => (
                  <li>
                    {row.module.kind === "existing"
                      ? `${unitName("target", row.module.targetModuleReference, true)} (${row.module.targetModuleReference})`
                      : `${unitName("source", row.module.sourceModuleReference, true)} (new: ${props.value.newModules[row.module.sourceModuleReference] ?? row.module.sourceModuleReference})`}
                    <ol>
                      <For each={row.assessments}>
                        {(entry) => (
                          <li>
                            {entry.kind === "existing"
                              ? `${unitName("target", entry.targetAssessmentId, false)} (${entry.targetAssessmentId})`
                              : `${unitName("source", entry.sourceAssessmentId, false)} (new: ${props.value.newAssessments[entry.sourceAssessmentId] ?? entry.sourceAssessmentId})`}
                          </li>
                        )}
                      </For>
                    </ol>
                  </li>
                )}
              </For>
            </ol>
          </>
        )}
      </Show>
      <For each={props.value.resultingJson.modules}>
        {(module) => (
          <section>
            <h4>{module.label}</h4>
            <For each={module.assessments}>
              {(assessment) => (
                <details>
                  <summary>{assessment.title}</summary>
                  <p>{assessmentTypePresentation(assessment.assessment_type).label}</p>
                  <p class="blueprint-fork-instructions">
                    {assessment.instructions || "No instructions."}
                  </p>
                  <Settings value={assessment.defaults} />
                  <ol>
                    <For each={assessment.entries}>
                      {(entry) => (
                        <li>
                          {entry.kind === "fixed"
                            ? `Question ${entry.published_question.questionId}, Revision ${entry.published_question.revisionNumber}; ${entry.points_possible} points`
                            : `Pool ${entry.question_pool_id}, Edit ${entry.question_pool_edit_number}; select ${entry.selection_count}; ${entry.points_per_item} points per item`}
                          <Settings value={entry} />
                        </li>
                      )}
                    </For>
                  </ol>
                </details>
              )}
            </For>
          </section>
        )}
      </For>
      <details>
        <summary>Committed new-reference mappings</summary>
        <For each={Object.entries(props.value.newModules)}>
          {([source, target]) => (
            <p>
              Module {source} to {target}
            </p>
          )}
        </For>
        <For each={Object.entries(props.value.newAssessments)}>
          {([source, target]) => (
            <p>
              Assessment {source} to {target}
            </p>
          )}
        </For>
      </details>
      <p>
        Daughter Course Instances remain unchanged. Incorporating this accepted Blueprint Revision
        into daughters is a separate normal Blueprint update workflow.
      </p>
    </section>
  );
}
