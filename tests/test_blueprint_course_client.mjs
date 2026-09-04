import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeBlueprintCourseView,
  decodeReplaceBlueprintCourseContentInput,
} from "../src/api/decoders/blueprint_course.ts";
import {
  ApiProtocolError,
  BlueprintCourseConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const { scope: _retiredPublicationScope, ...publishedQuestion } =
  publishedQuestionFixture.publishedQuestion;

function contentInput() {
  return {
    title: "Peptide fundamentals",
    instructions: "Use your course notes to explain each choice.",
    entries: [
      {
        kind: "fixed",
        question_id: publishedQuestion.questionId,
        points_possible: "2",
        scoring_rule: "normal",
      },
    ],
    defaults: {
      assignment_attempt_time_limit_seconds: null,
      attempt_limit: 2,
      late_work_rule: "accept",
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
        question_answer: "never",
        question_answer_explanation: "never",
        class_statistics: "never",
      },
    },
    schedule: { available_at: null, due_at: null, closes_at: null },
  };
}

function blueprint(revision = "7") {
  return {
    reference: "BP-7",
    title: "Biochemistry sequence",
    revision,
    read_access: "blueprint_course_owner",
    modules: [
      {
        blueprint_module_reference: "module-7",
        label: "Week one",
        assignments: [
          {
            blueprint_assignment_reference: "assignment-7",
            content: {
              ...contentInput(),
              entries: [
                {
                  kind: "fixed",
                  question: {
                    question_library: {
                      summary: publishedQuestion,
                      evidence: { state: "unavailable" },
                    },
                    selection_availability: "available",
                  },
                  points_possible: "2",
                  scoring_rule: "normal",
                },
                {
                  kind: "pool",
                  items: [
                    {
                      question_library: {
                        summary: publishedQuestion,
                        evidence: { state: "unavailable" },
                      },
                      selection_availability: "available",
                    },
                  ],
                  selection_count: 1,
                  points_per_item: "2",
                  scoring_rule: "normal",
                  selection_rule: { selected_question_order: "questionPoolOrder" },
                },
              ],
            },
          },
        ],
      },
    ],
  };
}

function creationInput() {
  return {
    title: "Biochemistry sequence",
    modules: [{ label: "Week one", assignments: [contentInput()] }],
  };
}

function replacementInput() {
  return {
    title: "Biochemistry sequence",
    modules: [
      {
        choice: { kind: "retained", blueprint_module_reference: "module-7" },
        label: "Week one",
        assignments: [
          {
            choice: {
              kind: "retained",
              blueprint_assignment_reference: "assignment-7",
            },
            content: contentInput(),
          },
        ],
      },
    ],
  };
}

function noStoreJson(value, etag, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
      ...(etag === undefined ? {} : { etag }),
    },
  });
}

test("B1 Blueprint Course decoder keeps views answer-free and rejects hostile fields", () => {
  assert.equal(decodeBlueprintCourseView(blueprint()).reference, "BP-7");
  const activeInstructor = structuredClone(blueprint());
  activeInstructor.read_access = "active_instructor";
  assert.equal(decodeBlueprintCourseView(activeInstructor).read_access, "active_instructor");
  const hostile = structuredClone(blueprint());
  hostile.modules[0].assignments[0].content.entries[0].question.answerKey = "secret";
  assert.throws(() => decodeBlueprintCourseView(hostile), DecodeError);
  const retiredGlobalStatistics = structuredClone(blueprint());
  retiredGlobalStatistics.modules[0].assignments[0].content.entries[0].question.question_library.evidence =
    {
      state: "available",
      observedCourseCount: 2,
      independentLearnerObservationCount: 5,
      difficultyIndex: 0.7,
      attemptsMean: 1.2,
      timeMedianSecondsEstimate: 30,
      evidenceAt: 0,
    };
  assert.throws(() => decodeBlueprintCourseView(retiredGlobalStatistics), DecodeError);
  const retiredPoolField = structuredClone(blueprint());
  const pool = retiredPoolField.modules[0].assignments[0].content.entries[1];
  pool.entries = pool.items;
  delete pool.items;
  assert.throws(() => decodeBlueprintCourseView(retiredPoolField), DecodeError);
  const retiredAccess = structuredClone(blueprint());
  retiredAccess.access = "owner";
  assert.throws(() => decodeBlueprintCourseView(retiredAccess), DecodeError);
  const retiredAssignmentId = structuredClone(blueprint());
  const assignment = retiredAssignmentId.modules[0].assignments[0];
  assignment.assignment_id = assignment.blueprint_assignment_reference;
  delete assignment.blueprint_assignment_reference;
  assert.throws(() => decodeBlueprintCourseView(retiredAssignmentId), DecodeError);
  const retiredEditChoice = replacementInput();
  const choice = retiredEditChoice.modules[0].assignments[0].choice;
  choice.assignment_id = choice.blueprint_assignment_reference;
  delete choice.blueprint_assignment_reference;
  assert.throws(() => decodeReplaceBlueprintCourseContentInput(retiredEditChoice), DecodeError);
});

test("B1 client uses canonical Blueprint Course commands and matching ETags", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (request.method === "GET" && path.endsWith("BP-7")) return noStoreJson(blueprint(), '"7"');
      if (request.method === "POST") return noStoreJson(blueprint(), '"7"', 201);
      if (request.method === "PUT") return noStoreJson(blueprint("8"), '"8"');
      if (request.method === "DELETE")
        return new Response(null, { status: 204, headers: { "cache-control": "no-store" } });
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  const current = await client.getBlueprintCourse("BP-7");
  await client.createBlueprintCourse(creationInput());
  const revised = await client.replaceBlueprintCourse("BP-7", replacementInput(), current.etag);
  await client.deleteBlueprintCourse("BP-7", revised.etag);
  const update = requests.find(
    (request) => request.method === "PUT" && request.url.endsWith("BP-7"),
  );
  assert.equal(update.headers.get("if-match"), '"7"');
  assert.equal(update.cache, "no-store");
  assert.equal(update.credentials, "same-origin");
  await assert.rejects(
    client.replaceBlueprintCourse("BP-7", replacementInput(), '"07"'),
    ApiProtocolError,
  );
  await assert.rejects(
    client.createBlueprintCourse({
      ...creationInput(),
      unrecognizedCreator: { displayName: "forged" },
    }),
    DecodeError,
  );
});

test("B1 client gives a typed conflict for a current Blueprint Course replacement", async () => {
  const conflict = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    conflict.replaceBlueprintCourse("BP-7", replacementInput(), '"7"'),
    BlueprintCourseConflictError,
  );
  await assert.rejects(conflict.getBlueprintCourse("BP-00"), DecodeError);
});
