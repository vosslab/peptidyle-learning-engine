// ordering.tsx - ordered-response controller with focus-preserving movement.

import { createSignal, For, type JSX } from "solid-js";

import type { ChoiceId } from "../../../generated/api/ChoiceId";
import type { ChoiceOption } from "../../../generated/api/ChoiceOption";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type OrderingDefinition,
  type WidgetBodyProps,
} from "./common";

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

export function OrderingResponse(props: WidgetBodyProps<OrderingDefinition>): JSX.Element {
  const initialOrder =
    props.initialResponse?.order ?? props.definition.items.map((item) => item.id);
  const [order, setOrder] = createSignal<ReadonlyArray<ChoiceId>>(initialOrder);
  let firstMoveControl!: HTMLButtonElement;
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
    setMovementAnnouncement(
      `${item === undefined ? "Item" : textFromBlocks(item.body)} moved to position ${to + 1}.`,
    );
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
  function reset(): void {
    const next = [...initialOrder];
    setOrder(next);
    setMovementAnnouncement("Order restored.");
    void controller.reset({ kind: "ordering", order: next });
    queueMicrotask(() => firstMoveControl.focus());
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
        disabled={controller.locked()}
      >
        <legend>Put the items in order</legend>
        <p class="keyboard-hint" id={`${props.attemptId}-order-help`}>
          Tab to a move control and press Space to activate it. Shortcut: use the Up or Down Arrow
          key on the focused move control.
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
                    disabled={index() === 0 || controller.locked()}
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
                    ref={
                      index() === 0
                        ? (element): void => {
                            firstMoveControl = element;
                          }
                        : undefined
                    }
                    disabled={index() === order().length - 1 || controller.locked()}
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
        disabled={!controller.canSubmit() || controller.locked()}
        resetDisabled={controller.locked()}
        onSubmit={submit}
        onReset={reset}
        resetLabel="Reset order"
        onEscape={props.onEscape}
      />
    </section>
  );
}
