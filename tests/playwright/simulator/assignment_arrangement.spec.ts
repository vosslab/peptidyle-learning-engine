// assignment_arrangement.spec.ts - contract tests for later seeded-course assignment arrangement.

import { expect, test } from "@playwright/test";

import {
  arrangeSeededCourseAssignments,
  AssignmentArrangementError,
  type InstructorArrangementApi,
  type InstructorArrangementResponse,
} from "./assignment_arrangement";
import { examContrastTitle, masteryRetryTitle } from "./assignment_titles";

const BASELINE_ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174000";
const COURSE = "123e4567-e89b-12d3-a456-426614174001";
const QUESTION = {
  questionId: "7K3-M9QP",
};
const MASTERY_ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174004";
const EXAM_ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174005";
const MASTERY_TITLE = masteryRetryTitle(QUESTION.questionId);
const EXAM_TITLE = examContrastTitle(QUESTION.questionId);

interface CapturedRequest {
  readonly method: "get" | "post";
  readonly path: string;
  readonly data?: unknown;
  readonly headers?: Readonly<Record<string, string>>;
}

interface Reply {
  readonly status: number;
  readonly payload: unknown;
  readonly requestError?: Error;
  readonly jsonError?: Error;
}

interface PolicyBody {
  readonly completion: { readonly kind: "allCorrect" | "answerAll" };
  readonly grade: "highest" | "first";
  readonly continuedPractice: { readonly kind: "unlimited" | "closed" };
  readonly variation: "newSeeds";
}

function policies(kind: "mastery" | "exam"): PolicyBody {
  return kind === "mastery"
    ? {
        completion: { kind: "allCorrect" },
        grade: "highest",
        continuedPractice: { kind: "unlimited" },
        variation: "newSeeds",
      }
    : {
        completion: { kind: "answerAll" },
        grade: "first",
        continuedPractice: { kind: "closed" },
        variation: "newSeeds",
      };
}

function assignment(
  id: string,
  courseId = COURSE,
  policyKind: "mastery" | "exam" = "mastery",
): Readonly<{ id: string; courseId: string; policies: PolicyBody }> {
  return { id, courseId, policies: policies(policyKind) };
}

function fakeApi(replies: readonly Reply[]): {
  readonly api: InstructorArrangementApi;
  readonly captured: CapturedRequest[];
} {
  const captured: CapturedRequest[] = [];
  let index = 0;
  function nextReply(): Reply {
    const reply = replies[index];
    index += 1;
    if (reply === undefined) throw new Error("unexpected supported-API request");
    return reply;
  }
  function response(reply: Reply): InstructorArrangementResponse {
    return {
      status: () => reply.status,
      json: () =>
        reply.jsonError === undefined
          ? Promise.resolve(reply.payload)
          : Promise.reject(reply.jsonError),
    };
  }
  return {
    api: {
      get: (path: string): Promise<InstructorArrangementResponse> => {
        captured.push({ method: "get", path });
        const reply = nextReply();
        return reply.requestError === undefined
          ? Promise.resolve(response(reply))
          : Promise.reject(reply.requestError);
      },
      post: (
        path: string,
        request: {
          readonly data: unknown;
          readonly headers: Readonly<Record<string, string>>;
        },
      ): Promise<InstructorArrangementResponse> => {
        captured.push({ method: "post", path, ...request });
        const reply = nextReply();
        return reply.requestError === undefined
          ? Promise.resolve(response(reply))
          : Promise.reject(reply.requestError);
      },
    },
    captured,
  };
}

