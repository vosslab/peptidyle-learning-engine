// Immutable browser drafts for the one reusable Blueprint Course model.

import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { CreateBlueprintCourseContentInput } from "../../../generated/api/CreateBlueprintCourseContentInput";
import type { BlueprintAssignmentDefaults } from "../../../generated/api/BlueprintAssignmentDefaults";
import type { BlueprintAssignmentContentInput } from "../../../generated/api/BlueprintAssignmentContentInput";
import type { BlueprintAssignmentContentView } from "../../../generated/api/BlueprintAssignmentContentView";
import type { BlueprintAssignmentEntryInput } from "../../../generated/api/BlueprintAssignmentEntryInput";
import type { BlueprintAssignmentEntryView } from "../../../generated/api/BlueprintAssignmentEntryView";
import type { RelativeAssignmentSchedule } from "../../../generated/api/RelativeAssignmentSchedule";
import type { RelativeAssignmentScheduleMoment } from "../../../generated/api/RelativeAssignmentScheduleMoment";
import type { QuestionPickerSelection } from "../question_picker";

export const MAX_REUSABLE_ENTRIES = 1024;
export const MAX_QUESTION_POOL_ITEMS = 1024;
export const MAX_REUSABLE_TITLE_LENGTH = 200;

export type ReusableScheduleField = "available_at" | "due_at" | "closes_at";
export type ReusableEntryDirection = -1 | 1;

export interface BlueprintCourseValidation {
  readonly valid: boolean;
  readonly message: string | null;
}

export interface BlueprintCourseContinuationPresentation {
  readonly visible: boolean;
  readonly action: string | null;
}

/** Appends a cursor page without duplicating an already visible Blueprint Course. */
export function appendBlueprintCoursePage<Record extends { readonly reference: string }>(
  current: ReadonlyArray<Record>,
  incoming: ReadonlyArray<Record>,
): ReadonlyArray<Record> {
  const known = new Set(current.map((record) => record.reference));
  return [...current, ...incoming.filter((record) => !known.has(record.reference))];
}

/** Gives every cursor continuation a precise Blueprint Course action. */
export function blueprintCourseContinuationPresentation(
  hasMore: boolean,
  retry: boolean,
): BlueprintCourseContinuationPresentation {
  if (!hasMore) return { visible: false, action: null };
  return {
    visible: true,
    action: retry ? "Retry loading Blueprint Courses" : "Load more Blueprint Courses",
  };
}

function defaultDefaults(): BlueprintAssignmentDefaults {
  return {
    assignment_attempt_time_limit_seconds: null,
    attempt_limit: null,
    late_work_rule: "mark_late",
    assignment_deadline_rule: "auto_submit",
    activity_rules: {
      assignmentCompletionRule: { kind: "answerAll" },
      assignmentAttemptGradeRule: "highest",
      assignmentAttemptContinuationRule: { kind: "unlimited" },
      questionPoolReuseRule: "reuseSelection",
      questionVariationRule: "newVariation",
      assignmentAttemptResumeRule: "resumable",
      assignmentQuestionDisplayRule: "allQuestions",
      assignmentNavigationRule: "freeNavigation",
      assignmentQuestionOrderRule: "authoredOrder",
    },
    student_feedback_release_rule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      question_feedback: "after_submit",
      question_answer: "after_close",
      question_answer_explanation: "after_close",
      class_statistics: "never",
    },
  };
}

function emptySchedule(): RelativeAssignmentSchedule {
  return { available_at: null, due_at: null, closes_at: null };
}

/** Builds an editable assignment content with visible teaching defaults. */
export function emptyReusableContent(
  title = "Untitled Blueprint Assignment",
): BlueprintAssignmentContentInput {
  return {
    title,
    instructions: "",
    entries: [],
    defaults: defaultDefaults(),
    schedule: emptySchedule(),
  };
}

/** Builds one complete local Blueprint Course draft with one labelled module. */
export function emptyBlueprintCourseContent(): CreateBlueprintCourseContentInput {
  return {
    title: "Untitled Blueprint Course",
    modules: [{ label: "Module 1", assignments: [emptyReusableContent()] }],
  };
}

function uniqueQuestionIds(selection: QuestionPickerSelection): ReadonlyArray<string> {
  return selection.questionIds.filter(
    (questionId, index, all) => all.indexOf(questionId) === index,
  );
}

function fixedEntry(questionId: string): BlueprintAssignmentEntryInput {
  return {
    kind: "fixed",
    question_id: questionId,
    points_possible: "1",
    scoring_rule: "normal",
    question_attempt_limit: { maxAttempts: null },
    question_attempt_time_limit: { kind: "unlimited" },
  };
}

