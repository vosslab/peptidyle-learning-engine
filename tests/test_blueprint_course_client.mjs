import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeBlueprintCourseView,
  decodeBlueprintMetadataState,
} from "../src/api/decoders/blueprint_course.ts";
import {
  ApiProtocolError,
  BlueprintCourseConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const { scope: _scope, ...publishedQuestion } = publishedQuestionFixture.publishedQuestion;
const metadataEtag = "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d";
const FOUR_MIB = 4 * 1_024 * 1_024;
const SIXTEEN_MIB = 16 * 1_024 * 1_024;

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

function blueprint(revision = "3") {
  return {
    reference: "BP-7",
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    availability: "available",
    metadata_etag: metadataEtag,
    current_revision: { reference: "BP-7", revision },
    read_access: "blueprint_course_owner",
    modules: modules(),
  };
}

function blueprintWithAssignments(assignmentCount) {
  const blueprintModule = modules()[0];
  const assignment = blueprintModule.assignments[0];
  assignment.content.instructions = "x".repeat(50_000);
  const assignments = Array.from({ length: assignmentCount }, (_, index) => ({
    ...structuredClone(assignment),
    blueprint_assignment_reference: `assignment-${index + 1}`,
  }));
  return { ...blueprint(), modules: [{ ...blueprintModule, assignments }] };
}

function creationInput() {
  return {
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    modules: [{ label: "Week one", assignments: [contentInput()] }],
  };
}

function replacementInput() {
  return {
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

test("B1 Blueprint Course decoder exposes one current Revision and opaque metadata", () => {
  assert.equal(decodeBlueprintCourseView(blueprint()).current_revision.revision, "3");
  assert.equal(decodeBlueprintCourseView(blueprint()).metadata_etag, metadataEtag);
  const hostile = structuredClone(blueprint());
  hostile.modules[0].assignments[0].content.entries[0].question.answerKey = "secret";
  assert.throws(() => decodeBlueprintCourseView(hostile), DecodeError);
  const retired = structuredClone(blueprint());
  retired.draft = { edit_number: "7", modules: [] };
  assert.throws(() => decodeBlueprintCourseView(retired), DecodeError);
});

test("B1 client sends Revision and metadata validators to their separate routes", async () => {
  const requests = [];
  const metadata = {
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    availability: "archived",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6f",
  };
  const restoredMetadata = {
    ...metadata,
    availability: "available",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d70",
  };
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/metadata")) return noStoreJson(metadata, `"${metadata.metadata_etag}"`);
      if (path.endsWith("/archive")) return noStoreJson(metadata, `"${metadata.metadata_etag}"`);
      if (path.endsWith("/restore"))
        return noStoreJson(restoredMetadata, `"${restoredMetadata.metadata_etag}"`);
      if (path.endsWith("/revisions/3"))
        return noStoreJson({
          blueprintRevision: { reference: "BP-7", revision: "3" },
          modules: modules(),
        });
      if (request.method === "GET" && path.endsWith("BP-7")) return noStoreJson(blueprint(), '"3"');
      if (request.method === "POST" && path.endsWith("course-blueprints"))
        return noStoreJson(blueprint("1"), '"1"', 201);
      if (request.method === "PUT" && path.endsWith("BP-7"))
        return noStoreJson({ blueprintCourse: blueprint("4"), changed: true }, '"4"');
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  const current = await client.getBlueprintCourse("BP-7");
  await client.createBlueprintCourse(creationInput(), "create-7");
  const saved = await client.saveBlueprintCourse(
    "BP-7",
    replacementInput(),
    current.revisionEtag,
    "save-7",
  );
  const renamed = await client.renameBlueprintCourse(
    "BP-7",
    { short_name: "Biochemistry", long_name: "Biochemistry sequence" },
    `"${metadataEtag}"`,
  );
  const archived = await client.archiveBlueprintCourse(
    "BP-7",
    "Biochemistry sequence",
    renamed.metadataEtag,
  );
  const restored = await client.restoreBlueprintCourse("BP-7", archived.metadataEtag);
  const revision = await client.getBlueprintRevision("BP-7", "3");
  assert.equal(saved.changed, true);
  assert.equal(saved.revisionEtag, '"4"');
  assert.equal(restored.metadata.availability, "available");
  assert.equal(revision.blueprintRevision.revision, "3");
  const save = requests.find((request) => request.method === "PUT" && request.url.endsWith("BP-7"));
  assert.equal(save?.headers.get("if-match"), '"3"');
  assert.equal(save?.headers.get("idempotency-key"), "save-7");
  await assert.rejects(
    client.saveBlueprintCourse("BP-7", replacementInput(), '"07"', "save-7"),
    ApiProtocolError,
  );
  await assert.rejects(
    client.createBlueprintCourse({ ...creationInput(), forged: true }, "create-8"),
    DecodeError,
  );
});

test("B1 client gives a typed conflict for a stale Blueprint Revision Save", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    client.saveBlueprintCourse("BP-7", replacementInput(), '"3"', "save-7"),
    BlueprintCourseConflictError,
  );
});

test("B1 client accepts a canonical Save no-op at the current Blueprint Revision", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(noStoreJson({ blueprintCourse: blueprint("3"), changed: false }, '"3"')),
  });
  const saved = await client.saveBlueprintCourse("BP-7", replacementInput(), '"3"', "save-no-op");
  assert.equal(saved.changed, false);
  assert.equal(saved.revisionEtag, '"3"');
});

test("B1 Blueprint aggregates have a dedicated bounded response budget", async () => {
  const largeBlueprint = blueprintWithAssignments(85);
  const largeBlueprintJson = JSON.stringify(largeBlueprint);
  assert.ok(largeBlueprintJson.length > FOUR_MIB);
  assert.ok(largeBlueprintJson.length <= SIXTEEN_MIB);
  const blueprintClient = createHttpApiClient({
    fetch: () => Promise.resolve(noStoreJson(largeBlueprint, '"3"')),
  });
  const loaded = await blueprintClient.getBlueprintCourse("BP-7");
  assert.equal(loaded.blueprintCourse.modules[0].assignments.length, 85);

  const ordinaryClient = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(`"${"x".repeat(FOUR_MIB)}"`, {
          headers: { "cache-control": "no-store", "content-type": "application/json" },
        }),
      ),
  });
  await assert.rejects(ordinaryClient.getInstructorProfile(), ApiProtocolError);
});

test("B1 Blueprint aggregates reject responses beyond their dedicated budget", async () => {
  const oversizedBlueprint = blueprintWithAssignments(336);
  assert.ok(JSON.stringify(oversizedBlueprint).length > SIXTEEN_MIB);
  const client = createHttpApiClient({
    fetch: () => Promise.resolve(noStoreJson(oversizedBlueprint, '"3"')),
  });
  await assert.rejects(client.getBlueprintCourse("BP-7"), ApiProtocolError);
});

test("B1 metadata decoder rejects non-opaque validators", () => {
  assert.throws(
    () =>
      decodeBlueprintMetadataState({
        short_name: "Short",
        long_name: "Long",
        availability: "available",
        metadata_etag: "7",
      }),
    DecodeError,
  );
});
