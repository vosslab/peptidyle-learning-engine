import { createEffect, createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type {
  CourseStudentWorkRecoveryClient,
  RecoveredAttempt,
  RecoverySelection,
} from "../api/course_student_work_recovery";
import { ApiRequestError } from "../api/http_client/error";
import { formatAssessmentActivity } from "./student_assessment_presentation";

function formatRecoveryInstant(value: string | null, timeZone: string, unsetLabel: string): string {
  if (value === null) return unsetLabel;
  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp)
    ? "Retained time unavailable"
    : formatAssessmentActivity(timestamp, timeZone);
}

function Evidence(props: { readonly label: string; readonly text: string | null }): JSX.Element {
  // ASVS 1.2.1: Solid text interpolation only, including retained backend HTML.
  return (
    <details>
      <summary>{props.label}</summary>
      <Show when={props.text !== null} fallback={<p>No retained evidence.</p>}>
        <pre
          style={{
            "white-space": "pre-wrap",
            "overflow-wrap": "anywhere",
            "max-height": "24rem",
            overflow: "auto",
          }}
        >
          {props.text}
        </pre>
      </Show>
    </details>
  );
}

/** Deliberate evidence access, not an archive browser or restoration command. */
export function CourseStudentWorkRecovery(props: {
  readonly courseInstanceId: CourseInstanceId;
  readonly client: CourseStudentWorkRecoveryClient;
  /** Authenticated Instructor Account preference, never the Course or browser zone. */
  readonly displayTimeZone: string;
}): JSX.Element {
  const [opened, setOpened] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [selection, setSelection] = createSignal<RecoverySelection>();
  const [selected, setSelected] = createSignal("");
  const [evidence, setEvidence] = createSignal<RecoveredAttempt>();
  const [message, setMessage] = createSignal("");
  let generation = 0;
  let action: HTMLButtonElement | undefined;
  let heading: HTMLHeadingElement | undefined;
  function clear(): void {
    generation += 1;
    setOpened(false);
    setBusy(false);
    setSelection(undefined);
    setSelected("");
    setEvidence(undefined);
    setMessage("");
  }
  createEffect(() => {
    void props.courseInstanceId;
    clear();
  });
  onCleanup(clear);
  function close(): void {
    clear();
    action?.focus();
  }
  function failure(error: unknown): void {
    // ASVS 16.5.1: do not disclose protocol, SQL or retained evidence in errors.
    setSelection(undefined);
    setSelected("");
    setEvidence(undefined);
    setMessage(
      error instanceof ApiRequestError && error.status === 404
        ? "Archived Student Work is unavailable through your current Course access or retention period."
        : "Archived Student Work could not be loaded. Try again.",
    );
  }
  async function load(cursor: string | null): Promise<void> {
    if (busy()) return;
    const token = ++generation;
    setOpened(true);
    setBusy(true);
    setMessage("");
    setSelected("");
    setEvidence(undefined);
    setSelection(undefined);
    try {
      const result = await props.client.selectArchivedStudentWork(props.courseInstanceId, cursor);
      if (token !== generation) return;
      setSelection(result);
      queueMicrotask(() => heading?.focus());
    } catch (error: unknown) {
      if (token === generation) failure(error);
    } finally {
      if (token === generation) setBusy(false);
    }
  }
  async function recover(): Promise<void> {
    const attempt = selected();
    if (busy() || !selection()?.attempts.some((item) => item.assessmentAttemptId === attempt))
      return;
    const token = ++generation;
    setBusy(true);
    setMessage("");
    setEvidence(undefined);
    try {
      const result = await props.client.recoverArchivedStudentWork(props.courseInstanceId, attempt);
      if (token !== generation) return;
      setEvidence(result);
    } catch (error: unknown) {
      if (token === generation) failure(error);
    } finally {
      if (token === generation) setBusy(false);
    }
  }
  return (
    <section aria-label="Archived Student Work recovery">
      <button
        ref={(element) => {
          action = element;
        }}
        type="button"
        class="quiet-button"
        disabled={busy()}
        onClick={() => void load(null)}
      >
        Recover archived Student Work
      </button>
      <Show when={opened()}>
        <section class="card" aria-busy={busy()}>
          <h2
            ref={(element) => {
              heading = element;
            }}
            tabIndex={-1}
          >
            Recover archived Student Work
          </h2>
          <p>
            Access retained evidence for this Course until its original deletion cutoff. Recovery
            does not restore ordinary access or extend retention.
          </p>
          <p>Times shown in your Instructor time zone: {props.displayTimeZone}.</p>
          <button type="button" class="quiet-button" onClick={close}>
            Close recovery
          </button>
          <Show when={busy()}>
            <p role="status">Loading archived Student Work...</p>
          </Show>
          <Show when={message() !== ""}>
            <p role="alert">{message()}</p>
          </Show>
          <Show when={message() !== ""}>
            <button type="button" disabled={busy()} onClick={() => void load(null)}>
              Try again
            </button>
          </Show>
          <Show when={selection()}>
            {(page) => (
              <>
                <Show
                  when={page().attempts.length > 0}
                  fallback={<p>No retained archived Attempts are available.</p>}
                >
                  <fieldset disabled={busy()}>
                    <legend>Select a retained Assessment Attempt</legend>
                    <For each={page().attempts}>
                      {(attempt) => (
                        <label
                          style={{
                            display: "block",
                            "margin-block": "0.75rem",
                            "overflow-wrap": "anywhere",
                          }}
                        >
                          <input
                            type="radio"
                            name="archived-student-work-attempt"
                            value={attempt.assessmentAttemptId}
                            checked={selected() === attempt.assessmentAttemptId}
                            onChange={() => {
                              setSelected(attempt.assessmentAttemptId);
                              setEvidence(undefined);
                            }}
                          />{" "}
                          {attempt.courseRosterTuple === null
                            ? "Roster ID not retained"
                            : `Roster ID: ${attempt.courseRosterTuple.rosterId}`}{" "}
                          | {attempt.assessmentTitle} | Attempt {attempt.assessmentAttemptNumber} (
                          {attempt.assessmentAttemptId})
                          <span style={{ display: "block" }}>
                            Started:{" "}
                            {formatRecoveryInstant(
                              attempt.startedAt,
                              props.displayTimeZone,
                              "No retained start time",
                            )}
                            ; submitted:{" "}
                            {formatRecoveryInstant(
                              attempt.submittedAt,
                              props.displayTimeZone,
                              "No retained submission time",
                            )}
                            ; deletion cutoff:{" "}
                            {formatRecoveryInstant(
                              attempt.deleteDueAt,
                              props.displayTimeZone,
                              "No retained deletion cutoff",
                            )}
                          </span>
                        </label>
                      )}
                    </For>
                  </fieldset>
                  <button
                    type="button"
                    class="primary-button"
                    disabled={busy() || selected() === ""}
                    onClick={() => void recover()}
                  >
                    Recover selected Attempt
                  </button>
                </Show>
                <Show when={page().nextCursor !== null}>
                  <button
                    type="button"
                    class="quiet-button"
                    disabled={busy()}
                    onClick={() => void load(page().nextCursor)}
                  >
                    Next retained Attempts
                  </button>
                </Show>
              </>
            )}
          </Show>
          <Show when={evidence()}>
            {(attempt) => (
              <section aria-label="Recovered Attempt evidence">
                <h3>
                  {attempt().assessmentTitle}: Attempt {attempt().assessmentAttemptNumber}
                </h3>
                <p role="status">
                  Retained evidence recovered. Original deletion cutoff:{" "}
                  {formatRecoveryInstant(
                    attempt().deleteDueAt,
                    props.displayTimeZone,
                    "No retained deletion cutoff",
                  )}
                  .
                </p>
                <p>
                  Archived:{" "}
                  {formatRecoveryInstant(
                    attempt().studentDataArchivedAt,
                    props.displayTimeZone,
                    "No retained archive time",
                  )}
                  ; started:{" "}
                  {formatRecoveryInstant(
                    attempt().startedAt,
                    props.displayTimeZone,
                    "No retained start time",
                  )}
                  ; expiry:{" "}
                  {formatRecoveryInstant(
                    attempt().expiresAt,
                    props.displayTimeZone,
                    "No retained expiry",
                  )}
                  .
                </p>
                <Evidence label="Exact Attempt facts" text={attempt().attemptFactsText} />
                <Evidence label="Submission" text={attempt().submissionText} />
                <For each={attempt().questions}>
                  {(question) => (
                    <section>
                      <h4>
                        Issued Question {question.issuedPosition + 1}:{" "}
                        {question.publishedQuestionRevisionTuple.publishedQuestionId}, Revision{" "}
                        {question.publishedQuestionRevisionTuple.revisionNumber}
                      </h4>
                      <Evidence label="Delivery and exact Revision" text={question.deliveryText} />
                      <Evidence label="Pool selection" text={question.poolText} />
                      <Evidence label="Question Attempt" text={question.attemptText} />
                      <Evidence label="Presentation" text={question.presentationText} />
                      <Evidence label="Reproduction evidence" text={question.reproductionText} />
                      <Evidence
                        label="Retained backend document (inert text)"
                        text={question.backendDocumentText}
                      />
                      <Evidence label="Saved response" text={question.savedResponseText} />
                      <Evidence label="Finalized response" text={question.finalizedResponseText} />
                      <Evidence label="Retained grading outcome" text={question.gradingText} />
                      <Show when={question.unavailableEvidence.length > 0}>
                        <p>Unavailable evidence:</p>
                        <ul>
                          <For each={question.unavailableEvidence}>{(item) => <li>{item}</li>}</For>
                        </ul>
                      </Show>
                    </section>
                  )}
                </For>
              </section>
            )}
          </Show>
        </section>
      </Show>
    </section>
  );
}
