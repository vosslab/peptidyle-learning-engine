// flat_ordering_editor_model.ts - canonical private ordering edits with stable item IDs.

import type { FlatQuestionItem, FlatQuestionSourceV2 } from "./flat_question_source";

const MINIMUM_ITEMS = 3;
const MAXIMUM_ITEMS = 100;

export type FlatOrderingEditResult = {
  readonly source: FlatQuestionSourceV2;
  readonly changed: boolean;
  readonly error: string | null;
  readonly focusId: string | null;
  readonly status: string | null;
};

function changed(
  source: FlatQuestionSourceV2,
  focusId: string | null,
  status: string | null,
): FlatOrderingEditResult {
  return { source, changed: true, error: null, focusId, status };
}

function refused(source: FlatQuestionSourceV2, error: string): FlatOrderingEditResult {
  return { source, changed: false, error, focusId: null, status: null };
}

function orderingResponse(
  source: FlatQuestionSourceV2,
): Extract<FlatQuestionSourceV2["response"], { readonly kind: "ordering" }> | null {
  return source.response.kind === "ordering" ? source.response : null;
}

/**
 * The format carries both display items and correctOrder for wire compatibility. This editor keeps
 * one private canonical order: items are in the intended sequence and correctOrder is derived.
 */
function withCanonicalOrder(
  source: FlatQuestionSourceV2,
  items: ReadonlyArray<FlatQuestionItem>,
  focusId: string | null,
  status: string,
): FlatOrderingEditResult {
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
  source: FlatQuestionSourceV2,
  itemId: string,
  text: string,
): FlatOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before editing its sequence.");
  const item = response.items.find((candidate) => candidate.id === itemId);
  if (item === undefined) return refused(source, "That ordering item no longer exists.");
  const items = response.items.map((current) =>
    current.id === itemId ? { ...current, text } : current,
  );
  return withCanonicalOrder(source, items, itemId, "Updated ordering item text.");
}

export function addOrderingItem(source: FlatQuestionSourceV2): FlatOrderingEditResult {
  const response = orderingResponse(source);
  if (response === null) return refused(source, "Choose ordering before adding items.");
  if (response.items.length >= MAXIMUM_ITEMS) {
    return refused(source, `A question can have at most ${MAXIMUM_ITEMS} ordering items.`);
  }
  const id = nextOrderingItemId(response.items);
  const item: FlatQuestionItem = { id, text: "New ordering item" };
  return withCanonicalOrder(
    source,
    [...response.items, item],
    id,
    "Added an item at the end of the correct order.",
  );
}

export function removeOrderingItem(
  source: FlatQuestionSourceV2,
  itemId: string,
): FlatOrderingEditResult {
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
  return withCanonicalOrder(
    source,
    items,
    focusItem.id,
    `Removed item ${index + 1} from the private correct order.`,
  );
}

export function moveOrderingItem(
  source: FlatQuestionSourceV2,
  itemId: string,
  direction: "earlier" | "later",
): FlatOrderingEditResult {
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
  return withCanonicalOrder(
    source,
    items,
    itemId,
    `Moved item to position ${position} in the correct order.`,
  );
}

function nextOrderingItemId(items: ReadonlyArray<FlatQuestionItem>): string {
  const ids = new Set(items.map((item) => item.id));
  let index = 1;
  while (ids.has(`item_${index}`)) index += 1;
  return `item_${index}`;
}
