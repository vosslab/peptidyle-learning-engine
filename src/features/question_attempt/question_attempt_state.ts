// question_attempt_state.ts - durable, question-agnostic browser state for one Student attempt.

import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { StudentFeedback } from "../../../generated/api/StudentFeedback";
import type { QuestionVariationPresentation } from "../../../generated/api/QuestionVariationPresentation";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { AssignmentAttemptId } from "../../../generated/api/AssignmentAttemptId";
import type { IssuedQuestionId } from "../../../generated/api/IssuedQuestionId";
import type { AssignmentAttemptCompletion } from "../../../generated/api/AssignmentAttemptCompletion";
import type { QuestionSeed } from "../../../generated/api/QuestionSeed";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type {
  GradedQuestionSubmissionReceipt,
  QuestionSubmissionAcknowledgement,
} from "../../api/contracts";
import type { FormatValidator } from "../../wasm/index";

export type IdempotencyKey = string;

export interface AttemptContext {
  readonly assignmentAttemptId: AssignmentAttemptId;
  readonly attemptId: QuestionAttemptId;
  /** Immutable selection identity issued with this attempt. */
  readonly issuedQuestionId: IssuedQuestionId;
  /** Exact immutable Question Revision selected for this attempt. */
  readonly questionRevision: QuestionRevisionReference;
  /** Question Seed that selects the exact issued Question Variation. */
  readonly seed: QuestionSeed;
  /** Unix milliseconds supplied by the server. A null deadline means untimed. */
  readonly deadline: number | null;
}

export interface AttemptBuffer {
  readonly response: StudentResponse;
  readonly idempotencyKey: IdempotencyKey;
}

