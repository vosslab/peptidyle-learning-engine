// assignment_editor_model.ts - Questions-page browser state for QID-only assignment content.

import type { AssignmentItemSummary } from "../../generated/api/AssignmentItemSummary";
import type { AssignmentSelectionGroupSummary } from "../../generated/api/AssignmentSelectionGroupSummary";
import type { Capability } from "../../generated/api/Capability";
import type { StudentDisclosurePolicy } from "../../generated/api/StudentDisclosurePolicy";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import { MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP } from "../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES } from "../../generated/api/MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionId } from "../../generated/api/QuestionId";
import type {
  AssignmentCapabilityViolation,
  AssignmentContentInput,
  AssignmentEditorEntryInput,
  AssignmentEditorDetail,
} from "../api/contracts";
import { normalizeQuestionIdSyntax } from "../question_id";

export interface AssignmentCatalogRow {
  readonly questionId: QuestionId;
  readonly title: string;
  readonly backend: QuestionBackend;
}

export type AssignmentEditorFixedEntry = AssignmentItemSummary & { readonly kind: "fixed" };

/**
 * Browser-owned pool state deliberately names only public Question IDs.
 * The server mints and preserves group/candidate identities when it saves this definition.
 */
export interface AssignmentEditorSelectionGroupEntry {
  readonly kind: "selectionGroup";
  readonly id?: string;
  readonly position: number;
  readonly candidates: ReadonlyArray<AssignmentCatalogRow>;
  readonly drawCount: number;
  readonly pointsPerItem: string;
  readonly ordering: "candidateOrder" | "randomized";
  /** The only currently executable server algorithm. It is intentionally not editable. */
  readonly algorithmVersion: 1;
}

export type AssignmentEditorEntry =
  AssignmentEditorFixedEntry | AssignmentEditorSelectionGroupEntry;

export interface AssignmentEditorDraft {
  readonly id: string;
  readonly courseId: string;
  readonly title: string;
  /** One ordered definition, shared by fixed questions and selection groups. */
  readonly entries: ReadonlyArray<AssignmentEditorEntry>;
  readonly policies: RunPolicies;
  readonly disclosurePolicy: StudentDisclosurePolicy;
  readonly revision: string;
}

export function assignmentEditorDraftFrom(detail: AssignmentEditorDetail): AssignmentEditorDraft {
  return {
    id: detail.id,
    courseId: detail.courseId,
    title: detail.title,
    entries: orderedAssignmentEntries(detail.items, detail.selectionGroups),
    policies: detail.policies,
    disclosurePolicy: detail.disclosurePolicy,
    revision: detail.revision,
  };
}

export function fixedEntry(item: AssignmentItemSummary): AssignmentEditorFixedEntry {
  return { ...item, kind: "fixed" };
}

export function selectionGroupEntry(
  group: AssignmentSelectionGroupSummary,
): AssignmentEditorSelectionGroupEntry {
  if (group.algorithmVersion !== 1) {
    throw new Error("This assignment uses an unsupported pool draw algorithm.");
  }
  return {
    kind: "selectionGroup",
    id: group.id,
    position: group.position,
    candidates: group.candidates.map((candidate) => ({
      questionId: candidate.questionId,
      title: candidate.title,
      backend: candidate.backend,
    })),
    drawCount: group.drawCount,
    pointsPerItem: group.pointsPerItem,
    ordering: group.ordering,
    algorithmVersion: 1,
  };
}

export function orderedAssignmentEntries(
  items: ReadonlyArray<AssignmentItemSummary>,
  groups: ReadonlyArray<AssignmentSelectionGroupSummary>,
): ReadonlyArray<AssignmentEditorEntry> {
  const entries = [...items.map(fixedEntry), ...groups.map(selectionGroupEntry)];
  return entries.sort((left, right) => left.position - right.position);
}

export function fixedEntries(
  draft: AssignmentEditorDraft,
): ReadonlyArray<AssignmentEditorFixedEntry> {
  return draft.entries.filter(
    (entry): entry is AssignmentEditorFixedEntry => entry.kind === "fixed",
  );
}

export function moveAssignmentEntry(
  draft: AssignmentEditorDraft,
  entryIndex: number,
  direction: -1 | 1,
): AssignmentEditorDraft {
  const nextIndex = entryIndex + direction;
  if (entryIndex < 0 || nextIndex < 0 || nextIndex >= draft.entries.length) return draft;
  const entries = [...draft.entries];
  const current = entries[entryIndex];
  const adjacent = entries[nextIndex];
  if (current === undefined || adjacent === undefined) return draft;
  entries[entryIndex] = { ...adjacent, position: entryIndex };
  entries[nextIndex] = { ...current, position: nextIndex };
  return { ...draft, entries };
}

