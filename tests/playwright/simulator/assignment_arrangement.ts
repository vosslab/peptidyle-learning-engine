// assignment_arrangement.ts - supported-API arrangement of seeded-course assignments.

import { examContrastTitle, masteryRetryTitle } from "./assignment_titles";

const IDENTIFIER = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const QUESTION_ID = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;

export interface LauncherManifestSummary {
  readonly assignmentId: string;
}

export interface QuestionRef {
  readonly questionId: string;
}

export interface InstructorArrangementResponse {
  status(): number;
  json(): Promise<unknown>;
}

/** An already-authenticated instructor context; authentication remains another owner. */
export interface InstructorArrangementApi {
  get(path: string): Promise<InstructorArrangementResponse>;
  post(
    path: string,
    request: {
      readonly data: AssignmentCreateInput;
      readonly headers: Readonly<Record<"content-type", "application/json">>;
    },
  ): Promise<InstructorArrangementResponse>;
}

export interface AssignmentArrangement {
  readonly arrangement: "seeded-course-assignments";
  readonly baselineAssignmentId: string;
  readonly courseId: string;
  readonly masteryAssignmentId: string;
  readonly examAssignmentId: string;
}

export type AssignmentArrangementStage =
  "input" | "baseline-read" | "mastery-create" | "exam-create";

/** A redacted supported-API failure that never carries identity or roster details. */
export class AssignmentArrangementError extends Error {
  public readonly stage: AssignmentArrangementStage;

  public constructor(stage: AssignmentArrangementStage) {
    super(`assignment arrangement failed during ${stage}`);
    this.name = "AssignmentArrangementError";
    this.stage = stage;
  }
}

interface AssignmentPolicies {
  readonly completion: { readonly kind: "allCorrect" | "answerAll" };
  readonly grade: "highest" | "first";
  readonly continuedPractice: { readonly kind: "unlimited" | "closed" };
  readonly variation: "newSeeds";
}

interface AssignmentCreateInput {
  readonly title: string;
  readonly questionIds: readonly string[];
  readonly policies: AssignmentPolicies;
}

interface AssignmentResponse {
  readonly id: string;
  readonly courseId: string;
  readonly policies: AssignmentPolicies;
}

const JSON_HEADERS = { "content-type": "application/json" } as const;

const MASTERY_POLICIES: AssignmentPolicies = {
  completion: { kind: "allCorrect" },
  grade: "highest",
  continuedPractice: { kind: "unlimited" },
  variation: "newSeeds",
};

const EXAM_POLICIES: AssignmentPolicies = {
  completion: { kind: "answerAll" },
  grade: "first",
  continuedPractice: { kind: "closed" },
  variation: "newSeeds",
};

/**
 * Reuses the launcher-seeded course and creates only the later Mastery and Exam arrangements.
 * Product assignment creation owns enrollment reconciliation; this module never reads or writes it.
 */
export async function arrangeSeededCourseAssignments(
  api: InstructorArrangementApi,
  manifest: LauncherManifestSummary,
  question: QuestionRef,
): Promise<AssignmentArrangement> {
  validateManifestAndQuestion(manifest, question);
  const baseline = await readBaselineAssignment(api, manifest.assignmentId);
  const mastery = await createAssignment(
    api,
    baseline.courseId,
    masteryInput(question),
    "mastery-create",
  );
  const exam = await createAssignment(api, baseline.courseId, examInput(question), "exam-create");
  const arrangement = {
    arrangement: "seeded-course-assignments" as const,
    baselineAssignmentId: manifest.assignmentId,
    courseId: baseline.courseId,
    masteryAssignmentId: mastery.id,
    examAssignmentId: exam.id,
  };
  return arrangement;
}

function validateManifestAndQuestion(
  manifest: LauncherManifestSummary,
  question: QuestionRef,
): void {
  if (!isIdentifier(manifest.assignmentId) || !QUESTION_ID.test(question.questionId)) {
    throw new AssignmentArrangementError("input");
  }
}

async function readBaselineAssignment(
  api: InstructorArrangementApi,
  assignmentId: string,
): Promise<AssignmentResponse> {
  try {
    const response = await api.get(`/api/assignments/${assignmentId}`);
    if (response.status() !== 200) throw new AssignmentArrangementError("baseline-read");
    const assignment = parseAssignmentResponse(await response.json());
    if (assignment === undefined || assignment.id !== assignmentId) {
      throw new AssignmentArrangementError("baseline-read");
    }
    return assignment;
  } catch (error: unknown) {
    if (error instanceof AssignmentArrangementError) throw error;
    throw new AssignmentArrangementError("baseline-read");
  }
}

function masteryInput(question: QuestionRef): AssignmentCreateInput {
  const input = {
    title: masteryRetryTitle(question.questionId),
    questionIds: [question.questionId],
    policies: MASTERY_POLICIES,
  };
  return input;
}

function examInput(question: QuestionRef): AssignmentCreateInput {
  const input = {
    title: examContrastTitle(question.questionId),
    questionIds: [question.questionId],
    policies: EXAM_POLICIES,
  };
  return input;
}

async function createAssignment(
  api: InstructorArrangementApi,
  courseId: string,
  input: AssignmentCreateInput,
  stage: "mastery-create" | "exam-create",
): Promise<AssignmentResponse> {
  try {
    const response = await api.post(`/api/courses/${courseId}/assignments`, {
      headers: JSON_HEADERS,
      data: input,
    });
    if (response.status() !== 201) throw new AssignmentArrangementError(stage);
    const assignment = parseAssignmentResponse(await response.json());
    if (
      assignment === undefined ||
      assignment.courseId !== courseId ||
      !samePolicies(assignment.policies, input.policies)
    ) {
      throw new AssignmentArrangementError(stage);
    }
    return assignment;
  } catch (error: unknown) {
    if (error instanceof AssignmentArrangementError) throw error;
    throw new AssignmentArrangementError(stage);
  }
}

function parseAssignmentResponse(value: unknown): AssignmentResponse | undefined {
  if (!isRecord(value)) return undefined;
  const { id, courseId, policies } = value;
  if (!isIdentifier(id) || !isIdentifier(courseId)) return undefined;
  const parsedPolicies = parsePolicies(policies);
  if (parsedPolicies === undefined) return undefined;
  return { id, courseId, policies: parsedPolicies };
}

function parsePolicies(value: unknown): AssignmentPolicies | undefined {
  if (!isRecord(value)) return undefined;
  const { completion, grade, continuedPractice, variation } = value;
  if (!isRecord(completion) || !isRecord(continuedPractice)) return undefined;
  if (
    (completion.kind !== "allCorrect" && completion.kind !== "answerAll") ||
    (grade !== "highest" && grade !== "first") ||
    (continuedPractice.kind !== "unlimited" && continuedPractice.kind !== "closed") ||
    variation !== "newSeeds"
  ) {
    return undefined;
  }
  return {
    completion: { kind: completion.kind },
    grade,
    continuedPractice: { kind: continuedPractice.kind },
    variation,
  };
}

function samePolicies(left: AssignmentPolicies, right: AssignmentPolicies): boolean {
  return (
    left.completion.kind === right.completion.kind &&
    left.grade === right.grade &&
    left.continuedPractice.kind === right.continuedPractice.kind &&
    left.variation === right.variation
  );
}

function isIdentifier(value: unknown): value is string {
  return typeof value === "string" && IDENTIFIER.test(value);
}

function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
