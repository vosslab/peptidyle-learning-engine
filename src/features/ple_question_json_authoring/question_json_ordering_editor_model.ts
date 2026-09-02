// question_json_ordering_editor_model.ts - private ordering edits with stable item IDs.

import { createPleQuestionJsonOrderingItem } from "./question_json_source";
import type { PleQuestionJsonDocument, PleQuestionJsonOrderingItem } from "./question_json_source";

const MINIMUM_ITEMS = 3;
const MAXIMUM_ITEMS = 100;

export type PleQuestionJsonOrderingEditResult = {
  readonly source: PleQuestionJsonDocument;
  readonly changed: boolean;
  readonly error: string | null;
  readonly focusId: string | null;
  readonly status: string | null;
};

function changed(
  source: PleQuestionJsonDocument,
  focusId: string | null,
  status: string | null,
): PleQuestionJsonOrderingEditResult {
  return { source, changed: true, error: null, focusId, status };
}

function refused(
  source: PleQuestionJsonDocument,
  error: string,
): PleQuestionJsonOrderingEditResult {
  return { source, changed: false, error, focusId: null, status: null };
}

function orderingResponse(
  source: PleQuestionJsonDocument,
): Extract<PleQuestionJsonDocument["response"], { readonly kind: "ordering" }> | null {
  return source.response.kind === "ordering" ? source.response : null;
}

/**
 * The format carries both display items and correctOrder for wire compatibility. Items are the
 * source of truth and correctOrder is derived.
 */
function withDerivedCorrectOrder(
  source: PleQuestionJsonDocument,
  items: ReadonlyArray<PleQuestionJsonOrderingItem>,
  focusId: string | null,
  status: string,
): PleQuestionJsonOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before editing its sequence.");
  const correctOrder = items.map((item) => item.id);
  return changed(
    { ...source, response: { ...response, items: [...items], correctOrder } },
    focusId,
    status,
  );
}

export function setOrderingItemText(
  source: PleQuestionJsonDocument,
  itemId: string,
  text: string,
): PleQuestionJsonOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before editing its sequence.");
  const item = response.items.find((candidate) => candidate.id === itemId);
  if (item === undefined) return refused(source, "That ordering item no longer exists.");
  const items = response.items.map((current) =>
    current.id === itemId ? { ...current, text } : current,
  );
  return withDerivedCorrectOrder(source, items, itemId, "Updated ordering item text.");
}

export function addOrderingItem(
  source: PleQuestionJsonDocument,
): PleQuestionJsonOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before adding items.");
  if (response.items.length >= MAXIMUM_ITEMS) {
    return refused(source, `A question can have at most ${MAXIMUM_ITEMS} ordering items.`);
  }
  const id = nextOrderingItemId(response.items);
  const item = createPleQuestionJsonOrderingItem(id, "New ordering item");
  return withDerivedCorrectOrder(
    source,
    [...response.items, item],
    id,
    "Added an item at the end of the correct order.",
  );
}

export function removeOrderingItem(
  source: PleQuestionJsonDocument,
  itemId: string,
): PleQuestionJsonOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before editing its sequence.");
  if (response.items.length <= MINIMUM_ITEMS) {
    return refused(source, `An ordering question needs at least ${MINIMUM_ITEMS} items.`);
  }
  const index = response.items.findIndex((item) => item.id === itemId);
  if (index < 0) return refused(source, "That ordering item no longer exists.");
  const items = response.items.filter((item) => item.id !== itemId);
  const focusItem = items[Math.min(index, items.length - 1)];
  if (focusItem === undefined)
    return refused(source, "Choose a remaining item before removing this one.");
  return withDerivedCorrectOrder(
    source,
    items,
    focusItem.id,
    `Removed item ${index + 1} from the private correct order.`,
  );
}

export function moveOrderingItem(
  source: PleQuestionJsonDocument,
  itemId: string,
  direction: "earlier" | "later",
): PleQuestionJsonOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before editing its sequence.");
  const index = response.items.findIndex((item) => item.id === itemId);
  if (index < 0) return refused(source, "That ordering item no longer exists.");
  const nextIndex = direction === "earlier" ? index - 1 : index + 1;
  if (nextIndex < 0 || nextIndex >= response.items.length) {
    return refused(source, `This item is already ${direction === "earlier" ? "first" : "last"}.`);
  }
  const items = [...response.items];
  const item = items[index];
  const neighbor = items[nextIndex];
  if (item === undefined || neighbor === undefined)
    return refused(source, "That ordering item no longer exists.");
  items[index] = neighbor;
  items[nextIndex] = item;
  const position = nextIndex + 1;
  return withDerivedCorrectOrder(
    source,
    items,
    itemId,
    `Moved item to position ${position} in the correct order.`,
  );
}

function nextOrderingItemId(items: ReadonlyArray<PleQuestionJsonOrderingItem>): string {
  const ids = new Set(items.map((item) => item.id));
  let index = 1;
  while (ids.has(`item_${index}`)) index += 1;
  return `item_${index}`;
}