test("resolves the seeded course and posts exactly Mastery then Exam", async () => {
  const fake = fakeApi([
    { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
    { status: 201, payload: assignment(MASTERY_ASSIGNMENT, COURSE, "mastery") },
    { status: 201, payload: assignment(EXAM_ASSIGNMENT, COURSE, "exam") },
  ]);
  await expect(
    arrangeSeededCourseAssignments(fake.api, { assignmentId: BASELINE_ASSIGNMENT }, QUESTION),
  ).resolves.toEqual({
    arrangement: "seeded-course-assignments",
    baselineAssignmentId: BASELINE_ASSIGNMENT,
    courseId: COURSE,
    masteryAssignmentId: MASTERY_ASSIGNMENT,
    examAssignmentId: EXAM_ASSIGNMENT,
  });
  expect(fake.captured).toEqual([
    { method: "get", path: `/api/assignments/${BASELINE_ASSIGNMENT}` },
    {
      method: "post",
      path: `/api/courses/${COURSE}/assignments`,
      headers: { "content-type": "application/json" },
      data: {
        title: MASTERY_TITLE,
        questionIds: [QUESTION.questionId],
        policies: policies("mastery"),
      },
    },
    {
      method: "post",
      path: `/api/courses/${COURSE}/assignments`,
      headers: { "content-type": "application/json" },
      data: {
        title: EXAM_TITLE,
        questionIds: [QUESTION.questionId],
        policies: policies("exam"),
      },
    },
  ]);
});

test("derives public titles from the assigned Question ID", () => {
  expect(MASTERY_TITLE).toBe("Peptide mastery retry 7K3-M9QP");
  expect(EXAM_TITLE).toBe("Peptide exam contrast 7K3-M9QP");
});

const SENSITIVE_SENTINEL = "student-identity-and-response-must-not-leak";

async function expectRedactedTransportFailure(
  replies: readonly Reply[],
  stage: "baseline-read" | "mastery-create" | "exam-create",
  requestCount: number,
): Promise<void> {
  const fake = fakeApi(replies);
  let thrown: unknown;
  try {
    await arrangeSeededCourseAssignments(fake.api, { assignmentId: BASELINE_ASSIGNMENT }, QUESTION);
  } catch (error: unknown) {
    thrown = error;
  }
  expect(thrown).toMatchObject({ name: "AssignmentArrangementError", stage });
  expect(String(thrown)).not.toContain(SENSITIVE_SENTINEL);
  expect(fake.captured).toHaveLength(requestCount);
}

test("redacts a rejected baseline GET before any assignment creation", async () => {
  await expectRedactedTransportFailure(
    [{ status: 0, payload: {}, requestError: new Error(SENSITIVE_SENTINEL) }],
    "baseline-read",
    1,
  );
});

test("redacts a rejected baseline JSON body before any assignment creation", async () => {
  await expectRedactedTransportFailure(
    [{ status: 200, payload: {}, jsonError: new Error(SENSITIVE_SENTINEL) }],
    "baseline-read",
    1,
  );
});

test("redacts a rejected Mastery POST before the Exam request", async () => {
  await expectRedactedTransportFailure(
    [
      { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
      { status: 0, payload: {}, requestError: new Error(SENSITIVE_SENTINEL) },
    ],
    "mastery-create",
    2,
  );
});

test("redacts a rejected Mastery JSON body before the Exam request", async () => {
  await expectRedactedTransportFailure(
    [
      { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
      { status: 201, payload: {}, jsonError: new Error(SENSITIVE_SENTINEL) },
    ],
    "mastery-create",
    2,
  );
});

test("redacts a rejected Exam POST without retrying either assignment", async () => {
  await expectRedactedTransportFailure(
    [
      { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
      { status: 201, payload: assignment(MASTERY_ASSIGNMENT, COURSE, "mastery") },
      { status: 0, payload: {}, requestError: new Error(SENSITIVE_SENTINEL) },
    ],
    "exam-create",
    3,
  );
});

test("redacts a rejected Exam JSON body without retrying either assignment", async () => {
  await expectRedactedTransportFailure(
    [
      { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
      { status: 201, payload: assignment(MASTERY_ASSIGNMENT, COURSE, "mastery") },
      { status: 201, payload: {}, jsonError: new Error(SENSITIVE_SENTINEL) },
    ],
    "exam-create",
    3,
  );
});

test("stops before arrangement when the summary or Question ID is unsafe", async () => {
  const fake = fakeApi([]);
  await expect(
    arrangeSeededCourseAssignments(fake.api, { assignmentId: "not-an-id" }, QUESTION),
  ).rejects.toMatchObject({ name: "AssignmentArrangementError", stage: "input" });
  await expect(
    arrangeSeededCourseAssignments(
      fake.api,
      { assignmentId: BASELINE_ASSIGNMENT },
      { ...QUESTION, questionId: "bad" },
    ),
  ).rejects.toBeInstanceOf(AssignmentArrangementError);
  expect(fake.captured).toEqual([]);
});

test("does not create either assignment when the baseline response is invalid", async () => {
  const fake = fakeApi([
    { status: 200, payload: assignment("123e4567-e89b-12d3-a456-426614174099") },
  ]);
  await expect(
    arrangeSeededCourseAssignments(fake.api, { assignmentId: BASELINE_ASSIGNMENT }, QUESTION),
  ).rejects.toMatchObject({ stage: "baseline-read" });
  expect(fake.captured).toEqual([
    { method: "get", path: `/api/assignments/${BASELINE_ASSIGNMENT}` },
  ]);
});

test("validates the created Mastery policy before the Exam request", async () => {
  const fake = fakeApi([
    { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
    { status: 201, payload: assignment(MASTERY_ASSIGNMENT, COURSE, "exam") },
  ]);
  await expect(
    arrangeSeededCourseAssignments(fake.api, { assignmentId: BASELINE_ASSIGNMENT }, QUESTION),
  ).rejects.toMatchObject({ stage: "mastery-create" });
  expect(fake.captured.map(({ method, path }) => ({ method, path }))).toEqual([
    { method: "get", path: `/api/assignments/${BASELINE_ASSIGNMENT}` },
    { method: "post", path: `/api/courses/${COURSE}/assignments` },
  ]);
});

test("validates the created Mastery course before the Exam request", async () => {
  const fake = fakeApi([
    { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
    {
      status: 201,
      payload: assignment(MASTERY_ASSIGNMENT, "123e4567-e89b-12d3-a456-426614174088", "mastery"),
    },
  ]);
  await expect(
    arrangeSeededCourseAssignments(fake.api, { assignmentId: BASELINE_ASSIGNMENT }, QUESTION),
  ).rejects.toMatchObject({ stage: "mastery-create" });
  expect(fake.captured).toHaveLength(2);
});

test("does not mutate the caller-owned manifest or public question reference", async () => {
  const manifest = { assignmentId: BASELINE_ASSIGNMENT };
  const question = { ...QUESTION };
  const fake = fakeApi([
    { status: 200, payload: assignment(BASELINE_ASSIGNMENT) },
    { status: 201, payload: assignment(MASTERY_ASSIGNMENT, COURSE, "mastery") },
    { status: 201, payload: assignment(EXAM_ASSIGNMENT, COURSE, "exam") },
  ]);
  await arrangeSeededCourseAssignments(fake.api, manifest, question);
  expect(manifest).toEqual({ assignmentId: BASELINE_ASSIGNMENT });
  expect(question).toEqual(QUESTION);
});
