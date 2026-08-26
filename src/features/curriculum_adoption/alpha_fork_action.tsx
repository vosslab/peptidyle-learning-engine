// alpha_fork_action.tsx - visible independent-copy action for approved Alpha readers.

import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import type { AlphaCourseView } from "../../../generated/api/AlphaCourseView";
import type { CurriculumPinReplacement } from "../../../generated/api/CurriculumPinReplacement";
import type { UnavailablePinRecoveryAction } from "../../../generated/api/UnavailablePinRecoveryAction";
import type { ForkAlphaPreviewView } from "../../../generated/api/ForkAlphaPreviewView";
import type { CurriculumAdoptionClient } from "../../api/curriculum_adoption";
import { ApiRequestError } from "../../api/http_client";

type ForkStage = "idle" | "preparing" | "review" | "applying" | "success" | "recovery";

interface AlphaForkActionProps {
  readonly source: AlphaCourseView;
  readonly client: CurriculumAdoptionClient;
}

function newIdempotencyKey(): string {
  return globalThis.crypto.randomUUID();
}

function errorMessage(error: unknown): string {
  if (error instanceof ApiRequestError) {
    if (error.status === 401) return "Your session ended. Sign in again, then retry the fork.";
    if (error.status === 403)
      return "This Alpha is no longer available to this instructor account.";
    if (error.status === 404)
      return "This Alpha is no longer available. Return to the curriculum library.";
    if (error.status === 409)
      return "This copy conflicts with the current live state. Prepare a fresh proposal before applying it.";
    if (error.status === 412)
      return "The Alpha changed while this action was preparing. Prepare a fresh copy from its current revision.";
  }
  if (error instanceof Error && error.message.length > 0) return error.message;
  return "The independent Alpha copy could not be completed. Review the recovery guidance and try again.";
}

function applyCanRetry(error: unknown): boolean {
  if (!(error instanceof ApiRequestError)) return true;
  return error.status === 408 || error.status === 429 || error.status >= 500;
}

function samePosition(
  left: CurriculumPinReplacement["position"],
  right: CurriculumPinReplacement["position"],
): boolean {
  return (
    left.moduleIndex === right.moduleIndex &&
    left.assignmentIndex === right.assignmentIndex &&
    left.entryIndex === right.entryIndex &&
    left.candidateIndex === right.candidateIndex
  );
}

function replacePin(
  replacements: ReadonlyArray<CurriculumPinReplacement>,
  action: UnavailablePinRecoveryAction,
  question: string,
): ReadonlyArray<CurriculumPinReplacement> {
  return [
    ...replacements.filter((replacement) => !samePosition(replacement.position, action.position)),
    { position: action.position, question },
  ];
}

