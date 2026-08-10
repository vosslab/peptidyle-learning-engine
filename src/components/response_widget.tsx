// response_widget.tsx - question-agnostic, browser-safe response entry controls.

import { createEffect, createSignal, For, on, onCleanup, onMount, Show, type JSX } from "solid-js";

import type { ChoiceId } from "../../generated/api/ChoiceId";
import type { ChoiceOption } from "../../generated/api/ChoiceOption";
import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { ExternalToolLaunch } from "../api/contracts";
import type { ResponseFormatReport, ResponseFormatViolation, WasmFacade } from "../wasm/index";

import { RESPONSE_WIDGET_STYLES } from "./response_widget_styles";

type MultipleChoiceDefinition = Extract<ResponseDefinition, { kind: "multipleChoice" }>;
type NumericDefinition = Extract<ResponseDefinition, { kind: "numeric" }>;
type ShortTextDefinition = Extract<ResponseDefinition, { kind: "shortText" }>;
type OrderingDefinition = Extract<ResponseDefinition, { kind: "ordering" }>;
type FileUploadDefinition = Extract<ResponseDefinition, { kind: "fileUpload" }>;
type ExternalToolDefinition = Extract<ResponseDefinition, { kind: "externalTool" }>;

type WidgetPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "validating" }
  | { readonly kind: "ready" }
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "submitting" }
  | { readonly kind: "submitted" }
  | { readonly kind: "failed"; readonly message: string };

interface ResponseWidgetBaseProps {
  readonly attemptId: string;
  readonly validator: WasmFacade;
  readonly onSubmit: (response: StudentResponse) => Promise<void>;
  readonly onEscape: () => void;
  /** Lets the attempt controller retain a refresh-safe, key-free draft response. */
  readonly onResponseChange?: (response: StudentResponse, validation: ResponseFormatReport) => void;
  /**
   * Gets the protected server-owned launch route only after the learner asks
   * to open the tool. The callback never returns provider material.
   */
  readonly getExternalToolLaunch?: () => Promise<ExternalToolLaunch>;
}

type ExternalToolPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "awaitingReady" }
  | { readonly kind: "ready" }
  | { readonly kind: "failed"; readonly message: string }
  | { readonly kind: "submitting" }
  | { readonly kind: "submitted" };

interface ExternalToolReadyMessage {
  readonly kind: "ple.externalTool.ready";
  readonly attemptId: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Only a field-exact, attempt-bound readiness event crosses the frame boundary. */
export function isExternalToolReadyMessage(
  value: unknown,
  attemptId: string,
): value is ExternalToolReadyMessage {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value);
  return (
    keys.length === 2 &&
    keys.includes("kind") &&
    keys.includes("attemptId") &&
    value["kind"] === "ple.externalTool.ready" &&
    value["attemptId"] === attemptId
  );
}

/** Reject route-state that could navigate an embedded learner surface off-origin. */
export function isSafeExternalToolLaunchPath(launchUrl: string): boolean {
  if (
    !launchUrl.startsWith("/") ||
    launchUrl.startsWith("//") ||
    launchUrl.includes("?") ||
    launchUrl.includes("#")
  ) {
    return false;
  }
  const parsed = new URL(launchUrl, window.location.origin);
  return (
    parsed.origin === window.location.origin &&
    parsed.username === "" &&
    parsed.password === "" &&
    parsed.search === "" &&
    parsed.hash === ""
  );
}

/**
 * The saved response remains a generated, key-free StudentResponse. The dispatcher below narrows
 * it against the definition before a concrete control receives it.
 */
export interface ResponseWidgetProps extends ResponseWidgetBaseProps {
  readonly definition: ResponseDefinition;
  readonly initialResponse?: StudentResponse;
}

export type MultipleChoiceResponseProps = WidgetBodyProps<MultipleChoiceDefinition>;

interface WidgetBodyProps<D extends ResponseDefinition> extends ResponseWidgetBaseProps {
  readonly definition: D;
  /** Saved state from the attempt controller, visible before the learner makes another edit. */
  readonly initialResponse?: Extract<StudentResponse, { readonly kind: D["kind"] }>;
}

