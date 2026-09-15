// Immutable browser working state for the one reusable Blueprint Course model.

import type { BlueprintModuleView } from "../../../generated/api/BlueprintModuleView";
import type { BlueprintAvailability } from "../../../generated/api/BlueprintAvailability";
import type { BlueprintCourseReadAccess } from "../../../generated/api/BlueprintCourseReadAccess";
import type { CreateBlueprintCourseInput } from "../../../generated/api/CreateBlueprintCourseInput";
import type { BlueprintAssessmentDefaults } from "../../../generated/api/BlueprintAssessmentDefaults";
import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import type { BlueprintAssessmentEntryInput } from "../../../generated/api/BlueprintAssessmentEntryInput";
import type { BlueprintAssessmentEntryView } from "../../../generated/api/BlueprintAssessmentEntryView";
import type { AssessmentType } from "../../../generated/api/AssessmentType";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionPickerSelection } from "../question_picker";

export const MAX_REUSABLE_ENTRIES = 1024;
export const MAX_REUSABLE_TITLE_LENGTH = 200;

export type ReusableEntryDirection = -1 | 1;

export interface BlueprintCourseValidation {
  readonly valid: boolean;
  readonly message: string | null;
}

export interface BlueprintCourseContinuationPresentation {
  readonly visible: boolean;
  readonly action: string | null;
}

/** Visible lifecycle choices mirror the server's directed Blueprint state machine. */
export interface BlueprintLifecyclePresentation {
  readonly meaning: string;
  readonly canAdopt: boolean;
  readonly canEdit: boolean;
  readonly canPublish: boolean;
  readonly canArchive: boolean;
  readonly canRestore: boolean;
  readonly canReturnToPrivate: boolean;
}

/** Fails closed for a state/read-access combination the server would not expose. */
export function blueprintLifecyclePresentation(
  availability: BlueprintAvailability,
  readAccess: BlueprintCourseReadAccess,
): BlueprintLifecyclePresentation {
  const owner = readAccess === "blueprint_course_owner";
  if (availability === "private") {
    return {
      meaning:
        "Only you can view and edit this Blueprint Course. Publish it to let other Instructors browse and adopt it.",
      canAdopt: false,
      canEdit: owner,
      canPublish: owner,
      canArchive: false,
      canRestore: false,
      canReturnToPrivate: false,
    };
  }
  if (availability === "public") {
    return {
      meaning:
        "Instructors can browse and adopt this Blueprint Course. Its current Revision is reusable.",
      canAdopt: true,
      canEdit: false,
      canPublish: false,
      canArchive: owner,
      canRestore: false,
      canReturnToPrivate: owner,
    };
  }
  return {
    meaning:
      "This Blueprint Course is read-only and unavailable for new Course Instance creation. Restore it to Public to make it available again.",
    canAdopt: false,
    canEdit: false,
    canPublish: false,
    canArchive: false,
    canRestore: owner,
    canReturnToPrivate: false,
  };
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

function defaultDefaults(assessmentType: AssessmentType): BlueprintAssessmentDefaults {
  return {
    assessment_attempt_time_limit_seconds: null,
    assessment_attempt_limit: assessmentType === "quiz" || assessmentType === "exam" ? 1 : null,
    late_work_rule: "reject",
    activity_rules: {
      assessmentAttemptGradeRule: "highest",
      questionPoolReuseRule: "reuseSelection",
      questionVariationRule: "newVariation",
      assessmentAttemptResumeRule: "resumable",
      assessmentQuestionDisplayRule: "oneQuestionAtATime",
      assessmentNavigationRule: "freeNavigation",
      assessmentQuestionOrderRule: "shuffled",
    },
    student_feedback_release_rule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
      question_feedback: "never",
      question_answer:
        assessmentType === "practice_question_assignment"
          ? "after_submit"
          : assessmentType === "quiz" || assessmentType === "exam"
            ? "after_submit"
            : "never",
      question_answer_explanation:
        assessmentType === "quiz" || assessmentType === "exam" ? "after_submit" : "never",
      class_statistics: "never",
    },
  };
}

/** Builds an editable assessment content with visible teaching defaults. */
export function emptyReusableContent(
  assessmentType: AssessmentType,
  title = "Untitled Blueprint Assessment",
): BlueprintAssessmentContentInput {
  return {
    assessment_type: assessmentType,
    title,
    instructions: "",
    entries: [],
    defaults: defaultDefaults(assessmentType),
  };
}

/** Builds complete local Blueprint Course working state with one explicitly typed Assessment. */
export function emptyBlueprintCourseContent(
  assessmentType: AssessmentType,
): CreateBlueprintCourseInput {
  return {
    short_name: "Untitled Blueprint",
    long_name: "Untitled Blueprint Course",
    modules: [{ label: "Module 1", assessments: [emptyReusableContent(assessmentType)] }],
  };
}

function uniqueQuestionIds(selection: QuestionPickerSelection): ReadonlyArray<string> {
  return selection.questionIds.filter(
    (questionId, index, all) => all.indexOf(questionId) === index,
  );
}