export function appendFixedEntries(
  draft: AssignmentEditorDraft,
  rows: ReadonlyArray<AssignmentCatalogRow>,
): AssignmentEditorDraft {
  const known = new Set(fixedEntries(draft).map((item) => item.questionId));
  const fresh = rows.filter((row) => !known.has(row.questionId));
  if (fresh.length === 0) return draft;
  const entries = [
    ...draft.entries,
    ...fresh.map((row, index) =>
      fixedEntry({
        id: `new-${row.questionId}`,
        questionId: row.questionId,
        title: row.title,
        backend: row.backend,
        capabilities: [],
        position: draft.entries.length + index,
        pointsPossible: "1",
        deliveryState: "active",
        scoringMode: "normal",
      }),
    ),
  ];
  return { ...draft, entries };
}

export function appendSelectionGroup(draft: AssignmentEditorDraft): AssignmentEditorDraft {
  const group: AssignmentEditorSelectionGroupEntry = {
    kind: "selectionGroup",
    position: draft.entries.length,
    candidates: [],
    drawCount: 1,
    pointsPerItem: "1",
    ordering: "candidateOrder",
    algorithmVersion: 1,
  };
  return { ...draft, entries: [...draft.entries, group] };
}

function entryInput(entry: AssignmentEditorEntry, position: number): AssignmentEditorEntryInput {
  if (entry.kind === "fixed") {
    return {
      kind: "fixed",
      questionId: entry.questionId,
      pointsPossible: entry.pointsPossible,
      deliveryState: entry.deliveryState,
      scoringMode: entry.scoringMode,
      position,
    };
  }
  return {
    kind: "selectionGroup",
    candidateQuestionIds: entry.candidates.map((candidate) => candidate.questionId),
    drawCount: entry.drawCount,
    pointsPerItem: entry.pointsPerItem,
    ordering: entry.ordering,
    position,
  };
}

/** Questions owns only the visible title and ordered fixed-or-pool definition. */
export function assignmentContentInput(draft: AssignmentEditorDraft): AssignmentContentInput {
  return {
    title: draft.title,
    entries: draft.entries.map(entryInput),
  };
}

export function validateSelectionGroupEntry(
  entry: AssignmentEditorSelectionGroupEntry,
): string | null {
  if (entry.candidates.length === 0) return "Add at least one candidate Question ID.";
  if (entry.candidates.length > MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP)
    return `Keep this pool to ${MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP} candidate Question IDs or fewer.`;
  if (entry.drawCount < 1) return "Draw count must be at least one.";
  if (entry.drawCount > entry.candidates.length)
    return "Draw count cannot exceed the number of candidate Question IDs.";
  if (
    new Set(entry.candidates.map((candidate) => candidate.questionId)).size !==
    entry.candidates.length
  )
    return "Each candidate Question ID can appear only once in a pool.";
  return null;
}

/**
 * Mirrors the server-owned pool bounds so the Instructor receives a recovery
 * path before a save request. Rust and PostgreSQL remain authoritative.
 */
export function validateAssignmentEditorDraft(draft: AssignmentEditorDraft): string | null {
  if (draft.entries.length > MAX_ASSIGNMENT_ORDERED_ENTRIES)
    return `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer.`;
  let totalCandidates = 0;
  for (const entry of draft.entries) {
    if (entry.kind !== "selectionGroup") continue;
    const entryError = validateSelectionGroupEntry(entry);
    if (entryError !== null) return `Question pool ${entry.position + 1}: ${entryError}`;
    totalCandidates += entry.candidates.length;
    if (totalCandidates > MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES)
      return `Keep all pools to ${MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES} candidate Question IDs or fewer.`;
  }
  return null;
}

export function parseExactProblemDisplayReferences(value: string): ReadonlyArray<string> {
  const values = value
    .split(/[\n,]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
  if (values.length === 0) throw new Error("Paste at least one Question ID, such as 7K3-M9QP.");
  const normalized = values.map((entry) => normalizeQuestionIdSyntax(entry));
  if (normalized.some((entry) => entry === null))
    throw new Error("Use canonical Question IDs such as 7K3-M9QP.");
  const ids = normalized as string[];
  if (new Set(ids).size !== ids.length)
    throw new Error("Each Question ID can appear once in this operation.");
  return ids;
}

export function capabilityLabel(capability: Capability): string {
  return capability.replace(/([A-Z])/gu, " $1").toLowerCase();
}
export function assignmentProblemLabel(row: AssignmentCatalogRow | AssignmentItemSummary): string {
  return row.questionId;
}
export function questionBackendLabel(backend: QuestionBackend): string {
  return { native: "PLE native", webwork: "WeBWorK", qti: "QTI", h5p: "H5P", imathas: "iMathAS" }[
    backend
  ];
}
export function violationMatchesQuestion(
  violation: AssignmentCapabilityViolation,
  questionId: string,
): boolean {
  return violation.questionId === questionId;
}