function poolEntry(questionPoolItems: ReadonlyArray<string>): BlueprintAssignmentEntryInput {
  return {
    kind: "pool",
    items: [...questionPoolItems],
    selection_count: 1,
    points_per_item: "1",
    scoring_rule: "normal",
    selection_rule: { selectedQuestionOrder: "questionPoolOrder" },
    question_attempt_limit: { maxAttempts: null },
    question_attempt_time_limit: { kind: "unlimited" },
  };
}

/** Appends chosen Questions as fixed entries while retaining picker order. */
export function appendPickedFixedEntries(
  content: BlueprintAssignmentContentInput,
  selection: QuestionPickerSelection,
): BlueprintAssignmentContentInput {
  return {
    ...content,
    entries: [...content.entries, ...uniqueQuestionIds(selection).map(fixedEntry)],
  };
}

/** Appends one Question Pool with Question Pool Item order selected by the Instructor. */
export function appendPickedPool(
  content: BlueprintAssignmentContentInput,
  selection: QuestionPickerSelection,
): BlueprintAssignmentContentInput {
  const questionPoolItems = uniqueQuestionIds(selection);
  return questionPoolItems.length === 0
    ? content
    : { ...content, entries: [...content.entries, poolEntry(questionPoolItems)] };
}

export function moveReusableEntry(
  content: BlueprintAssignmentContentInput,
  index: number,
  direction: ReusableEntryDirection,
): BlueprintAssignmentContentInput {
  const destination = index + direction;
  if (index < 0 || destination < 0 || destination >= content.entries.length) return content;
  const entries = [...content.entries];
  const current = entries[index];
  const adjacent = entries[destination];
  if (current === undefined || adjacent === undefined) return content;
  entries[index] = adjacent;
  entries[destination] = current;
  return { ...content, entries };
}

export function removeReusableEntry(
  content: BlueprintAssignmentContentInput,
  index: number,
): BlueprintAssignmentContentInput {
  if (index < 0 || index >= content.entries.length) return content;
  return {
    ...content,
    entries: content.entries.filter((_, entryIndex) => entryIndex !== index),
  };
}

export function updateReusablePoolSelectionCount(
  content: BlueprintAssignmentContentInput,
  index: number,
  selectionCount: number,
): BlueprintAssignmentContentInput {
  const entry = content.entries[index];
  if (entry === undefined || entry.kind !== "pool") return content;
  const entries = [...content.entries];
  entries[index] = { ...entry, selection_count: selectionCount };
  return { ...content, entries };
}

export function updateReusableSchedule(
  content: BlueprintAssignmentContentInput,
  field: ReusableScheduleField,
  moment: RelativeAssignmentScheduleMoment | null,
): BlueprintAssignmentContentInput {
  return { ...content, schedule: { ...content.schedule, [field]: moment } };
}

export function updateReusableDefaults(
  content: BlueprintAssignmentContentInput,
  defaults: BlueprintAssignmentDefaults,
): BlueprintAssignmentContentInput {
  return { ...content, defaults };
}

export function updateReusableText(
  content: BlueprintAssignmentContentInput,
  change: Partial<Pick<BlueprintAssignmentContentInput, "title" | "instructions">>,
): BlueprintAssignmentContentInput {
  return { ...content, ...change };
}

function momentValue(moment: RelativeAssignmentScheduleMoment | null): number | null {
  if (moment === null) return null;
  if (!/^([01]\d|2[0-3]):[0-5]\d:[0-5]\d\.\d{3}$/.test(moment.local_time)) return null;
  const [hours, minutes, seconds, milliseconds] = moment.local_time.split(/[:.]/).map(Number);
  if (
    hours === undefined ||
    minutes === undefined ||
    seconds === undefined ||
    milliseconds === undefined
  ) {
    return null;
  }
  return (
    moment.day_offset * 86_400_000 +
    hours * 3_600_000 +
    minutes * 60_000 +
    seconds * 1_000 +
    milliseconds
  );
}

function validateSchedule(schedule: RelativeAssignmentSchedule): BlueprintCourseValidation {
  const available = momentValue(schedule.available_at);
  const due = momentValue(schedule.due_at);
  const closes = momentValue(schedule.closes_at);
  if (
    [schedule.available_at, schedule.due_at, schedule.closes_at].some(
      (moment) => moment !== null && momentValue(moment) === null,
    )
  ) {
    return { valid: false, message: "Use local times in HH:MM:SS.sss format." };
  }
  if (
    (available !== null && due !== null && available > due) ||
    (due !== null && closes !== null && due > closes) ||
    (available !== null && closes !== null && available > closes)
  ) {
    return {
      valid: false,
      message: "Available, due, and close moments must remain in calendar order.",
    };
  }
  return { valid: true, message: null };
}