/** Lets an approved Alpha reader create an independent, server-validated Alpha copy. */
export function AlphaForkAction(props: AlphaForkActionProps): JSX.Element {
  const [stage, setStage] = createSignal<ForkStage>("idle");
  const [preview, setPreview] = createSignal<ForkAlphaPreviewView>();
  const [completed, setCompleted] =
    createSignal<Awaited<ReturnType<CurriculumAdoptionClient["applyForkAlpha"]>>>();
  const [replacements, setReplacements] = createSignal<ReadonlyArray<CurriculumPinReplacement>>([]);
  const [idempotencyKey, setIdempotencyKey] = createSignal<string>();
  const [applyRetryAvailable, setApplyRetryAvailable] = createSignal(false);
  const [notice, setNotice] = createSignal(
    "Create an independent copy with the current Alpha revision. The server checks every question before it changes anything.",
  );

  function correction(): UnavailablePinRecoveryAction | null {
    return preview()?.pinCorrection ?? null;
  }

  async function prepare(): Promise<void> {
    setStage("preparing");
    setNotice(
      "Preparing the server-owned Alpha copy. Keep this page open while the source is checked.",
    );
    try {
      const nextPreview = await props.client.previewForkAlpha({
        source: { reference: props.source.reference, revision: props.source.revision },
        replacements: [...replacements()],
      });
      setPreview(nextPreview);
      if (nextPreview.pinCorrection !== null) {
        setStage("recovery");
        setNotice(
          "The server found an unavailable question. Choose a displayed replacement, then prepare again.",
        );
      } else {
        setStage("review");
        setNotice(
          "Review the source revision and resulting Alpha, then apply the independent copy.",
        );
      }
    } catch (error: unknown) {
      setStage("recovery");
      setNotice(errorMessage(error));
    }
  }

  async function apply(): Promise<void> {
    const currentPreview = preview();
    if (currentPreview === undefined || currentPreview.pinCorrection !== null) return;
    const key = idempotencyKey() ?? newIdempotencyKey();
    setIdempotencyKey(key);
    setApplyRetryAvailable(false);
    setStage("applying");
    setNotice("Applying the independent Alpha copy. A retry uses the same operation key.");
    try {
      setCompleted(await props.client.applyForkAlpha(currentPreview, key));
      setStage("success");
      setNotice(
        "The independent Alpha copy is complete. Its source lineage is recorded by the server.",
      );
    } catch (error: unknown) {
      const retryable = applyCanRetry(error);
      setApplyRetryAvailable(retryable);
      setStage("recovery");
      setNotice(
        retryable
          ? `${errorMessage(error)} Retry apply to reuse the same operation key.`
          : errorMessage(error),
      );
    }
  }

  function chooseReplacement(action: UnavailablePinRecoveryAction, question: string): void {
    setReplacements((current) => replacePin(current, action, question));
    setIdempotencyKey(undefined);
    setApplyRetryAvailable(false);
    setStage("recovery");
    setNotice(
      "Replacement selected. Prepare again so the live server can validate the complete copy.",
    );
  }

  function prepareFresh(): void {
    setPreview(undefined);
    setIdempotencyKey(undefined);
    setApplyRetryAvailable(false);
    void prepare();
  }

  function abandonProposal(): void {
    setPreview(undefined);
    setCompleted(undefined);
    setReplacements([]);
    setIdempotencyKey(undefined);
    setApplyRetryAvailable(false);
    setStage("idle");
    setNotice(
      "Create an independent copy with the current Alpha revision. The server checks every question before it changes anything.",
    );
  }

  return (
    <section class="curriculum-alpha-fork" aria-label="Create an independent Alpha copy">
      <div class="curriculum-alpha-fork-heading">
        <div>
          <h2>Reuse this Alpha curriculum</h2>
          <p>
            Create an independently editable Alpha owned by you. The source stays unchanged and the
            server records the source revision as lineage evidence.
          </p>
        </div>
        <Show when={stage() === "idle"}>
          <button type="button" class="primary-action" onClick={() => void prepare()}>
            Create independent copy
          </button>
        </Show>
      </div>
      <p class="curriculum-alpha-fork-status" role={stage() === "recovery" ? "alert" : "status"}>
        {notice()}
      </p>
      <Switch>
        <Match when={stage() === "preparing"}>
          <p role="status">Preparing the live server proposal...</p>
        </Match>
        <Match when={stage() === "review"}>
          <Show when={preview()}>
            {(current) => (
              <section class="curriculum-alpha-fork-review" aria-label="Alpha copy proposal">
                <h3>Review the independent copy</h3>
                <dl>
                  <div>
                    <dt>Source Alpha</dt>
                    <dd>
                      {current().source.reference}, revision {current().source.revision}
                    </dd>
                  </div>
                  <div>
                    <dt>New Alpha</dt>
                    <dd>{current().resultingAlphaTitle}</dd>
                  </div>
                </dl>
                <p>
                  {current().replacements.length === 0
                    ? "All source questions remain valid for the independent copy."
                    : `${current().replacements.length} explicit question replacement(s) will be recorded.`}
                </p>
                <div class="curriculum-alpha-fork-actions">
                  <button type="button" onClick={abandonProposal}>
                    Change my mind
                  </button>
                  <button type="button" class="primary-action" onClick={() => void apply()}>
                    Apply independent copy
                  </button>
                </div>
              </section>
            )}
          </Show>
        </Match>
        <Match when={stage() === "applying"}>
          <p role="status">Applying the server-owned independent copy...</p>
        </Match>
        <Match when={stage() === "recovery"}>
          <div class="curriculum-alpha-fork-recovery">
            <Show
              when={correction()}
              fallback={
                <div>
                  <h3>Recover this action</h3>
                  <p>Keep the source selection, then prepare a fresh server proposal.</p>
                </div>
              }
            >
              {(action) => (
                <fieldset>
                  <legend>Choose a replacement question</legend>
                  <p>
                    The source question is unavailable in the destination. Select one
                    server-provided alternative.
                  </p>
                  <div class="curriculum-alpha-fork-replacements">
                    <For each={action().candidates}>
                      {(question) => (
                        <button
                          type="button"
                          onClick={() => chooseReplacement(action(), question)}
                          aria-label={`Use replacement question ${question}`}
                        >
                          {question}
                        </button>
                      )}
                    </For>
                  </div>
                </fieldset>
              )}
            </Show>
            <div class="curriculum-alpha-fork-actions">
              <button type="button" onClick={prepareFresh}>
                Prepare again
              </button>
              <Show when={applyRetryAvailable() && idempotencyKey() !== undefined}>
                <button type="button" class="primary-action" onClick={() => void apply()}>
                  Retry apply
                </button>
              </Show>
            </div>
          </div>
        </Match>
        <Match when={stage() === "success"}>
          <Show when={completed()}>
            {(result) => (
              <section
                class="curriculum-alpha-fork-success"
                aria-label="Independent Alpha copy complete"
              >
                <h3>Independent Alpha copy complete</h3>
                <p>
                  {result().alpha} is separate from source {result().source.reference}. The server
                  recorded immutable source-lineage evidence ({result().replay}).
                </p>
                <A class="primary-link" href={`/curriculum/${encodeURIComponent(result().alpha)}`}>
                  Open the new Alpha curriculum
                </A>
              </section>
            )}
          </Show>
        </Match>
      </Switch>
    </section>
  );
}
