import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeBlueprintCourseView } from "../src/api/decoders/blueprint_course.ts";
import {
  ApiProtocolError,
  BlueprintCourseConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const { scope: _scope, ...publishedQuestion } = publishedQuestionFixture.publishedQuestion;

function contentInput() {
  return {
    title: "Peptide fundamentals",
    instructions: "Use your course notes.",
    entries: [
      {
        kind: "fixed",
        question_id: publishedQuestion.questionId,
        points_possible: "2",
        scoring_rule: "normal",
        question_attempt_limit: { maxAttempts: null },
        question_attempt_time_limit: { kind: "unlimited" },
      },
    ],
    defaults: {
      assignment_attempt_time_limit_seconds: null,
      attempt_limit: 2,
      late_work_rule: "accept",
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
        submitted_response: "after_submit",
        question_feedback: "after_submit",
        question_answer: "never",
        question_answer_explanation: "never",
        class_statistics: "never",
      },
    },
    schedule: { available_at: null, due_at: null, closes_at: null },
  };
}

function modules() {
  return [
    {
      blueprint_module_reference: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
      label: "Week one",
      assignments: [
        {
          blueprint_assignment_reference: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
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
                question_attempt_limit: { maxAttempts: null },
                question_attempt_time_limit: { kind: "unlimited" },
              },
            ],
          },
        },
      ],
    },
  ];
}

function blueprint(editNumber = "7") {
  return {
    reference: "BP-7",
    title: "Biochemistry sequence",
    availability: "available",
    availability_edit_number: "4",
    latest_published_revision: { reference: "BP-7", revision: "3" },
    read_access: "blueprint_course_owner",
    draft: { edit_number: editNumber, modules: modules() },
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
        choice: {
          kind: "retained",
          blueprint_module_reference: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
        },
        label: "Week one",
        assignments: [
          {
            choice: {
              kind: "retained",
              blueprint_assignment_reference: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
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

test("B1 Blueprint Course decoder keeps private Draft state separate from immutable publication", () => {
  assert.equal(decodeBlueprintCourseView(blueprint()).draft.edit_number, "7");
  const hostile = structuredClone(blueprint());
  hostile.draft.modules[0].assignments[0].content.entries[0].question.answerKey = "secret";
  assert.throws(() => decodeBlueprintCourseView(hostile), DecodeError);
  const retired = structuredClone(blueprint());
  retired.revision = "7";
  assert.throws(() => decodeBlueprintCourseView(retired), DecodeError);
});

test("B1 client preserves separate Draft and availability ETags", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/publish"))
        return noStoreJson({ blueprintRevision: { reference: "BP-7", revision: "4" } });
      if (path.endsWith("/archive"))
        return noStoreJson({ availability: "archived", editNumber: "5" }, '"5"');
      if (path.endsWith("/revisions/3"))
        return noStoreJson({
          blueprintRevision: { reference: "BP-7", revision: "3" },
          title: "Biochemistry sequence",
          modules: modules(),
        });
      if (request.method === "GET" && path.endsWith("BP-7")) return noStoreJson(blueprint(), '"7"');
      if (request.method === "POST") return noStoreJson(blueprint(), '"7"', 201);
      if (request.method === "PUT") return noStoreJson(blueprint("8"), '"8"');
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  const current = await client.getBlueprintCourse("BP-7");
  assert.notEqual(current.draftEtag, undefined);
  await client.createBlueprintCourse(creationInput(), "create-7");
  await client.saveBlueprintDraft("BP-7", replacementInput(), current.draftEtag, "save-7");
  const publication = await client.publishBlueprintDraft("BP-7", current.draftEtag, "publish-7");
  const archive = await client.archiveBlueprintCourse("BP-7", "Biochemistry sequence", '"4"');
  const revision = await client.getBlueprintRevision("BP-7", "3");
  assert.equal(publication.revision, "4");
  assert.equal(archive.etag, '"5"');
  assert.equal(revision.blueprintRevision.revision, "3");
  const save = requests.find((request) => request.method === "PUT");
  assert.equal(save.headers.get("if-match"), '"7"');
  assert.equal(save.headers.get("idempotency-key"), "save-7");
  await assert.rejects(
    client.saveBlueprintDraft("BP-7", replacementInput(), '"07"', "save-7"),
    ApiProtocolError,
  );
  await assert.rejects(
    client.createBlueprintCourse({ ...creationInput(), forged: true }, "create-8"),
    DecodeError,
  );
});

test("B1 client gives a typed conflict for a current Blueprint Draft save", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    client.saveBlueprintDraft("BP-7", replacementInput(), '"7"', "save-7"),
    BlueprintCourseConflictError,
  );
});