/** Guides local drafting before the server performs authoritative validation. */
export function validateReusableContent(
  content: BlueprintAssignmentContentInput,
): BlueprintCourseValidation {
  if (content.title.trim().length === 0 || content.title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return {
      valid: false,
      message: "Give this Blueprint Assignment a title of up to 200 characters.",
    };
  }
  if (content.entries.length === 0 || content.entries.length > MAX_REUSABLE_ENTRIES) {
    return {
      valid: false,
      message: "Add at least one fixed Question or Question Pool before saving.",
    };
  }
  for (const entry of content.entries) {
    if (entry.kind !== "pool") continue;
    if (entry.items.length === 0 || entry.items.length > MAX_QUESTION_POOL_ITEMS) {
      return {
        valid: false,
        message: "Each Question Pool needs from 1 through 1024 Question Pool Items.",
      };
    }
    if (new Set(entry.items).size !== entry.items.length) {
      return { valid: false, message: "Each Question Pool Item must appear only once." };
    }
    if (
      !Number.isSafeInteger(entry.selection_count) ||
      entry.selection_count < 1 ||
      entry.selection_count > entry.items.length
    ) {
      return {
        valid: false,
        message: "Choose a whole selection count between 1 and this Question Pool's Item count.",
      };
    }
  }
  return validateSchedule(content.schedule);
}

/** Validates the complete local Blueprint Course tree before its create request. */
export function validateBlueprintCourseContent(
  content: CreateBlueprintCourseContentInput,
): BlueprintCourseValidation {
  if (content.title.trim().length === 0 || content.title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return { valid: false, message: "Give this Blueprint Course a title of up to 200 characters." };
  }
  if (content.modules.length === 0 || content.modules.length > MAX_REUSABLE_ENTRIES) {
    return {
      valid: false,
      message: "Add at least one labelled module before creating the Blueprint Course.",
    };
  }
  for (const module of content.modules) {
    if (module.label.trim().length === 0 || module.label.length > MAX_REUSABLE_TITLE_LENGTH) {
      return {
        valid: false,
        message: "Give each Blueprint Course module a label of up to 200 characters.",
      };
    }
    if (module.assignments.length === 0 || module.assignments.length > MAX_REUSABLE_ENTRIES) {
      return { valid: false, message: "Each module needs at least one Blueprint Assignment." };
    }
    for (const assignment of module.assignments) {
      const validation = validateReusableContent(assignment);
      if (!validation.valid) return validation;
    }
  }
  return { valid: true, message: null };
}

function entryInputFromView(entry: BlueprintAssignmentEntryView): BlueprintAssignmentEntryInput {
  if (entry.kind === "pool") {
    return {
      kind: "pool",
      items: entry.items.map((item) => item.question_library.summary.questionId),
      selection_count: entry.selection_count,
      points_per_item: entry.points_per_item,
      scoring_rule: entry.scoring_rule,
      selection_rule: entry.selection_rule,
      question_attempt_limit: entry.question_attempt_limit,
      question_attempt_time_limit: entry.question_attempt_time_limit,
    };
  }
  return {
    kind: "fixed",
    question_id: entry.question.question_library.summary.questionId,
    points_possible: entry.points_possible,
    scoring_rule: entry.scoring_rule,
    question_attempt_limit: entry.question_attempt_limit,
    question_attempt_time_limit: entry.question_attempt_time_limit,
  };
}

export function reusableContentInputFromView(
  content: BlueprintAssignmentContentView,
): BlueprintAssignmentContentInput {
  return {
    title: content.title,
    instructions: content.instructions,
    entries: content.entries.map(entryInputFromView),
    defaults: content.defaults,
    schedule: content.schedule,
  };
}

/** Converts a BlueprintCourseView to editable complete Blueprint Revision Content. */
export function replacementContentFromBlueprintCourse(
  view: BlueprintCourseView,
): import("../../../generated/api/ReplaceBlueprintCourseContentInput").ReplaceBlueprintCourseContentInput {
  return {
    title: view.title,
    modules: view.modules.map((module) => ({
      handle: { kind: "retained", module_id: module.module_id },
      label: module.label,
      assignments: module.assignments.map((assignment) => ({
        handle: { kind: "retained", assignment_id: assignment.assignment_id },
        content: reusableContentInputFromView(assignment.content),
      })),
    })),
  };
}