interface SubmissionController {
  readonly phase: () => WidgetPhase;
  readonly invalid: () => boolean;
  readonly pending: () => boolean;
  readonly canSubmit: () => boolean;
  readonly validate: (response: StudentResponse) => Promise<void>;
  readonly submit: (response: StudentResponse) => Promise<void>;
}

function textFromBlocks(blocks: ReadonlyArray<ContentBlock>): string {
  return blocks
    .map((block) => {
      switch (block.kind) {
        case "text":
          return block.markdown;
        case "math":
        case "image":
        case "table":
          return block.description;
        case "code":
          return block.source;
      }
    })
    .join(" ");
}

function violationMessage(violation: ResponseFormatViolation): string {
  switch (violation.kind) {
    case "selectionCount":
      return "Choose the requested number of responses.";
    case "duplicateChoice":
      return "Each response may be selected only once.";
    case "unknownChoice":
      return "That response is not available for this question.";
    case "numericNotFinite":
      return "Enter a finite number.";
    case "textTooLong":
      return `Keep the response within ${violation.maxLength} characters.`;
    case "orderingItemsMismatch":
      return "Place every item in the requested order.";
    case "missingUploadReference":
      return "Choose an uploaded file before submitting.";
    case "responseKindMismatch":
      return "This response does not match the question format.";
  }
}

function reportMessage(report: ResponseFormatReport): string {
  const first = report.violations[0];
  return first === undefined ? "Response format is ready to submit." : violationMessage(first);
}

/** Browser-only format check: deliberately has no submit or grading dependency. */
export async function validateResponseLocally(
  validator: WasmFacade,
  definition: ResponseDefinition,
  response: StudentResponse,
): Promise<ResponseFormatReport> {
  return validator.validateResponseFormat(definition, response);
}

/**
 * Preserve an empty numeric control as an invalid response rather than letting JavaScript coerce
 * it to zero. The exact same parser is used for validation and for a submit attempt.
 */
export function numericResponseFromInput(input: string): StudentResponse {
  return {
    kind: "numeric",
    value: input.trim() === "" ? Number.NaN : Number(input),
  };
}

function phaseMessage(phase: WidgetPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Complete the response, then submit it.";
    case "validating":
      return "Checking response format...";
    case "ready":
      return "Response format is ready to submit.";
    case "invalid":
    case "failed":
      return phase.message;
    case "submitting":
      return "Submitting your response. Please wait.";
    case "submitted":
      return "Answer submitted. Server feedback will appear when it is released.";
  }
}

/** Key-free validation state machine. Validation never invokes server grading. */
export function createSubmissionController(
  props: ResponseWidgetProps,
  initialResponse?: StudentResponse,
): SubmissionController {
  const [phase, setPhase] = createSignal<WidgetPhase>({ kind: "idle" });
  let validationRequest = 0;
  let submissionRequest = 0;

  async function validate(response: StudentResponse): Promise<void> {
    if (phase().kind === "submitting") {
      return;
    }
    validationRequest += 1;
    const request = validationRequest;
    setPhase({ kind: "validating" });
    try {
      const report = await validateResponseLocally(props.validator, props.definition, response);
      if (request !== validationRequest || phase().kind === "submitting") {
        return;
      }
      props.onResponseChange?.(response, report);
      setPhase(
        report.violations.length === 0
          ? { kind: "ready" }
          : { kind: "invalid", message: reportMessage(report) },
      );
    } catch (error: unknown) {
      if (request !== validationRequest || phase().kind === "submitting") {
        return;
      }
      const message = error instanceof Error ? error.message : "format validation was unavailable";
      setPhase({ kind: "failed", message: `Cannot check this response yet: ${message}.` });
    }
  }

  async function submit(response: StudentResponse): Promise<void> {
    if (phase().kind !== "ready") {
      await validate(response);
      if (phase().kind !== "ready") {
        return;
      }
    }
    submissionRequest += 1;
    const request = submissionRequest;
    setPhase({ kind: "submitting" });
    try {
      await props.onSubmit(response);
      if (request === submissionRequest) {
        setPhase({ kind: "submitted" });
      }
    } catch (error: unknown) {
      if (request !== submissionRequest) {
        return;
      }
      const message =
        error instanceof Error
          ? `Your response is still available. Submission failed: ${error.message}. Try again.`
          : "Your response is still available. Submission failed. Try again.";
      setPhase({ kind: "failed", message });
    }
  }

  // Each controlled widget supplies its first response here so a valid pre-populated answer is
  // ready without a redundant edit. This is still format-only validation; it never grades.
  if (initialResponse !== undefined) {
    void validate(initialResponse);
  }

  return {
    phase,
    invalid: () => phase().kind === "invalid" || phase().kind === "failed",
    pending: () => phase().kind === "submitting",
    canSubmit: () => phase().kind === "ready",
    validate,
    submit,
  };
}

