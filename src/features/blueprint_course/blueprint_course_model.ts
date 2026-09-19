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
import type { QuestionPoolEditNumber } from "../../../generated/api/QuestionPoolEditNumber";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
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
      // ASVS 8.2.2: Per-Blueprint owner access controls this edit affordance; the server
      // independently authorizes every mutation.
      canEdit: owner,
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
export function appendBlueprintCoursePage<Record extends { readonly id: string }>(
  current: ReadonlyArray<Record>,
  incoming: ReadonlyArray<Record>,
): ReadonlyArray<Record> {
  const known = new Set(current.map((record) => record.id));
  return [...current, ...incoming.filter((record) => !known.has(record.id))];
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
      questionVariationRule: "newVariation",
      assessmentQuestionOrderRule: "shuffled",
    },
    student_feedback_release_rule: {
      score: "after_submit",
      per_item_correctness: "after_submit",
      submitted_response: "after_submit",
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
  classification: import("../../../generated/api/CourseClassification").CourseClassification,
): CreateBlueprintCourseInput {
  return {
    classification,
    short_name: "Untitled Blueprint",
    long_name: "Untitled Blueprint Course",
    modules: [{ label: "Module 1", assessments: [emptyReusableContent(assessmentType)] }],
  };
}

function fixedEntry(publishedQuestion: QuestionRevisionReference): BlueprintAssessmentEntryInput {
  return {
    kind: "fixed",
    published_question: publishedQuestion,
    points_possible: "1",
    scoring_rule: "normal",
    question_attempt_limit: { maxAttempts: null },
    question_attempt_time_limit: { kind: "unlimited" },
  };
}

function poolEntry(
  questionPoolId: QuestionId,
  questionPoolEditNumber: QuestionPoolEditNumber,
): BlueprintAssessmentEntryInput {
  return {
    kind: "pool",
    pool: {
      kind: "import",
      question_pool_id: questionPoolId,
      question_pool_edit_number: questionPoolEditNumber,
    },
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
    entries: [
      ...content.entries,
      ...selection.questions.map((question) => fixedEntry(question.row.questionRevision)),
    ],
  };
}

/** Appends one Question Pool with Question Pool Item order selected by the Instructor. */
export function appendPickedPool(
  content: BlueprintAssessmentContentInput,
  questionPoolId: QuestionId,
  questionPoolEditNumber: QuestionPoolEditNumber,
): BlueprintAssessmentContentInput {
  return {
    ...content,
    entries: [...content.entries, poolEntry(questionPoolId, questionPoolEditNumber)],
  };
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
    if (entry.kind === "fixed") {
      // ASVS 2.2.1: a fixed selection carries its complete immutable Revision identity.
      if (
        !entry.published_question?.questionId ||
        !Number.isSafeInteger(entry.published_question.revisionNumber) ||
        entry.published_question.revisionNumber < 1
      ) {
        return { valid: false, message: "Choose a fixed Question with a published Revision." };
      }
      continue;
    }
    if (entry.kind !== "pool") continue;
    if (
      !Number.isSafeInteger(entry.selection_count) ||
      entry.selection_count < 1 ||
      entry.selection_count > 4_294_967_295
    ) {
      return {
        valid: false,
        message: "Choose a positive whole Question Pool selection count.",
      };
    }
    if (entry.pool.kind === "retained" && entry.pool.members !== null) {
      const members = entry.pool.members;
      if (
        members.length === 0 ||
        members.length > MAX_REUSABLE_ENTRIES ||
        entry.selection_count > members.length ||
        new Set(members.map((member) => member.questionId)).size !== members.length ||
        members.some(
          (member) =>
            !member.questionId ||
            !Number.isSafeInteger(member.revisionNumber) ||
            member.revisionNumber < 1,
        ) ||
        !entry.pool.interchangeabilityAttested
      ) {
        return {
          valid: false,
          message:
            "Choose unique Pool members, review their interchangeability, and keep the selection count within the member count.",
        };
      }
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
      pool: {
        kind: "retained",
        question_pool_id: entry.question_pool_id,
        question_pool_edit_number: entry.question_pool_edit_number,
        members: null,
        interchangeabilityAttested: false,
      },
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
    published_question: entry.question.reference,
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
          blueprint_assessment_id: assessment.blueprint_assessment_id,
        },
        content: reusableContentInputFromView(assessment.content),
      })),
    })),
  };
}
