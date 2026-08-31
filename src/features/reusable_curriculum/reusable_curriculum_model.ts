// Immutable browser drafts for the one reusable Blueprint Course model.

import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { CreateBlueprintCourseDefinitionInput } from "../../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { ReusableAssignmentDefaults } from "../../../generated/api/ReusableAssignmentDefaults";
import type { ReusableAssignmentDefinitionInput } from "../../../generated/api/ReusableAssignmentDefinitionInput";
import type { ReusableAssignmentDefinitionView } from "../../../generated/api/ReusableAssignmentDefinitionView";
import type { ReusableAssignmentEntryInput } from "../../../generated/api/ReusableAssignmentEntryInput";
import type { ReusableAssignmentEntryView } from "../../../generated/api/ReusableAssignmentEntryView";
import type { RelativeAssignmentSchedule } from "../../../generated/api/RelativeAssignmentSchedule";
import type { RelativeScheduleMoment } from "../../../generated/api/RelativeScheduleMoment";
import type { ProblemPickerSelection } from "../problem_picker";

export const MAX_REUSABLE_ENTRIES = 1024;
export const MAX_POOL_CANDIDATES = 1024;
export const MAX_REUSABLE_TITLE_LENGTH = 200;

export type ReusableScheduleField = "available_at" | "due_at" | "closes_at";
export type ReusableEntryDirection = -1 | 1;

export interface CurriculumValidation {
  readonly valid: boolean;
  readonly message: string | null;
}

export interface CurriculumContinuationPresentation {
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
): CurriculumContinuationPresentation {
  if (!hasMore) return { visible: false, action: null };
  return {
    visible: true,
    action: retry ? "Retry loading Blueprint Courses" : "Load more Blueprint Courses",
  };
}

function defaultDefaults(): ReusableAssignmentDefaults {
  return {
    time_limit_seconds: null,
    attempt_limit: null,
    late_submission: "markLate",
    deadline_behavior: "autoSubmit",
    activity_rules: {
      completion: { kind: "answerAll" },
      grade: "highest",
      continuedPractice: { kind: "unlimited" },
      variation: "newSeeds",
    },
    student_disclosure: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      feedback_text: "after_submit",
      solution: "after_close",
      class_statistics: "never",
    },
  };
}

function emptySchedule(): RelativeAssignmentSchedule {
  return { available_at: null, due_at: null, closes_at: null };
}

/** Builds an editable assignment definition with visible teaching defaults. */
export function emptyReusableDefinition(
  title = "Untitled reusable assignment",
): ReusableAssignmentDefinitionInput {
  return {
    title,
    instructions: "",
    entries: [],
    defaults: defaultDefaults(),
    schedule: emptySchedule(),
  };
}

/** Builds one complete local Blueprint Course draft with one labelled module. */
export function emptyBlueprintCourseDefinition(): CreateBlueprintCourseDefinitionInput {
  return {
    title: "Untitled Blueprint Course",
    modules: [{ label: "Module 1", definitions: [emptyReusableDefinition()] }],
  };
}

function uniqueQuestionIds(selection: ProblemPickerSelection): ReadonlyArray<string> {
  return selection.questionIds.filter(
    (questionId, index, all) => all.indexOf(questionId) === index,
  );
}

function fixedEntry(questionId: string): ReusableAssignmentEntryInput {
  return { kind: "fixed", question_id: questionId, points_possible: "1", scoring_mode: "normal" };
}

function poolEntry(questionIds: ReadonlyArray<string>): ReusableAssignmentEntryInput {
  return {
    kind: "pool",
    candidates: [...questionIds],
    draw_count: 1,
    points_per_item: "1",
    ordering: "candidateOrder",
    algorithm: "v1",
  };
}

/** Appends chosen Questions as fixed entries while retaining picker order. */
export function appendPickedFixedEntries(
  definition: ReusableAssignmentDefinitionInput,
  selection: ProblemPickerSelection,
): ReusableAssignmentDefinitionInput {
  return {
    ...definition,
    entries: [...definition.entries, ...uniqueQuestionIds(selection).map(fixedEntry)],
  };
}

/** Appends one Question Pool with candidate order selected by the Instructor. */
export function appendPickedPool(
  definition: ReusableAssignmentDefinitionInput,
  selection: ProblemPickerSelection,
): ReusableAssignmentDefinitionInput {
  const candidates = uniqueQuestionIds(selection);
  return candidates.length === 0
    ? definition
    : { ...definition, entries: [...definition.entries, poolEntry(candidates)] };
}

export function moveReusableEntry(
  definition: ReusableAssignmentDefinitionInput,
  index: number,
  direction: ReusableEntryDirection,
): ReusableAssignmentDefinitionInput {
  const destination = index + direction;
  if (index < 0 || destination < 0 || destination >= definition.entries.length) return definition;
  const entries = [...definition.entries];
  const current = entries[index];
  const adjacent = entries[destination];
  if (current === undefined || adjacent === undefined) return definition;
  entries[index] = adjacent;
  entries[destination] = current;
  return { ...definition, entries };
}

export function removeReusableEntry(
  definition: ReusableAssignmentDefinitionInput,
  index: number,
): ReusableAssignmentDefinitionInput {
  if (index < 0 || index >= definition.entries.length) return definition;
  return {
    ...definition,
    entries: definition.entries.filter((_, entryIndex) => entryIndex !== index),
  };
}