function Status(props: {
  readonly attemptId: string;
  readonly controller: SubmissionController;
}): JSX.Element {
  return (
    <p
      id={`${props.attemptId}-format-status`}
      class="format-status"
      classList={{
        error: props.controller.invalid(),
        ready:
          props.controller.phase().kind === "ready" ||
          props.controller.phase().kind === "submitted",
      }}
      role="status"
      aria-label="Response format"
      aria-live="polite"
    >
      {phaseMessage(props.controller.phase())}
    </p>
  );
}

function Actions(props: {
  readonly disabled: boolean;
  readonly onSubmit: () => void;
  readonly onEscape: () => void;
}): JSX.Element {
  return (
    <>
      <button
        class="primary-action"
        type="button"
        disabled={props.disabled}
        onClick={props.onSubmit}
      >
        Submit answer
      </button>
      <button class="quiet-action" type="button" onClick={props.onEscape}>
        Return to assignment <span aria-hidden="true">(Esc)</span>
      </button>
    </>
  );
}

/**
 * The response-level shortcuts deliberately opt in only to the controls that are answer entry
 * fields in this component. Events from buttons, links, textareas, selects, or future embedded
 * interactive content retain their native keyboard semantics.
 */
function isResponseEntryTarget(target: EventTarget | null): target is HTMLInputElement {
  if (!(target instanceof HTMLInputElement)) return false;
  return target.type === "number" || target.type === "radio" || target.type === "checkbox";
}

function isInsideNativeDialog(target: EventTarget | null): boolean {
  return (
    typeof Element !== "undefined" &&
    target instanceof Element &&
    target.closest("dialog, [role='dialog']") !== null
  );
}

/**
 * Escape returns to the assignment from any widget descendant, except while an IME composition or
 * a native dialog is handling that key. Enter remains an opt-in response-entry shortcut.
 */
export function handleWidgetKeyDown(
  event: KeyboardEvent,
  onEscape: () => void,
  submit: () => void,
  canSubmit: () => boolean,
): void {
  if (event.defaultPrevented || event.isComposing) return;

  if (event.key === "Escape") {
    if (isInsideNativeDialog(event.target)) return;
    event.preventDefault();
    onEscape();
    return;
  }

  if (event.key === "Enter" && isResponseEntryTarget(event.target) && canSubmit()) {
    event.preventDefault();
    submit();
  }
}

function choiceInputType(definition: MultipleChoiceDefinition): "radio" | "checkbox" {
  return definition.selection.kind === "exactlyOne" ? "radio" : "checkbox";
}

