// assignment_editor_model.ts - Questions-page browser state for QID-only assignment content.

import type { FixedQuestionAssignmentEntrySummary } from "../../generated/api/FixedQuestionAssignmentEntrySummary";
import type { QuestionPoolAssignmentEntrySummary } from "../../generated/api/QuestionPoolAssignmentEntrySummary";
import type { AssignmentEntrySummary } from "../../generated/api/AssignmentEntrySummary";
import type { Capability } from "../../generated/api/Capability";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { AssignmentActivityRules } from "../../generated/api/AssignmentActivityRules";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_QUESTION_POOL_ITEMS } from "../../generated/api/MAX_ASSIGNMENT_QUESTION_POOL_ITEMS";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionId } from "../../generated/api/QuestionId";
import type {
  AssignmentCapabilityViolation,
  AssignmentContentInput,
  AssignmentEditorEntryInput,
  AssignmentEditorDetail,
} from "../api/contracts";
import { normalizeQuestionIdSyntax } from "../question_id";

export interface AssignmentQuestionRow {
  readonly questionId: QuestionId;
  readonly title: string;
  readonly backend: QuestionBackend;
}

export type AssignmentEditorFixedQuestionEntry = FixedQuestionAssignmentEntrySummary & {
  readonly kind: "fixedQuestion";
};

/**
 * Browser-owned pool state deliberately names only public Question IDs.
 * The server mints Question Pool Assignment Entry and Question Pool Item identities when it saves this Assignment Content.
 */
export interface AssignmentEditorQuestionPoolAssignmentEntry {
  readonly kind: "questionPool";
  readonly id?: string;
  readonly availability: "available" | "retired";
  readonly scoringRule: "normal" | "fullCredit" | "extraCredit" | "excluded";
  readonly items: ReadonlyArray<AssignmentQuestionRow>;
  readonly selectionCount: number;
  readonly pointsPerItem: string;
  readonly selectionRule: {
    readonly selectedQuestionOrder: "questionPoolOrder" | "randomOrder";
  };
}

export type AssignmentEditorEntry =
  AssignmentEditorFixedQuestionEntry | AssignmentEditorQuestionPoolAssignmentEntry;

export interface AssignmentEditorState {
  readonly id: string;
  readonly courseId: string;
  readonly title: string;
  /** One ordered Assignment Content record, shared by fixed Questions and Question Pools. */
  readonly entries: ReadonlyArray<AssignmentEditorEntry>;
  readonly policies: AssignmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  readonly revision: string;
}

export function assignmentEditorStateFrom(detail: AssignmentEditorDetail): AssignmentEditorState {
  return {
    id: detail.id,
    courseId: detail.courseId,
    title: detail.title,
    entries: detail.entries.map(assignmentEditorEntryFrom),
    policies: detail.policies,
    studentFeedbackReleaseRule: detail.studentFeedbackReleaseRule,
    revision: detail.revision,
  };
}

export function fixedQuestionEntry(
  entry: FixedQuestionAssignmentEntrySummary,
): AssignmentEditorFixedQuestionEntry {
  return { ...entry, kind: "fixedQuestion" };
}

export function questionPoolAssignmentEntry(
  entry: QuestionPoolAssignmentEntrySummary,
): AssignmentEditorQuestionPoolAssignmentEntry {
  return {
    kind: "questionPool",
    id: entry.id,
    availability: entry.availability,
    scoringRule: entry.scoringRule,
    items: entry.items.map((item) => ({
      questionId: item.questionId,
      title: item.title,
      backend: item.backend,
    })),
    selectionCount: entry.selectionCount,
    pointsPerItem: entry.pointsPerItem,
    selectionRule: entry.selectionRule,
  };
}

export function assignmentEditorEntryFrom(entry: AssignmentEntrySummary): AssignmentEditorEntry {
  if (entry.kind === "fixedQuestion") return fixedQuestionEntry(entry);
  return questionPoolAssignmentEntry(entry);
}

export function fixedEntries(
  draft: AssignmentEditorState,
): ReadonlyArray<AssignmentEditorFixedQuestionEntry> {
  return draft.entries.filter(
    (entry): entry is AssignmentEditorFixedQuestionEntry => entry.kind === "fixedQuestion",
  );
}

export function moveAssignmentEntry(
  draft: AssignmentEditorState,
  entryIndex: number,
  direction: -1 | 1,
): AssignmentEditorState {
  const nextIndex = entryIndex + direction;
  if (entryIndex < 0 || nextIndex < 0 || nextIndex >= draft.entries.length) return draft;
  const entries = [...draft.entries];
  const current = entries[entryIndex];
  const adjacent = entries[nextIndex];
  if (current === undefined || adjacent === undefined) return draft;
  entries[entryIndex] = adjacent;
  entries[nextIndex] = current;
  return { ...draft, entries };
}