export function updateReusablePoolDrawCount(
  definition: ReusableAssignmentDefinitionInput,
  index: number,
  drawCount: number,
): ReusableAssignmentDefinitionInput {
  const entry = definition.entries[index];
  if (entry === undefined || entry.kind !== "pool") return definition;
  const entries = [...definition.entries];
  entries[index] = { ...entry, draw_count: drawCount };
  return { ...definition, entries };
}

export function updateReusableSchedule(
  definition: ReusableAssignmentDefinitionInput,
  field: ReusableScheduleField,
  moment: RelativeScheduleMoment | null,
): ReusableAssignmentDefinitionInput {
  return { ...definition, schedule: { ...definition.schedule, [field]: moment } };
}

export function updateReusableDefaults(
  definition: ReusableAssignmentDefinitionInput,
  defaults: ReusableAssignmentDefaults,
): ReusableAssignmentDefinitionInput {
  return { ...definition, defaults };
}

export function updateReusableText(
  definition: ReusableAssignmentDefinitionInput,
  change: Partial<Pick<ReusableAssignmentDefinitionInput, "title" | "instructions">>,
): ReusableAssignmentDefinitionInput {
  return { ...definition, ...change };
}

function momentValue(moment: RelativeScheduleMoment | null): number | null {
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

function validateSchedule(schedule: RelativeAssignmentSchedule): CurriculumValidation {
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
export function validateReusableDefinition(
  definition: ReusableAssignmentDefinitionInput,
): CurriculumValidation {
  if (definition.title.trim().length === 0 || definition.title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return {
      valid: false,
      message: "Give this reusable assignment a title of up to 200 characters.",
    };
  }
  if (definition.entries.length === 0 || definition.entries.length > MAX_REUSABLE_ENTRIES) {
    return {
      valid: false,
      message: "Add at least one fixed Question or Question Pool before saving.",
    };
  }
  for (const entry of definition.entries) {
    if (entry.kind !== "pool") continue;
    if (entry.candidates.length === 0 || entry.candidates.length > MAX_POOL_CANDIDATES) {
      return {
        valid: false,
        message: "Each Question Pool needs from 1 through 1024 candidate Questions.",
      };
    }
    if (new Set(entry.candidates).size !== entry.candidates.length) {
      return { valid: false, message: "Each Question Pool candidate must appear only once." };
    }
    if (
      !Number.isSafeInteger(entry.draw_count) ||
      entry.draw_count < 1 ||
      entry.draw_count > entry.candidates.length
    ) {
      return {
        valid: false,
        message: "Choose a whole draw count between 1 and this pool's candidate count.",
      };
    }
  }
  return validateSchedule(definition.schedule);
}

/** Validates the complete local Blueprint Course tree before its create request. */
export function validateBlueprintCourseDefinition(
  definition: CreateBlueprintCourseDefinitionInput,
): CurriculumValidation {
  if (definition.title.trim().length === 0 || definition.title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return { valid: false, message: "Give this Blueprint Course a title of up to 200 characters." };
  }
  if (definition.modules.length === 0 || definition.modules.length > MAX_REUSABLE_ENTRIES) {
    return {
      valid: false,
      message: "Add at least one labelled module before creating the Blueprint Course.",
    };
  }
  for (const module of definition.modules) {
    if (module.label.trim().length === 0 || module.label.length > MAX_REUSABLE_TITLE_LENGTH) {
      return {
        valid: false,
        message: "Give each Blueprint Course module a label of up to 200 characters.",
      };
    }
    if (module.definitions.length === 0 || module.definitions.length > MAX_REUSABLE_ENTRIES) {
      return { valid: false, message: "Each module needs at least one reusable assignment." };
    }
    for (const assignment of module.definitions) {
      const validation = validateReusableDefinition(assignment);
      if (!validation.valid) return validation;
    }
  }
  return { valid: true, message: null };
}

function entryInputFromView(entry: ReusableAssignmentEntryView): ReusableAssignmentEntryInput {
  if (entry.kind === "pool") {
    return {
      kind: "pool",
      candidates: entry.candidates.map((candidate) => candidate.catalog.summary.questionId),
      draw_count: entry.draw_count,
      points_per_item: entry.points_per_item,
      ordering: entry.ordering,
      algorithm: entry.algorithm,
    };
  }
  return {
    kind: "fixed",
    question_id: entry.question.catalog.summary.questionId,
    points_possible: entry.points_possible,
    scoring_mode: entry.scoring_mode,
  };
}

export function reusableDefinitionInputFromView(
  definition: ReusableAssignmentDefinitionView,
): ReusableAssignmentDefinitionInput {
  return {
    title: definition.title,
    instructions: definition.instructions,
    entries: definition.entries.map(entryInputFromView),
    defaults: definition.defaults,
    schedule: definition.schedule,
  };
}

/** Converts a current Blueprint Course projection to an editable complete-tree replacement. */
export function replacementDefinitionFromBlueprintCourse(
  view: BlueprintCourseView,
): import("../../../generated/api/ReplaceBlueprintCourseDefinitionInput").ReplaceBlueprintCourseDefinitionInput {
  return {
    title: view.title,
    modules: view.modules.map((module) => ({
      handle: { kind: "retained", module_id: module.module_id },
      label: module.label,
      definitions: module.definitions.map((assignment) => ({
        handle: { kind: "retained", assignment_id: assignment.assignment_id },
        definition: reusableDefinitionInputFromView(assignment.definition),
      })),
    })),
  };
}