/** Controlled multiple-choice widget. It validates shape only; grading stays server-only. */
function MultipleChoiceBody(props: MultipleChoiceResponseProps): JSX.Element {
  const restored = props.initialResponse?.selected ?? [];
  const [selected, setSelected] = createSignal<ReadonlyArray<ChoiceId>>(restored);
  const initialResponse: StudentResponse = {
    kind: "multipleChoice",
    selected: [...restored],
  };
  const controller = createSubmissionController(props, initialResponse);
  const response = (): StudentResponse => ({ kind: "multipleChoice", selected: [...selected()] });

  function choose(choice: ChoiceId): void {
    if (controller.pending()) return;
    const next =
      props.definition.selection.kind === "exactlyOne"
        ? [choice]
        : selected().includes(choice)
          ? selected().filter((item) => item !== choice)
          : [...selected(), choice];
    setSelected(next);
    void controller.validate({ kind: "multipleChoice", selected: [...next] });
  }
  function submit(): void {
    void controller.submit(response());
  }
  function handleKeyDown(event: KeyboardEvent): void {
    const target = event.target;
    if (
      target instanceof HTMLInputElement &&
      target.type === "checkbox" &&
      ["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft"].includes(event.key)
    ) {
      const current = props.definition.choices.findIndex((choice) => choice.id === target.value);
      const direction = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
      const next =
        (current + direction + props.definition.choices.length) % props.definition.choices.length;
      event.preventDefault();
      document.getElementById(`${props.attemptId}-choice-${next}`)?.focus();
      return;
    }
    if (/^[1-9]$/.test(event.key)) {
      const index = Number(event.key) - 1;
      const choice = props.definition.choices[index];
      if (choice !== undefined && !controller.pending()) {
        event.preventDefault();
        choose(choice.id);
        document.getElementById(`${props.attemptId}-choice-${index}`)?.focus();
      }
      return;
    }
    handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit);
  }
  return (
    <section class="response-widget" data-phase={controller.phase().kind} onKeyDown={handleKeyDown}>
      <fieldset
        aria-describedby={`${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
      >
        <legend>Choose your response</legend>
        <p class="keyboard-hint">Press 1-{props.definition.choices.length} to select a response.</p>
        <div class="choice-list">
          <For each={props.definition.choices}>
            {(choice, index) => (
              <label class="choice-card" classList={{ selected: selected().includes(choice.id) }}>
                <input
                  id={`${props.attemptId}-choice-${index()}`}
                  type={choiceInputType(props.definition)}
                  name={`response-${props.attemptId}`}
                  value={choice.id}
                  checked={selected().includes(choice.id)}
                  onChange={() => choose(choice.id)}
                />
                <span class="choice-number" aria-hidden="true">
                  {index() + 1}
                </span>
                <span>{textFromBlocks(choice.body)}</span>
              </label>
            )}
          </For>
        </div>
      </fieldset>
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}

/** Standalone multiple-choice entry point retained for the reference run screen. */
export function MultipleChoiceResponse(props: MultipleChoiceResponseProps): JSX.Element {
  return (
    <>
      <style>{RESPONSE_WIDGET_STYLES}</style>
      <MultipleChoiceBody {...props} />
    </>
  );
}

function NumericResponse(props: WidgetBodyProps<NumericDefinition>): JSX.Element {
  const restored = props.initialResponse?.value;
  const initialValue = restored === undefined ? "" : String(restored);
  const [value, setValue] = createSignal(initialValue);
  const controller = createSubmissionController(props, numericResponseFromInput(initialValue));
  const response = (): StudentResponse => numericResponseFromInput(value());
  function update(next: string): void {
    setValue(next);
    void controller.validate(numericResponseFromInput(next));
  }
  function submit(): void {
    void controller.submit(response());
  }
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <label for={`${props.attemptId}-numeric`}>
        Numeric response{props.definition.unit === null ? "" : ` (${props.definition.unit})`}
      </label>
      <input
        id={`${props.attemptId}-numeric`}
        class="response-control"
        type="number"
        inputmode="decimal"
        value={value()}
        aria-describedby={`${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
        onInput={(event) => update(event.currentTarget.value)}
      />
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}

function ShortTextResponse(props: WidgetBodyProps<ShortTextDefinition>): JSX.Element {
  const initialText = props.initialResponse?.text ?? "";
  const [text, setText] = createSignal(initialText);
  const controller = createSubmissionController(props, { kind: "shortText", text: initialText });
  const characterCount = (): number => [...text()].length;
  const response = (): StudentResponse => ({ kind: "shortText", text: text() });
  function update(next: string): void {
    setText(next);
    void controller.validate({ kind: "shortText", text: next });
  }
  function submit(): void {
    void controller.submit(response());
  }
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <label for={`${props.attemptId}-short-text`}>Short written response</label>
      <p class="field-help" id={`${props.attemptId}-short-text-help`}>
        Up to {props.definition.maxLength} characters. {characterCount()} used.
      </p>
      <textarea
        id={`${props.attemptId}-short-text`}
        class="response-control"
        value={text()}
        maxlength={props.definition.maxLength}
        aria-describedby={`${props.attemptId}-short-text-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
        onInput={(event) => update(event.currentTarget.value)}
      />
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}

function moveItem(
  order: ReadonlyArray<ChoiceId>,
  from: number,
  to: number,
): ReadonlyArray<ChoiceId> {
  const next = [...order];
  const item = next[from];
  if (item === undefined || to < 0 || to >= next.length) return order;
  next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}
function choiceById(items: ReadonlyArray<ChoiceOption>, id: ChoiceId): ChoiceOption | undefined {
  return items.find((item) => item.id === id);
}

function OrderingResponse(props: WidgetBodyProps<OrderingDefinition>): JSX.Element {
  const initialOrder =
    props.initialResponse?.order ?? props.definition.items.map((item) => item.id);
  const [order, setOrder] = createSignal<ReadonlyArray<ChoiceId>>(initialOrder);
  const [movementAnnouncement, setMovementAnnouncement] = createSignal("");
  const controller = createSubmissionController(props, {
    kind: "ordering",
    order: [...initialOrder],
  });
  const response = (): StudentResponse => ({ kind: "ordering", order: [...order()] });
  function update(next: ReadonlyArray<ChoiceId>): void {
    setOrder(next);
    void controller.validate({ kind: "ordering", order: [...next] });
  }
  function rowId(id: ChoiceId): string {
    return `${props.attemptId}-order-${id}`;
  }
  function focusMovedItem(id: ChoiceId, preferredDirection: "earlier" | "later"): void {
    queueMicrotask(() => {
      const row = document.getElementById(rowId(id));
      const preferred = row?.querySelector<HTMLButtonElement>(
        `[data-order-direction="${preferredDirection}"]:not(:disabled)`,
      );
      const fallback = row?.querySelector<HTMLButtonElement>("button:not(:disabled)");
      (preferred ?? fallback)?.focus();
    });
  }
  function moveOrderItem(
    id: ChoiceId,
    from: number,
    to: number,
    preferredDirection: "earlier" | "later",
  ): void {
    const next = moveItem(order(), from, to);
    if (next === order()) return;
    update(next);
    const item = choiceById(props.definition.items, id);
    const label = item === undefined ? "Item" : textFromBlocks(item.body);
    setMovementAnnouncement(`${label} moved to position ${to + 1}.`);
    focusMovedItem(id, preferredDirection);
  }
  function handleOrderArrow(event: KeyboardEvent, id: ChoiceId, index: number): void {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    const direction = event.key === "ArrowUp" ? "earlier" : "later";
    const nextIndex = index + (direction === "earlier" ? -1 : 1);
    if (nextIndex < 0 || nextIndex >= order().length) return;
    event.preventDefault();
    moveOrderItem(id, index, nextIndex, direction);
  }
  function submit(): void {
    void controller.submit(response());
  }
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-order-help ${props.attemptId}-order-movement ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
      >
        <legend>Put the items in order</legend>
        <p class="keyboard-hint" id={`${props.attemptId}-order-help`}>
          Tab to a move control, then press Enter or use the Up and Down Arrow keys.
        </p>
        <p
          class="visually-hidden"
          id={`${props.attemptId}-order-movement`}
          role="status"
          aria-live="polite"
        >
          {movementAnnouncement()}
        </p>
        <ol class="ordering-list">
          <For each={order()}>
            {(id, index) => {
              const itemText = (): string => {
                const item = choiceById(props.definition.items, id);
                return item === undefined ? "Unavailable item" : textFromBlocks(item.body);
              };
              return (
                <li class="ordering-row" id={rowId(id)}>
                  <span>{itemText()}</span>
                  <button
                    class="order-action"
                    type="button"
                    data-order-direction="earlier"
                    disabled={index() === 0 || controller.pending()}
                    onClick={() => moveOrderItem(id, index(), index() - 1, "earlier")}
                    onKeyDown={(event) => handleOrderArrow(event, id, index())}
                    aria-label={`Move item ${index() + 1} earlier`}
                  >
                    Up
                  </button>
                  <button
                    class="order-action"
                    type="button"
                    data-order-direction="later"
                    disabled={index() === order().length - 1 || controller.pending()}
                    onClick={() => moveOrderItem(id, index(), index() + 1, "later")}
                    onKeyDown={(event) => handleOrderArrow(event, id, index())}
                    aria-label={`Move item ${index() + 1} later`}
                  >
                    Down
                  </button>
                </li>
              );
            }}
          </For>
        </ol>
      </fieldset>
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}

function acceptedExtensions(definition: FileUploadDefinition): string {
  return definition.acceptedExtensions.length === 0
    ? "Any allowed file type"
    : definition.acceptedExtensions.join(", ");
}

/**
 * This UI deliberately refuses file submissions until the API provides a tenant-scoped upload-slot
 * contract. A learner must never type an object-store key into a browser form.
 */
function FileUploadResponse(props: WidgetBodyProps<FileUploadDefinition>): JSX.Element {
  return (
    <section
      class="response-widget"
      data-phase="unavailable"
      onKeyDown={(event) =>
        handleWidgetKeyDown(
          event,
          props.onEscape,
          () => undefined,
          () => false,
        )
      }
    >
      <h3>File upload is not available for this question yet</h3>
      <p class="field-help">
        This question accepts {acceptedExtensions(props.definition)} up to{" "}
        {props.definition.maxBytes} bytes. Your instructor can use a supported response type while
        secure upload is being enabled.
      </p>
      <p
        id={`${props.attemptId}-format-status`}
        class="format-status error"
        role="status"
        aria-live="polite"
      >
        A secure, tenant-scoped upload slot is required before a file can be submitted.
      </p>
      <Actions disabled onSubmit={() => undefined} onEscape={props.onEscape} />
    </section>
  );
}

function externalToolStatus(phase: ExternalToolPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Open the learning tool when you are ready.";
    case "loading":
      return "Opening the learning tool...";
    case "awaitingReady":
      return "The learning tool is loading. Complete its preparation to enable submission.";
    case "ready":
      return "The learning tool is ready. Submit when you are finished.";
    case "failed":
      return phase.message;
    case "submitting":
      return "Recording your external-tool response. Please wait.";
    case "submitted":
      return "Response recorded. Server feedback will appear when it is released.";
  }
}

function externalToolFrameTitle(attemptId: string): string {
  return `External learning tool for question attempt ${attemptId}`;
}

/**
 * Learner surface for a server-brokered external tool. The iframe receives a
 * protected same-origin broker path after explicit activation. Readiness is a
 * presentation signal only: it cannot supply a response, correctness, score,
 * provider identity, or any value used by grading.
 */
function ExternalToolResponse(props: WidgetBodyProps<ExternalToolDefinition>): JSX.Element {
  const [phase, setPhase] = createSignal<ExternalToolPhase>({ kind: "idle" });
  const [launchUrl, setLaunchUrl] = createSignal<string | null>(null);
  let frame: HTMLIFrameElement | undefined;
  let submitButton: HTMLButtonElement | undefined;
  let launchRequest = 0;

  function marker(): StudentResponse {
    return { kind: "externalTool" };
  }

  function setFrame(element: HTMLIFrameElement): void {
    frame = element;
  }

  function setSubmitButton(element: HTMLButtonElement): void {
    submitButton = element;
  }

  function persistMarker(): void {
    props.onResponseChange?.(marker(), { violations: [] });
  }

  function resetForAttempt(): void {
    launchRequest += 1;
    frame = undefined;
    setLaunchUrl(null);
    setPhase({ kind: "idle" });
  }

  function handleMessage(event: MessageEvent<unknown>): void {
    if (
      event.origin !== window.location.origin ||
      event.source !== frame?.contentWindow ||
      !isExternalToolReadyMessage(event.data, props.attemptId)
    ) {
      return;
    }
    setPhase({ kind: "ready" });
    queueMicrotask(() => submitButton?.focus());
  }

  async function launch(): Promise<void> {
    const getLaunch = props.getExternalToolLaunch;
    if (getLaunch === undefined || phase().kind === "loading") {
      return;
    }
    persistMarker();
    launchRequest += 1;
    const request = launchRequest;
    setPhase({ kind: "loading" });
    try {
      const launch = await getLaunch();
      if (request !== launchRequest) return;
      if (!isSafeExternalToolLaunchPath(launch.launchUrl)) {
        setPhase({ kind: "failed", message: "The learning tool route was not safe to open." });
        return;
      }
      setLaunchUrl(launch.launchUrl);
      setPhase({ kind: "awaitingReady" });
    } catch (error: unknown) {
      if (request !== launchRequest) return;
      const detail = error instanceof Error ? ` ${error.message}` : "";
      setPhase({
        kind: "failed",
        message: `The learning tool is unavailable right now.${detail} Try again when ready.`,
      });
    }
  }

  async function submit(): Promise<void> {
    if (phase().kind !== "ready") return;
    setPhase({ kind: "submitting" });
    try {
      await props.onSubmit(marker());
      setPhase({ kind: "submitted" });
    } catch (error: unknown) {
      const detail = error instanceof Error ? ` ${error.message}` : "";
      setPhase({
        kind: "failed",
        message: `Your response is still saved. Recording it did not finish.${detail} Try again.`,
      });
    }
  }

  function handleKeyDown(event: KeyboardEvent): void {
    handleWidgetKeyDown(
      event,
      props.onEscape,
      () => void submit(),
      () => phase().kind === "ready",
    );
  }

  onMount(() => {
    window.addEventListener("message", handleMessage);
  });
  onCleanup(() => {
    launchRequest += 1;
    window.removeEventListener("message", handleMessage);
  });
  createEffect(
    on(
      () => props.attemptId,
      () => resetForAttempt(),
      { defer: true },
    ),
  );

  const statusId = (): string => `${props.attemptId}-external-tool-status`;
  return (
    <section
      class="response-widget external-tool-widget"
      data-phase={phase().kind}
      onKeyDown={handleKeyDown}
    >
      <h3>External learning tool</h3>
      <p class="field-help">
        Open the tool in this question. When it reports that it is ready, you can record this
        learning activity with the ordinary submission button.
      </p>
      <p id={statusId()} class="format-status" role="status" aria-live="polite">
        {externalToolStatus(phase())}
      </p>
      <Show when={launchUrl()}>
        {(source) => (
          <iframe
            ref={setFrame}
            class="external-tool-frame"
            src={source()}
            title={externalToolFrameTitle(props.attemptId)}
            sandbox="allow-forms allow-same-origin allow-scripts"
            referrerpolicy="same-origin"
            aria-describedby={statusId()}
          />
        )}
      </Show>
      <div class="external-tool-actions">
        <Show when={phase().kind === "idle" || phase().kind === "failed"}>
          <button class="primary-action" type="button" onClick={() => void launch()}>
            {phase().kind === "failed" ? "Retry learning tool" : "Open learning tool"}
          </button>
        </Show>
        <Show when={phase().kind === "loading"}>
          <button class="primary-action" type="button" disabled>
            Opening learning tool...
          </button>
        </Show>
        <button
          ref={setSubmitButton}
          class="primary-action"
          type="button"
          disabled={phase().kind !== "ready"}
          onClick={() => void submit()}
        >
          Submit answer
        </button>
        <button class="quiet-action" type="button" onClick={props.onEscape}>
          Return to assignment <span aria-hidden="true">(Esc)</span>
        </button>
      </div>
    </section>
  );
}

function assertNever(value: never): never {
  throw new Error(`Unhandled response definition: ${JSON.stringify(value)}`);
}

/** Exhaustive dispatch point for every browser-safe ResponseDefinition variant. */
export function ResponseWidget(props: ResponseWidgetProps): JSX.Element {
  let body: JSX.Element;
  switch (props.definition.kind) {
    case "numeric":
      body = (
        <NumericResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "numeric" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "multipleChoice":
      body = (
        <MultipleChoiceBody
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "multipleChoice" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "shortText":
      body = (
        <ShortTextResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "shortText" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "ordering":
      body = (
        <OrderingResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "ordering" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "fileUpload":
      body = (
        <FileUploadResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "fileUpload" ? props.initialResponse : undefined
          }
        />
      );
      break;
    case "externalTool":
      body = (
        <ExternalToolResponse
          {...props}
          definition={props.definition}
          initialResponse={
            props.initialResponse?.kind === "externalTool" ? props.initialResponse : undefined
          }
        />
      );
      break;
    default:
      body = assertNever(props.definition);
  }
  return (
    <>
      <style>{RESPONSE_WIDGET_STYLES}</style>
      {body}
    </>
  );
}