export interface AttemptStorage {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

export interface AttemptNetwork {
  readonly isOnline: () => boolean;
}

export interface AttemptClock {
  readonly now: () => number;
}

export interface ResponseValidation {
  readonly valid: boolean;
  readonly message: string | null;
}

export type Feedback =
  | { readonly kind: "none" }
  | { readonly kind: "awaiting"; readonly feedback: null }
  | { readonly kind: "released"; readonly feedback: StudentFeedback };

/**
 * A transport acknowledgement deliberately separate from a grade result. The student state owns
 * only the fact that a response was accepted; disclosed feedback is projected separately.
 */
export interface SubmissionAcknowledgement {
  readonly accepted: true;
  readonly attemptId: QuestionAttemptId;
  /** Authoritative Assignment Attempt Completion, never inferred from successor availability. */
  readonly assignmentAttemptCompletion: GradedQuestionSubmissionReceipt["assignmentAttemptCompletion"];
  /** Immutable server-selected successor, if the submission has one. */
  readonly nextIssued: GradedQuestionSubmissionReceipt["nextIssued"];
  /** Receipt state that keeps feedback visible while successor issuance recovers. */
  readonly nextPending: GradedQuestionSubmissionReceipt["nextPending"];
  /** Currentness of the server-owned score projection. */
  readonly assignmentScoringState: GradedQuestionSubmissionReceipt["assignmentScoringState"];
}

/** Answer-free acknowledgement that permits a status read but never another answer POST. */
export interface PendingSubmissionAcknowledgement {
  readonly accepted: true;
  readonly attemptId: QuestionAttemptId;
  readonly gradingState: "pending" | "instructorAttention";
  readonly nextAction: "check_status";
}

/**
 * The explicit delivery result shared with a response controller.
 *
 * A completed promise is not itself evidence that the server accepted an answer:
 * the controller must distinguish durable acceptance, recovery that still owns a
 * retained replay, and a request or receipt refusal the student can correct.
 * ASVS 2.3.1: a response advances only after its accepted receipt, while each
 * recovery path retains its explicit sequential action.
 */
export type SubmissionOutcome =
  | { readonly kind: "accepted" }
  | {
      readonly kind: "recoveryPending";
      readonly reason: "offline" | "network" | "sessionExpired";
      readonly message: string;
    }
  | { readonly kind: "rejected"; readonly message: string };

interface StateBase {
  readonly context: AttemptContext;
  readonly response: StudentResponse | null;
  readonly validation: ResponseValidation;
  readonly remainingMilliseconds: number | null;
  readonly feedback: Feedback;
  readonly rendererFailure: string | null;
  /** Non-blocking notice that this response cannot survive a browser refresh. */
  readonly storageWarning: string | null;
  /** The prefetched, answer-free browser envelope for the current issued attempt. */
  readonly envelope: QuestionVariationPresentation | null;
}

type RecoveryReason =
  "offline" | "network" | "requestFailed" | "sessionExpired" | "advanceFailed" | "renderer";

export type QuestionAttemptExperienceState =
  | (StateBase & { readonly phase: "loading" })
  | (StateBase & { readonly phase: "answering" })
  | (StateBase & { readonly phase: "submitting"; readonly idempotencyKey: IdempotencyKey })
  | (StateBase & {
      readonly phase: "feedback";
      readonly acknowledgement: SubmissionAcknowledgement;
      readonly checkingStatus: boolean;
      readonly statusMessage: string | null;
    })
  | (StateBase & {
      readonly phase: "acceptedPending";
      readonly acknowledgement: PendingSubmissionAcknowledgement;
      readonly checkingStatus: boolean;
      readonly statusMessage: string | null;
    })
  | (StateBase & { readonly phase: "advancing" })
  | (StateBase & {
      readonly phase: "recovering";
      readonly reason: RecoveryReason;
      readonly message: string;
    })
  | (StateBase & { readonly phase: "expired"; readonly reason: "missingOrInvalidResponse" })
  | (StateBase & {
      readonly phase: "terminal";
      readonly assignmentAttemptCompletion: AssignmentAttemptCompletion;
    });

export interface QuestionAttemptStateMachine {
  readonly state: () => QuestionAttemptExperienceState;
  /** Starts one issued attempt, validating any saved response against its exact issued definition. */
  readonly start: (definition?: QuestionResponseFormat) => void;
  readonly setResponse: (response: StudentResponse, validation: ResponseValidation) => void;
  readonly submit: () => Promise<SubmissionOutcome>;
  readonly retry: () => Promise<SubmissionOutcome>;
  /** Call when connectivity returns to perform the documented automatic retry. */
  readonly retryWhenOnline: () => Promise<void>;
  /** Reads the acknowledgement status only; it never repeats the student's answer POST. */
  readonly checkGradingStatus: () => Promise<void>;
  /** Retries only loading a prefetched next envelope after a committed submission. */
  readonly retryAdvance: () => Promise<void>;
  readonly resumeAfterReauthentication: () => void;
  readonly reportRendererFailure: (message: string) => void;
  readonly retryRenderer: () => void;
  readonly tick: () => void;
  readonly advance: (loadNext: () => Promise<NextAttempt>) => Promise<void>;
  readonly finish: (assignmentAttemptCompletion: AssignmentAttemptCompletion) => void;
  readonly dispose: () => void;
}

export interface NextAttempt {
  readonly context: AttemptContext;
  readonly envelope: QuestionVariationPresentation;
}

export interface QuestionAttemptStateMachineOptions {
  readonly context: AttemptContext;
  readonly storage: AttemptStorage;
  readonly clock: AttemptClock;
  readonly network: AttemptNetwork;
  readonly generateIdempotencyKey: () => IdempotencyKey;
  readonly submitResponse: (
    attemptId: QuestionAttemptId,
    response: StudentResponse,
    idempotencyKey: IdempotencyKey,
  ) => Promise<QuestionSubmissionAcknowledgement>;
  /** Route-bound status reader injected by the owning Student page/client composition. */
  readonly getSubmissionStatus: (
    attemptId: QuestionAttemptId,
  ) => Promise<QuestionSubmissionAcknowledgement>;
  /** Recognizes an authentication failure without coupling this state to an HTTP implementation. */
  readonly isSessionExpired: (error: unknown) => boolean;
  /**
   * Recognizes a browser transport failure without coupling attempt state to an HTTP client.
   * Request refusals and response-contract failures remain actionable Student errors instead.
   */
  readonly isTransientTransportFailure: (error: unknown) => boolean;
  /** Browser-safe semantic validation for a persisted response before it reaches a controlled UI. */
  readonly validateSavedResponse?: FormatValidator;
  readonly onStateChange?: (state: QuestionAttemptExperienceState) => void;
}

function emptyValidation(): ResponseValidation {
  return { valid: false, message: null };
}

function bufferKey(context: AttemptContext): string {
  return `ple:attempt:${context.assignmentAttemptId}:${context.attemptId}`;
}

function remainingMilliseconds(context: AttemptContext, now: number): number | null {
  if (context.deadline === null) return null;
  return Math.max(0, context.deadline - now);
}

function initialState(
  context: AttemptContext,
  clock: AttemptClock,
  envelope: QuestionVariationPresentation | null = null,
): QuestionAttemptExperienceState {
  return {
    phase: "loading",
    context,
    response: null,
    validation: emptyValidation(),
    remainingMilliseconds: remainingMilliseconds(context, clock.now()),
    feedback: { kind: "none" },
    rendererFailure: null,
    storageWarning: null,
    envelope,
  };
}

function parseBuffer(value: string | null): AttemptBuffer | null {
  if (value === null) return null;
  try {
    const candidate: unknown = JSON.parse(value);
    if (typeof candidate !== "object" || candidate === null) return null;
    if (!hasBufferFields(candidate)) return null;
    if (typeof candidate.idempotencyKey !== "string" || !isStudentResponse(candidate.response)) {
      return null;
    }
    return { response: candidate.response, idempotencyKey: candidate.idempotencyKey };
  } catch {
    return null;
  }
}

function hasBufferFields(
  value: object,
): value is { readonly response?: unknown; readonly idempotencyKey?: unknown } {
  return "response" in value && "idempotencyKey" in value;
}

function isStringArray(value: unknown): value is Array<string> {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isRecordArray(value: unknown): value is Array<Record<string, unknown>> {
  return Array.isArray(value) && value.every((item) => typeof item === "object" && item !== null);
}

function isStudentResponse(value: unknown): value is StudentResponse {
  if (typeof value !== "object" || value === null || !("kind" in value)) return false;
  const keys = Object.keys(value);
  if (value.kind === "numeric") {
    return keys.length === 2 && "value" in value && typeof value.value === "number";
  }
  if (value.kind === "multipleChoice") {
    return keys.length === 2 && "selected" in value && isStringArray(value.selected);
  }
  if (value.kind === "shortText") {
    return keys.length === 2 && "text" in value && typeof value.text === "string";
  }
  if (value.kind === "multiBlank") {
    return (
      keys.length === 2 &&
      "answers" in value &&
      isRecordArray(value.answers) &&
      value.answers.every(
        (answer) =>
          Object.keys(answer).length === 2 &&
          typeof answer.slot === "string" &&
          typeof answer.text === "string",
      )
    );
  }
  if (value.kind === "matching") {
    return (
      keys.length === 2 &&
      "matches" in value &&
      isRecordArray(value.matches) &&
      value.matches.every(
        (pair) =>
          Object.keys(pair).length === 2 &&
          typeof pair.prompt === "string" &&
          typeof pair.choice === "string",
      )
    );
  }
  if (value.kind === "ordering") {
    return keys.length === 2 && "order" in value && isStringArray(value.order);
  }
  if (value.kind === "hotspot") {
    return (
      keys.length === 2 &&
      "selections" in value &&
      isRecordArray(value.selections) &&
      value.selections.every(
        (selection) => Object.keys(selection).length === 1 && typeof selection.region === "string",
      )
    );
  }
  return value.kind === "externalTool" && keys.length === 1;
}

function serializeBuffer(buffer: AttemptBuffer): string {
  return JSON.stringify(buffer);
}

/** Canonical comparison keeps retries stable while giving edited answers a fresh replay key. */
function responsesEqual(left: StudentResponse, right: StudentResponse): boolean {
  if (left.kind !== right.kind) return false;
  if (left.kind === "numeric" && right.kind === "numeric") return left.value === right.value;
  if (left.kind === "multipleChoice" && right.kind === "multipleChoice") {
    const leftSelected = [...left.selected].sort();
    const rightSelected = [...right.selected].sort();
    return JSON.stringify(leftSelected) === JSON.stringify(rightSelected);
  }
  if (left.kind === "shortText" && right.kind === "shortText") return left.text === right.text;
  if (left.kind === "multiBlank" && right.kind === "multiBlank") {
    return JSON.stringify(left.answers) === JSON.stringify(right.answers);
  }
  if (left.kind === "matching" && right.kind === "matching") {
    return JSON.stringify(left.matches) === JSON.stringify(right.matches);
  }
  if (left.kind === "ordering" && right.kind === "ordering") {
    return JSON.stringify(left.order) === JSON.stringify(right.order);
  }
  if (left.kind === "hotspot" && right.kind === "hotspot") {
    return JSON.stringify(left.selections) === JSON.stringify(right.selections);
  }
  if (left.kind === "externalTool" && right.kind === "externalTool") return true;
  return false;
}

function feedbackFor(receipt: GradedQuestionSubmissionReceipt): Feedback {
  return receipt.feedback === null
    ? { kind: "awaiting", feedback: null }
    : { kind: "released", feedback: receipt.feedback };
}

function pendingAcknowledgement(
  status: Exclude<QuestionSubmissionAcknowledgement, { readonly gradingState: "graded" }>,
): PendingSubmissionAcknowledgement {
  return {
    accepted: status.receipt.accepted,
    attemptId: status.receipt.attemptId,
    gradingState: status.gradingState,
    nextAction: status.nextAction,
  };
}

function completedAcknowledgement(
  status: Extract<QuestionSubmissionAcknowledgement, { readonly gradingState: "graded" }>,
): SubmissionAcknowledgement {
  return {
    accepted: true,
    attemptId: status.receipt.attempt.id,
    assignmentAttemptCompletion: status.receipt.assignmentAttemptCompletion,
    nextIssued: status.receipt.nextIssued,
    nextPending: status.receipt.nextPending,
    assignmentScoringState: status.receipt.assignmentScoringState,
  };
}

function messageFor(error: unknown): string {
  return error instanceof Error ? error.message : "The request could not be completed.";
}

/**
 * Recovery copy follows the classified failure, so browser transport details never replace the
 * student's durable next action while server and protocol failures retain their useful message.
 */
function recoveryMessageFor(reason: RecoveryReason, error: unknown): string {
  switch (reason) {
    case "offline":
      return "Your response is retained in this browser. Reconnect, then retry submission.";
    case "network":
      return "Your response is retained in this browser. Retry submission after the service is restored.";
    case "requestFailed":
    case "sessionExpired":
    case "advanceFailed":
    case "renderer":
      return messageFor(error);
  }
}

function envelopeMatchesContext(
  envelope: QuestionVariationPresentation,
  context: AttemptContext,
): boolean {
  return (
    envelope.variation.seed === context.seed &&
    envelope.variation.questionRevision.questionId === context.questionRevision.questionId &&
    envelope.variation.questionRevision.revisionNumber === context.questionRevision.revisionNumber
  );
}

/**
 * Owns the browser's durable response buffer and retry state. It deliberately does not know a
 * Question Type, page layout, answer key, or grading rule.
 */
export function createQuestionAttemptStateMachine(
  options: QuestionAttemptStateMachineOptions,
): QuestionAttemptStateMachine {
  let context = options.context;
  let current = initialState(context, options.clock);
  let disposed = false;
  let requestNumber = 0;
  let memoryBuffer: AttemptBuffer | null = null;
  let storageWarning: string | null = null;
  let nextLoader: (() => Promise<NextAttempt>) | null = null;
  let deadlineAutomaticSubmissionStarted = false;

  function publish(next: QuestionAttemptExperienceState): void {
    if (disposed) return;
    current = next;
    options.onStateChange?.(next);
  }

  function base(overrides: Partial<Omit<StateBase, "context">> = {}): StateBase {
    const base = {
      context,
      response: current.response,
      validation: current.validation,
      remainingMilliseconds: remainingMilliseconds(context, options.clock.now()),
      feedback: current.feedback,
      rendererFailure: current.rendererFailure,
      storageWarning,
      envelope: current.envelope,
      ...overrides,
    };
    return base;
  }

  function savedBuffer(): AttemptBuffer | null {
    try {
      const stored = parseBuffer(options.storage.getItem(bufferKey(context)));
      if (stored !== null) memoryBuffer = stored;
      return stored ?? memoryBuffer;
    } catch {
      storageWarning = "Your response will not be saved if this page is refreshed.";
      return memoryBuffer;
    }
  }

  function saveBuffer(response: StudentResponse): AttemptBuffer {
    const existing = savedBuffer();
    const idempotencyKey =
      existing !== null && responsesEqual(existing.response, response)
        ? existing.idempotencyKey
        : options.generateIdempotencyKey();
    const buffer = { response, idempotencyKey };
    memoryBuffer = buffer;
    try {
      options.storage.setItem(bufferKey(context), serializeBuffer(buffer));
    } catch {
      storageWarning = "Your response will not be saved if this page is refreshed.";
    }
    return buffer;
  }

  function clearBuffer(): void {
    memoryBuffer = null;
    try {
      options.storage.removeItem(bufferKey(context));
    } catch {
      storageWarning =
        "Your response was accepted, but this browser could not clear its local copy.";
    }
  }

  function discardUnsafeBuffer(): void {
    memoryBuffer = null;
    try {
      options.storage.removeItem(bufferKey(context));
      storageWarning =
        "A saved response was not valid for this question and was removed. Enter a new response.";
    } catch {
      storageWarning =
        "A saved response was not valid for this question and will not be used. Enter a new response.";
    }
  }

  function publishAnswering(
    response: StudentResponse | null,
    validation = emptyValidation(),
  ): void {
    const state = {
      ...base({ response, validation, feedback: { kind: "none" }, rendererFailure: null }),
      phase: "answering" as const,
    } satisfies QuestionAttemptExperienceState;
    publish(state);
    tick();
  }

  async function validateSavedBuffer(
    buffer: AttemptBuffer,
    definition: QuestionResponseFormat,
  ): Promise<void> {
    try {
      const report = await options.validateSavedResponse!(definition, buffer.response);
      if (disposed) return;
      if (report.violations.length > 0) {
        discardUnsafeBuffer();
        publishAnswering(null);
        return;
      }
      publishAnswering(buffer.response, { valid: true, message: null });
    } catch {
      if (disposed) return;
      discardUnsafeBuffer();
      publishAnswering(null);
    }
  }

  function start(definition?: QuestionResponseFormat): void {
    const buffer = savedBuffer();
    if (
      buffer !== null &&
      definition !== undefined &&
      options.validateSavedResponse !== undefined
    ) {
      void validateSavedBuffer(buffer, definition);
      return;
    }
    publishAnswering(buffer?.response ?? null);
  }

  function setResponse(response: StudentResponse, validation: ResponseValidation): void {
    if (current.phase !== "answering" && current.phase !== "recovering") return;
    if (current.phase === "recovering" && current.reason === "advanceFailed") return;
    saveBuffer(response);
    if (
      current.phase === "recovering" &&
      (current.reason === "offline" || current.reason === "network")
    ) {
      const state = {
        ...base({ response, validation, feedback: { kind: "none" } }),
        phase: "recovering" as const,
        reason: current.reason,
        message: current.message,
      } satisfies QuestionAttemptExperienceState;
      publish(state);
      return;
    }
    const state = {
      ...base({ response, validation, feedback: { kind: "none" } }),
      phase: "answering" as const,
    } satisfies QuestionAttemptExperienceState;
    publish(state);
  }

  function rejected(message: string): SubmissionOutcome {
    return { kind: "rejected", message };
  }

  function recoveryPending(
    reason: "offline" | "network" | "sessionExpired",
    message: string,
  ): SubmissionOutcome {
    return { kind: "recoveryPending", reason, message };
  }

  async function submitBuffered(allowExpired: boolean): Promise<SubmissionOutcome> {
    if (
      current.phase === "submitting" ||
      current.phase === "terminal" ||
      current.phase === "feedback" ||
      current.phase === "acceptedPending" ||
      current.phase === "advancing" ||
      (current.phase === "recovering" && current.reason === "advanceFailed")
    ) {
      return rejected("This response cannot be submitted from its current state.");
    }
    const response = current.response;
    if (response === null || !current.validation.valid) {
      if (allowExpired) {
        const state = {
          ...base({ remainingMilliseconds: 0 }),
          phase: "expired" as const,
          reason: "missingOrInvalidResponse" as const,
        } satisfies QuestionAttemptExperienceState;
        publish(state);
      }
      return rejected("Response format needs attention before submission.");
    }
    const buffer = saveBuffer(response);
    if (!options.network.isOnline()) {
      const state = {
        ...base({ response }),
        phase: "recovering" as const,
        reason: "offline" as const,
        message: recoveryMessageFor("offline", null),
      } satisfies QuestionAttemptExperienceState;
      publish(state);
      return recoveryPending("offline", state.message);
    }
    requestNumber += 1;
    const request = requestNumber;
    const submitting = {
      ...base({ response }),
      phase: "submitting" as const,
      idempotencyKey: buffer.idempotencyKey,
    } satisfies QuestionAttemptExperienceState;
    publish(submitting);
    try {
      const status = await options.submitResponse(
        context.attemptId,
        response,
        buffer.idempotencyKey,
      );
      if (disposed || request !== requestNumber) {
        return rejected("This response is no longer current.");
      }
      clearBuffer();
      if (status.gradingState !== "graded") {
        const state = {
          ...base({ response, feedback: { kind: "none" } }),
          phase: "acceptedPending" as const,
          acknowledgement: pendingAcknowledgement(status),
          checkingStatus: false,
          statusMessage: null,
        } satisfies QuestionAttemptExperienceState;
        publish(state);
        return { kind: "accepted" };
      }
      const feedback = feedbackFor(status.receipt);
      const acknowledgement = completedAcknowledgement(status);
      const state = {
        ...base({ response, feedback }),
        phase: "feedback" as const,
        acknowledgement,
        checkingStatus: false,
        statusMessage: null,
      } satisfies QuestionAttemptExperienceState;
      publish(state);
      return { kind: "accepted" };
    } catch (error: unknown) {
      if (disposed || request !== requestNumber) {
        return rejected("This response is no longer current.");
      }
      const sessionExpired = options.isSessionExpired(error);
      const offline = !options.network.isOnline();
      const transientTransportFailure = options.isTransientTransportFailure(error);
      const reason = sessionExpired
        ? "sessionExpired"
        : offline
          ? "offline"
          : transientTransportFailure
            ? "network"
            : "requestFailed";
      const state = {
        ...base({ response }),
        phase: "recovering" as const,
        reason,
        message: recoveryMessageFor(reason, error),
      } satisfies QuestionAttemptExperienceState;
      publish(state);
      if (reason === "requestFailed") return rejected(state.message);
      return recoveryPending(reason, state.message);
    }
  }

  async function submit(): Promise<SubmissionOutcome> {
    if (current.phase === "expired") {
      return rejected("This attempt has expired.");
    }
    if (current.phase === "recovering" && current.reason === "advanceFailed") {
      return rejected("The next question is still being recovered.");
    }
    const timedOut =
      context.deadline !== null && remainingMilliseconds(context, options.clock.now()) === 0;
    return submitBuffered(timedOut);
  }

  async function retry(): Promise<SubmissionOutcome> {
    if (
      current.phase !== "recovering" ||
      (current.reason !== "offline" && current.reason !== "network")
    ) {
      return rejected("This response is not waiting for a transport retry.");
    }
    if (!options.network.isOnline()) {
      return recoveryPending("offline", current.message);
    }
    return submit();
  }

  async function retryWhenOnline(): Promise<void> {
    await retry();
  }

  async function checkGradingStatus(): Promise<void> {
    const checkingStatus =
      current.phase === "acceptedPending" || current.phase === "feedback"
        ? current.checkingStatus
        : false;
    const canCheck =
      (current.phase === "acceptedPending" ||
        (current.phase === "feedback" &&
          current.acknowledgement.assignmentScoringState !== "current")) &&
      !checkingStatus;
    if (!canCheck) return;
    const pending = current as Extract<
      QuestionAttemptExperienceState,
      { readonly phase: "acceptedPending" | "feedback" }
    >;
    requestNumber += 1;
    const request = requestNumber;
    publish({ ...pending, checkingStatus: true, statusMessage: null });
    try {
      const status = await options.getSubmissionStatus(context.attemptId);
      if (disposed || request !== requestNumber) return;
      if (status.gradingState !== "graded") {
        publish({
          ...base({ response: pending.response, feedback: { kind: "none" } }),
          phase: "acceptedPending",
          acknowledgement: pendingAcknowledgement(status),
          checkingStatus: false,
          statusMessage: null,
        });
        return;
      }
      const feedback = feedbackFor(status.receipt);
      const acknowledgement = completedAcknowledgement(status);
      publish({
        ...base({ response: pending.response, feedback }),
        phase: "feedback",
        acknowledgement,
        checkingStatus: false,
        statusMessage: null,
      });
    } catch (error: unknown) {
      if (disposed || request !== requestNumber) return;
      publish({ ...pending, checkingStatus: false, statusMessage: messageFor(error) });
    }
  }

  async function retryAdvance(): Promise<void> {
    if (
      current.phase !== "recovering" ||
      current.reason !== "advanceFailed" ||
      nextLoader === null
    ) {
      return;
    }
    await loadNextAttempt(nextLoader);
  }

  function resumeAfterReauthentication(): void {
    if (current.phase !== "recovering" || current.reason !== "sessionExpired") return;
    const state = {
      ...base(),
      phase: "answering" as const,
    } satisfies QuestionAttemptExperienceState;
    publish(state);
  }

  function reportRendererFailure(message: string): void {
    if (current.phase === "terminal") return;
    const state = {
      ...base({ rendererFailure: message }),
      phase: "recovering" as const,
      reason: "renderer" as const,
      message,
    } satisfies QuestionAttemptExperienceState;
    publish(state);
  }

  function retryRenderer(): void {
    if (current.phase !== "recovering" || current.reason !== "renderer") return;
    const state = {
      ...base({ rendererFailure: null }),
      phase: "answering" as const,
    } satisfies QuestionAttemptExperienceState;
    publish(state);
  }

  function tick(): void {
    if (
      current.phase === "terminal" ||
      current.phase === "feedback" ||
      current.phase === "acceptedPending" ||
      current.phase === "advancing"
    )
      return;
    const remaining = remainingMilliseconds(context, options.clock.now());
    if (remaining === 0 && context.deadline !== null) {
      if (deadlineAutomaticSubmissionStarted) return;
      // The deadline itself gets one automatic delivery attempt. Recovery can then retry the
      // same buffered key only through an explicit student or connectivity action.
      deadlineAutomaticSubmissionStarted = true;
      void submitBuffered(true);
      return;
    }
    publish({ ...current, remainingMilliseconds: remaining });
  }

  async function advance(loadNext: () => Promise<NextAttempt>): Promise<void> {
    if (current.phase !== "feedback") return;
    nextLoader = loadNext;
    await loadNextAttempt(loadNext);
  }

  async function loadNextAttempt(loadNext: () => Promise<NextAttempt>): Promise<void> {
    const priorContext = context;
    const advancing = {
      ...base(),
      phase: "advancing" as const,
    } satisfies QuestionAttemptExperienceState;
    publish(advancing);
    try {
      const next = await loadNext();
      if (disposed) return;
      if (!envelopeMatchesContext(next.envelope, next.context)) {
        throw new Error("The next question did not match its issued attempt. Please retry.");
      }
      context = next.context;
      memoryBuffer = null;
      deadlineAutomaticSubmissionStarted = false;
      nextLoader = null;
      current = initialState(context, options.clock, next.envelope);
      start(next.envelope.response);
    } catch (error: unknown) {
      if (disposed) return;
      context = priorContext;
      const state = {
        ...base(),
        phase: "recovering" as const,
        reason: "advanceFailed" as const,
        message: recoveryMessageFor("advanceFailed", error),
      } satisfies QuestionAttemptExperienceState;
      publish(state);
    }
  }

  function finish(assignmentAttemptCompletion: AssignmentAttemptCompletion): void {
    clearBuffer();
    const state = {
      ...base({ response: null, feedback: { kind: "none" } }),
      phase: "terminal" as const,
      assignmentAttemptCompletion,
    } satisfies QuestionAttemptExperienceState;
    publish(state);
  }

  function dispose(): void {
    disposed = true;
    requestNumber += 1;
    clearBuffer();
  }

  return {
    state: () => current,
    start,
    setResponse,
    submit,
    retry,
    retryWhenOnline,
    checkGradingStatus,
    retryAdvance,
    resumeAfterReauthentication,
    reportRendererFailure,
    retryRenderer,
    tick,
    advance,
    finish,
    dispose,
  };
}