function fixedEntry(questionId: string): BlueprintAssessmentEntryInput {
  return {
    kind: "fixed",
    question_id: questionId,
    points_possible: "1",
    scoring_rule: "normal",
    question_attempt_limit: { maxAttempts: null },
    question_attempt_time_limit: { kind: "unlimited" },
  };
}

function poolEntry(questionPoolId: QuestionId): BlueprintAssessmentEntryInput {
  return {
    kind: "pool",
    question_pool_id: questionPoolId,
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
  content: BlueprintAssessmentContentInput,
  selection: QuestionPickerSelection,
): BlueprintAssessmentContentInput {
  return {
    ...content,
    entries: [...content.entries, ...uniqueQuestionIds(selection).map(fixedEntry)],
  };
}

/** Appends one Question Pool with Question Pool Item order selected by the Instructor. */
export function appendPickedPool(
  content: BlueprintAssessmentContentInput,
  questionPoolId: QuestionId,
): BlueprintAssessmentContentInput {
  return { ...content, entries: [...content.entries, poolEntry(questionPoolId)] };
}

export function moveReusableEntry(
  content: BlueprintAssessmentContentInput,
  index: number,
  direction: ReusableEntryDirection,
): BlueprintAssessmentContentInput {
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
  content: BlueprintAssessmentContentInput,
  index: number,
): BlueprintAssessmentContentInput {
  if (index < 0 || index >= content.entries.length) return content;
  return {
    ...content,
    entries: content.entries.filter((_, entryIndex) => entryIndex !== index),
  };
}

export function updateReusablePoolSelectionCount(
  content: BlueprintAssessmentContentInput,
  index: number,
  selectionCount: number,
): BlueprintAssessmentContentInput {
  if (!Number.isSafeInteger(selectionCount) || selectionCount < 1) return content;
  const entry = content.entries[index];
  if (entry === undefined || entry.kind !== "pool") return content;
  const entries = [...content.entries];
  entries[index] = { ...entry, selection_count: selectionCount };
  return { ...content, entries };
}

export function updateReusableDefaults(
  content: BlueprintAssessmentContentInput,
  defaults: BlueprintAssessmentDefaults,
): BlueprintAssessmentContentInput {
  return { ...content, defaults };
}

export function updateReusableText(
  content: BlueprintAssessmentContentInput,
  change: Partial<Pick<BlueprintAssessmentContentInput, "title" | "instructions">>,
): BlueprintAssessmentContentInput {
  return { ...content, ...change };
}

/** Guides local authoring before the server performs authoritative validation. */
export function validateReusableContent(
  content: BlueprintAssessmentContentInput,
): BlueprintCourseValidation {
  if (content.title.trim().length === 0 || content.title.length > MAX_REUSABLE_TITLE_LENGTH) {
    return {
      valid: false,
      message: "Give this Blueprint Assessment a title of up to 200 characters.",
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
    if (!Number.isSafeInteger(entry.selection_count) || entry.selection_count < 1) {
      return {
        valid: false,
        message: "Choose a positive whole Question Pool selection count.",
      };
    }
  }
  return { valid: true, message: null };
}

/** Validates the complete local Blueprint Course tree before its create request. */
export function validateBlueprintCourseContent(
  content: CreateBlueprintCourseInput,
): BlueprintCourseValidation {
  if (
    content.short_name.trim().length === 0 ||
    content.short_name.length > MAX_REUSABLE_TITLE_LENGTH ||
    content.long_name.trim().length === 0 ||
    content.long_name.length > MAX_REUSABLE_TITLE_LENGTH
  ) {
    return {
      valid: false,
      message: "Give this Blueprint Course short and long names of up to 200 characters.",
    };
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
    if (module.assessments.length === 0 || module.assessments.length > MAX_REUSABLE_ENTRIES) {
      return { valid: false, message: "Each module needs at least one Blueprint Assessment." };
    }
    for (const assessment of module.assessments) {
      const validation = validateReusableContent(assessment);
      if (!validation.valid) return validation;
    }
  }
  return { valid: true, message: null };
}

function entryInputFromView(entry: BlueprintAssessmentEntryView): BlueprintAssessmentEntryInput {
  if (entry.kind === "pool") {
    return {
      kind: "pool",
      question_pool_id: entry.question_pool_revision.questionPoolId,
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
  content: BlueprintAssessmentContentView,
): BlueprintAssessmentContentInput {
  return {
    assessment_type: content.assessment_type,
    title: content.title,
    instructions: content.instructions,
    entries: content.entries.map(entryInputFromView),
    defaults: content.defaults,
  };
}

/** Converts exact immutable Revision content to a complete editable Save command. */
export function replacementContentFromBlueprintModules(
  modules: ReadonlyArray<BlueprintModuleView>,
): import("../../../generated/api/ReplaceBlueprintCourseContentInput").ReplaceBlueprintCourseContentInput {
  return {
    modules: modules.map((module) => ({
      choice: {
        kind: "retained",
        blueprint_module_reference: module.blueprint_module_reference,
      },
      label: module.label,
      assessments: module.assessments.map((assessment) => ({
        choice: {
          kind: "retained",
          blueprint_assessment_reference: assessment.blueprint_assessment_reference,
        },
        content: reusableContentInputFromView(assessment.content),
      })),
    })),
  };
}