export function appendFixedEntries(
  draft: AssignmentEditorState,
  rows: ReadonlyArray<AssignmentQuestionRow>,
): AssignmentEditorState {
  const known = new Set(fixedEntries(draft).map((item) => item.questionId));
  const fresh = rows.filter((row) => !known.has(row.questionId));
  if (fresh.length === 0) return draft;
  const entries = [
    ...draft.entries,
    ...fresh.map((row) =>
      fixedQuestionEntry({
        id: `new-${row.questionId}`,
        questionId: row.questionId,
        title: row.title,
        backend: row.backend,
        capabilities: [],
        pointsPossible: "1",
        availability: "available",
        scoringRule: "normal",
      }),
    ),
  ];
  return { ...draft, entries };
}

export function appendQuestionPool(draft: AssignmentEditorState): AssignmentEditorState {
  const questionPool: AssignmentEditorQuestionPoolAssignmentEntry = {
    kind: "questionPool",
    items: [],
    availability: "available",
    scoringRule: "normal",
    selectionCount: 1,
    pointsPerItem: "1",
    selectionRule: { selectedQuestionOrder: "questionPoolOrder" },
  };
  return { ...draft, entries: [...draft.entries, questionPool] };
}

function entryInput(entry: AssignmentEditorEntry): AssignmentEditorEntryInput {
  if (entry.kind === "fixedQuestion") {
    return {
      kind: "fixedQuestion",
      questionId: entry.questionId,
      pointsPossible: entry.pointsPossible,
      availability: entry.availability,
      scoringRule: entry.scoringRule,
    };
  }
  return {
    kind: "questionPool",
    questionIds: entry.items.map((item) => item.questionId),
    availability: entry.availability,
    scoringRule: entry.scoringRule,
    selectionCount: entry.selectionCount,
    pointsPerItem: entry.pointsPerItem,
    selectionRule: entry.selectionRule,
  };
}

/** Questions owns only the visible title and ordered fixed-or-pool Assignment Content. */
export function assignmentContentInput(draft: AssignmentEditorState): AssignmentContentInput {
  return {
    title: draft.title,
    entries: draft.entries.map(entryInput),
  };
}

export function validateQuestionPoolAssignmentEntry(
  entry: AssignmentEditorQuestionPoolAssignmentEntry,
): string | null {
  if (entry.items.length === 0) return "Add at least one Question ID to this Question Pool.";
  if (entry.items.length > MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY)
    return `Keep this Question Pool to ${MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY} Question IDs or fewer.`;
  if (entry.selectionCount < 1) return "Selection count must be at least one.";
  if (entry.selectionCount > entry.items.length)
    return "Selection count cannot exceed the number of Question IDs in this Question Pool.";
  if (new Set(entry.items.map((item) => item.questionId)).size !== entry.items.length)
    return "Each Question ID can appear only once in a Question Pool.";
  return null;
}

/**
 * Mirrors the server-owned pool bounds so the Instructor receives a recovery
 * path before a save request. Rust and PostgreSQL remain authoritative.
 */
export function validateAssignmentEditorState(draft: AssignmentEditorState): string | null {
  if (draft.entries.length > MAX_ASSIGNMENT_ORDERED_ENTRIES)
    return `Keep this assignment to ${MAX_ASSIGNMENT_ORDERED_ENTRIES} ordered entries or fewer.`;
  let totalQuestionPoolItems = 0;
  for (const entry of draft.entries) {
    if (entry.kind !== "questionPool") continue;
    const entryError = validateQuestionPoolAssignmentEntry(entry);
    if (entryError !== null)
      return `Question pool ${draft.entries.indexOf(entry) + 1}: ${entryError}`;
    totalQuestionPoolItems += entry.items.length;
    if (totalQuestionPoolItems > MAX_ASSIGNMENT_QUESTION_POOL_ITEMS)
      return `Keep all Question Pools to ${MAX_ASSIGNMENT_QUESTION_POOL_ITEMS} Question IDs or fewer.`;
  }
  return null;
}

export function parseExactQuestionIds(value: string): ReadonlyArray<string> {
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
export function assignmentQuestionLabel(
  row: AssignmentQuestionRow | FixedQuestionAssignmentEntrySummary,
): string {
  return row.questionId;
}
export function questionBackendLabel(backend: QuestionBackend): string {
  return { ple: "PLE", webwork: "WeBWorK", qti: "QTI", imathas: "iMathAS" }[backend];
}
export function violationMatchesQuestion(
  violation: AssignmentCapabilityViolation,
  questionId: string,
): boolean {
  return violation.questionId === questionId;
}
