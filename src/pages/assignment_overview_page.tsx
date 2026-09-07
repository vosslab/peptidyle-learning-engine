// M11 Student Assignment Access and initial issued presentation.

import { A, createAsync, useParams } from "@solidjs/router";
import { createSignal, For, Match, Show, Switch, type JSX } from "solid-js";

import type { LiveAssignmentAttempt } from "../api/assignment_attempt_issuance";
import { useApplicationApi } from "../api/application_api";
import { QuestionPresentationRenderer } from "../components/question_renderer";
import { QuestionPresentationResponseControl } from "../components/question_response_controls/question_response_control";
import { parseAssignmentReference, parseCourseInstanceReference } from "../navigation/public_route";
import { useWasmFacade } from "../wasm/context";
import type { SubmissionOutcome } from "../features/question_attempt/question_attempt_state";
import type { StudentResponse } from "../../generated/api/StudentResponse";

function startDecisionMessage(decision: string): string {
  switch (decision) {
    case "may_start":
      return "This Assignment is available to start.";
    case "not_yet_available":
      return "This Assignment is not yet available.";
    case "closed":
      return "This Assignment is closed for new work.";
    case "attempt_limit_reached":
      return "The allowed number of Assignment Attempts has been reached.";
    case "late_work_refused":
      return "New work is not available under the late-work policy.";
    default:
      return "This Assignment is unavailable.";
  }
}

function presentationNonce(value: string | undefined): string | null {
  return value !== undefined && /^[0-9a-f]{32}$/u.test(value) ? value : null;
}

/** M11 uses only public C-/A- references and keeps the response boundary for M12. */
export function AssignmentOverviewPage(): JSX.Element {
  const runtime = useApplicationApi();
  const validator = useWasmFacade();
  const params = useParams();
  const [issued, setIssued] = createSignal<LiveAssignmentAttempt>();
  const [starting, setStarting] = createSignal(false);
  const [startError, setStartError] = createSignal<string>();
  const course = () => parseCourseInstanceReference(params["courseRef"] ?? "");
  const assignment = () => parseAssignmentReference(params["assignmentRef"] ?? "");
  const selectedPresentationNonce = () => presentationNonce(params["presentationNonce"]);
  const isSubmissionScreen = () => selectedPresentationNonce() !== null;
  const access = createAsync(() => {
    const courseReference = course();
    const assignmentReference = assignment();
    if (courseReference === null || assignmentReference === null) return Promise.resolve(undefined);
    return runtime.client.getLiveAssignmentAccess(courseReference, assignmentReference);
  });

  async function startAssignment(): Promise<void> {
    const courseReference = course();
    const assignmentReference = assignment();
    if (courseReference === null || assignmentReference === null || starting()) return;
    setStarting(true);
    setStartError(undefined);
    try {
      const attempt = await runtime.client.startLiveAssignment(
        courseReference,
        assignmentReference,
      );
      const selected = selectedPresentationNonce();
      if (
        selected !== null &&
        !attempt.questions.some((question) => question.presentationNonce === selected)
      ) {
        setStartError("This response screen is unavailable.");
        return;
      }
      setIssued(attempt);
    } catch (_error: unknown) {
      setStartError("Assignment could not be started. Please try again.");
    } finally {
      setStarting(false);
    }
  }

  async function submitResponse(
    presentationNonce: string,
    response: StudentResponse,
  ): Promise<SubmissionOutcome> {
    const courseReference = course();
    const assignmentReference = assignment();
    if (courseReference === null || assignmentReference === null) {
      return { kind: "rejected", message: "This Assignment is unavailable." };
    }
    try {
      await runtime.client.submitLiveNativePleResponse(
        courseReference,
        assignmentReference,
        presentationNonce,
        response,
      );
      return { kind: "accepted" };
    } catch (_error: unknown) {
      return { kind: "rejected", message: "Your response could not be submitted. Try again." };
    }
  }

  return (
    <section class="page" data-route-surface="assignmentOverview">
      <Show
        when={issued()}
        fallback={
          <>
            <h1>Assignment</h1>
            <Show
              when={access()}
              fallback={<p class="loading-state">Loading Assignment Access...</p>}
            >
              {(current) => (
                <>
                  <p role="status">{startDecisionMessage(current().startDecision)}</p>
                  <Switch>
                    <Match when={current().startDecision === "may_start"}>
                      <button
                        class="primary-action"
                        type="button"
                        disabled={starting()}
                        onClick={() => void startAssignment()}
                      >
                        {starting() ? "Starting Assignment..." : "Start Assignment"}
                      </button>
                    </Match>
                    <Match when={true}>
                      <p>
                        Check with your Instructor if you expected this Assignment to be available.
                      </p>
                    </Match>
                  </Switch>
                  <Show when={startError()}>
                    {(message) => (
                      <p role="alert" class="inline-error">
                        {message()}
                      </p>
                    )}
                  </Show>
                </>
              )}
            </Show>
          </>
        }
      >
        {(current) => (
          <>
            <header>
              <p class="eyebrow">Assignment Attempt {current().attemptNumber}</p>
              <h1>{current().title}</h1>
              <Show when={current().resumed}>
                <p role="status">Your current Assignment Attempt has been reopened.</p>
              </Show>
            </header>
            <Show when={current().instructions.length > 0}>
              <section aria-labelledby="assignment-instructions-heading">
                <h2 id="assignment-instructions-heading">Instructions</h2>
                <p>{current().instructions}</p>
              </section>
            </Show>
            <section aria-labelledby="issued-questions-heading">
              <h2 id="issued-questions-heading" tabindex="-1">
                Questions
              </h2>
              <For
                each={current().questions.filter(
                  (question) =>
                    selectedPresentationNonce() === null ||
                    question.presentationNonce === selectedPresentationNonce(),
                )}
              >
                {(question, index) => (
                  <article class="question-presentation">
                    <h3>
                      Question {index() + 1}: {question.questionTitle}
                    </h3>
                    <QuestionPresentationRenderer
                      presentation={question}
                      assetUrl={(asset) =>
                        new URL(
                          runtime.client.assetUrl(asset.questionAsset),
                          window.location.origin,
                        )
                      }
                    />
                    <QuestionPresentationResponseControl
                      attemptId={question.presentationNonce}
                      mode={isSubmissionScreen() ? "submission" : "formatOnly"}
                      responseFormat={question.response}
                      validator={validator}
                      onSubmit={
                        isSubmissionScreen()
                          ? (response) => submitResponse(question.presentationNonce, response)
                          : undefined
                      }
                      onEscape={() => document.getElementById("issued-questions-heading")?.focus()}
                    />
                    <Show when={!isSubmissionScreen()}>
                      <p>
                        <A
                          href={`/courses/${course() ?? ""}/assignments/${assignment() ?? ""}/presentations/${question.presentationNonce}`}
                        >
                          Answer this question
                        </A>
                      </p>
                    </Show>
                  </article>
                )}
              </For>
            </section>
          </>
        )}
      </Show>
    </section>
  );
}
