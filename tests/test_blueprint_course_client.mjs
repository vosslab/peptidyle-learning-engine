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

const { scope: _scope, ...questionSummary } = publishedQuestionFixture.publishedQuestion;
const publishedQuestion = { ...questionSummary, questionFormat: "pleQuestionJson" };
const metadataEtag = "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d";
const FOUR_MIB = 4 * 1_024 * 1_024;
const SIXTEEN_MIB = 16 * 1_024 * 1_024;
function contentInput() {
  return {
    assessment_type: "exam",
    title: "Peptide fundamentals",
    instructions: "Use your course notes.",
    entries: [
      {
        kind: "fixed",
        published_question: publishedQuestion.latestQuestionRevision,
        points_possible: "2",
        scoring_rule: "normal",
        question_attempt_limit: { maxAttempts: null },
        question_attempt_time_limit: { kind: "unlimited" },
      },
    ],
    defaults: {
      assessment_attempt_time_limit_seconds: null,
      assessment_attempt_limit: 2,
      late_work_rule: "accept",
      activity_rules: {
        questionVariationRule: "newVariation",
        assessmentQuestionOrderRule: "authoredOrder",
      },
      student_feedback_release_rule: {
        score: "after_submit",
        per_item_correctness: "after_submit",
        submitted_response: "after_submit",
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
      assessments: [
        {
          blueprint_assessment_reference: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
          content: {
            ...contentInput(),
            entries: [
              {
                kind: "fixed",
                question: {
                  reference: publishedQuestion.latestQuestionRevision,
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
    reference: "BP7K3M2Q",
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    availability: "private",
    metadata_etag: metadataEtag,
    current_revision: { reference: "BP7K3M2Q", revision },
    fork_source: null,
    read_access: "blueprint_course_owner",
    modules: modules(),
  };
}

function blueprintWithAssessments(assessmentCount) {
  const blueprintModule = modules()[0];
  const assessment = blueprintModule.assessments[0];
  assessment.content.instructions = "x".repeat(50_000);
  const assessments = Array.from({ length: assessmentCount }, (_, index) => ({
    ...structuredClone(assessment),
    blueprint_assessment_reference: `assessment-${index + 1}`,
  }));
  return { ...blueprint(), modules: [{ ...blueprintModule, assessments }] };
}

function creationInput() {
  return {
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    modules: [{ label: "Week one", assessments: [contentInput()] }],
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
        assessments: [
          {
            choice: {
              kind: "retained",
              blueprint_assessment_reference: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
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
  const missingType = structuredClone(blueprint());
  delete missingType.modules[0].assessments[0].content.assessment_type;
  assert.throws(() => decodeBlueprintCourseView(missingType), DecodeError);
  const unknownType = structuredClone(blueprint());
  unknownType.modules[0].assessments[0].content.assessment_type = "project";
  assert.throws(() => decodeBlueprintCourseView(unknownType), DecodeError);
  const hostile = structuredClone(blueprint());
  hostile.modules[0].assessments[0].content.entries[0].question.answerKey = "secret";
  assert.throws(() => decodeBlueprintCourseView(hostile), DecodeError);
  const retired = structuredClone(blueprint());
  retired.draft = { edit_number: "7", modules: [] };
  assert.throws(() => decodeBlueprintCourseView(retired), DecodeError);
});

// Protect the explicit history choice; failure means repair request encoding, not normal discovery.
test("Blueprint discovery requests Archived history only when explicitly included", async () => {
  const paths = [];
  const client = createHttpApiClient({
    fetch: async (input) => {
      paths.push(new URL(input.toString(), "https://ple.example"));
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  await client.listBlueprintCourses();
  await client.listBlueprintCourses(undefined, 50, false);
  await client.listBlueprintCourses(undefined, 50, true);
  await client.listBlueprintCourses(undefined, 50, false, "Biochem & %_+?", true);
  assert.equal(paths[0].searchParams.has("includeArchived"), false);
  assert.equal(paths[1].searchParams.has("includeArchived"), false);
  assert.equal(paths[2].searchParams.get("includeArchived"), "true");
  assert.equal(paths[2].searchParams.get("pageSize"), "50");
  assert.equal(paths[3].searchParams.get("query"), "Biochem & %_+?");
  assert.equal(paths[3].searchParams.get("publicOnly"), "true");
  assert.equal(paths[3].searchParams.has("includeArchived"), false);
  assert.equal(paths[0].searchParams.has("publicOnly"), false);
  await assert.rejects(
    client.listBlueprintCourses(undefined, 50, true, "", true),
    ApiProtocolError,
  );
  await assert.rejects(
    client.listBlueprintCourses(undefined, 50, false, "x".repeat(257), true),
    ApiProtocolError,
  );
  await assert.rejects(
    client.listBlueprintCourses(undefined, 50, false, "bad\u0000query", true),
    ApiProtocolError,
  );
  assert.equal(paths.length, 4);
});

test("B1 client sends Revision and metadata validators to their separate routes", async () => {
  const requests = [];
  const renamedMetadata = {
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    availability: "private",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6f",
  };
  const publishedMetadata = {
    ...renamedMetadata,
    availability: "public",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d70",
  };
  const archivedMetadata = {
    ...publishedMetadata,
    availability: "archived",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d71",
  };
  const restoredMetadata = {
    ...archivedMetadata,
    availability: "public",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d72",
  };
  const privateMetadata = {
    ...restoredMetadata,
    availability: "private",
    metadata_etag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d73",
  };
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/metadata"))
        return noStoreJson(renamedMetadata, `"${renamedMetadata.metadata_etag}"`);
      if (path.endsWith("/publish"))
        return noStoreJson(publishedMetadata, `"${publishedMetadata.metadata_etag}"`);
      if (path.endsWith("/archive"))
        return noStoreJson(archivedMetadata, `"${archivedMetadata.metadata_etag}"`);
      if (path.endsWith("/restore"))
        return noStoreJson(restoredMetadata, `"${restoredMetadata.metadata_etag}"`);
      if (path.endsWith("/return-to-private"))
        return noStoreJson(privateMetadata, `"${privateMetadata.metadata_etag}"`);
      if (path.endsWith("/revisions/3"))
        return noStoreJson({
          blueprintRevision: { reference: "BP7K3M2Q", revision: "3" },
          modules: modules(),
        });
      if (request.method === "GET" && path.endsWith("BP7K3M2Q"))
        return noStoreJson(blueprint(), '"3"');
      if (request.method === "POST" && path.endsWith("course-blueprints"))
        return noStoreJson(blueprint("1"), '"1"', 201);
      if (request.method === "PUT" && path.endsWith("BP7K3M2Q"))
        return noStoreJson({ blueprintCourse: blueprint("4"), changed: true }, '"4"');
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  const current = await client.getBlueprintCourse("BP7K3M2Q");
  await client.createBlueprintCourse(creationInput(), "create-7");
  const saved = await client.saveBlueprintCourse(
    "BP7K3M2Q",
    replacementInput(),
    current.revisionEtag,
    "save-7",
  );
  const renamed = await client.renameBlueprintCourse(
    "BP7K3M2Q",
    { short_name: "Biochemistry", long_name: "Biochemistry sequence" },
    `"${metadataEtag}"`,
  );
  const published = await client.publishBlueprintCourse("BP7K3M2Q", renamed.metadataEtag);
  const archived = await client.archiveBlueprintCourse(
    "BP7K3M2Q",
    "Biochemistry sequence",
    published.metadataEtag,
  );
  const restored = await client.restoreBlueprintCourse("BP7K3M2Q", archived.metadataEtag);
  const returned = await client.returnBlueprintCourseToPrivate("BP7K3M2Q", restored.metadataEtag);
  const revision = await client.getBlueprintRevision("BP7K3M2Q", "3");
  assert.equal(current.blueprintCourse.modules[0].assessments[0].content.assessment_type, "exam");
  assert.equal(saved.changed, true);
  assert.equal(saved.blueprintCourse.modules[0].assessments[0].content.assessment_type, "exam");
  assert.equal(saved.revisionEtag, '"4"');
  assert.equal(returned.metadata.availability, "private");
  assert.equal(revision.blueprintRevision.revision, "3");
  const save = requests.find(
    (request) => request.method === "PUT" && request.url.endsWith("BP7K3M2Q"),
  );
  const creation = requests.find(
    (request) => request.method === "POST" && request.url.endsWith("course-blueprints"),
  );
  assert.ok(creation);
  assert.equal((await creation.json()).modules[0].assessments[0].assessment_type, "exam");
  assert.ok(save);
  assert.equal(save?.headers.get("if-match"), '"3"');
  assert.equal(save?.headers.get("idempotency-key"), "save-7");
  const savedRequest = await save.json();
  assert.equal(savedRequest.modules[0].assessments[0].content.assessment_type, "exam");
  assert.equal(savedRequest.modules[0].assessments[0].content.defaults.assessment_attempt_limit, 2);
  assert.ok(
    requests.some(
      (request) =>
        request.method === "POST" &&
        request.url.endsWith("/api/course-blueprints/BP7K3M2Q/publish"),
    ),
  );
  assert.ok(
    requests.some(
      (request) =>
        request.method === "POST" &&
        request.url.endsWith("/api/course-blueprints/BP7K3M2Q/return-to-private"),
    ),
  );
  await assert.rejects(
    client.saveBlueprintCourse("BP7K3M2Q", replacementInput(), '"07"', "save-7"),
    ApiProtocolError,
  );
  await assert.rejects(
    client.createBlueprintCourse({ ...creationInput(), forged: true }, "create-8"),
    DecodeError,
  );
  const missingTypeCreation = creationInput();
  delete missingTypeCreation.modules[0].assessments[0].assessment_type;
  await assert.rejects(
    client.createBlueprintCourse(missingTypeCreation, "create-without-type"),
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
    client.saveBlueprintCourse("BP7K3M2Q", replacementInput(), '"3"', "save-7"),
    BlueprintCourseConflictError,
  );
});

test("B1 client accepts a canonical Save no-op at the current Blueprint Revision", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(noStoreJson({ blueprintCourse: blueprint("3"), changed: false }, '"3"')),
  });
  const saved = await client.saveBlueprintCourse(
    "BP7K3M2Q",
    replacementInput(),
    '"3"',
    "save-no-op",
  );
  assert.equal(saved.changed, false);
  assert.equal(saved.revisionEtag, '"3"');
});

test("B1 Blueprint aggregates have a dedicated bounded response budget", async () => {
  const largeBlueprint = blueprintWithAssessments(85);
  const largeBlueprintJson = JSON.stringify(largeBlueprint);
  assert.ok(largeBlueprintJson.length > FOUR_MIB);
  assert.ok(largeBlueprintJson.length <= SIXTEEN_MIB);
  const blueprintClient = createHttpApiClient({
    fetch: () => Promise.resolve(noStoreJson(largeBlueprint, '"3"')),
  });
  const loaded = await blueprintClient.getBlueprintCourse("BP7K3M2Q");
  assert.equal(loaded.blueprintCourse.modules[0].assessments.length, 85);

  const ordinaryClient = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(`"${"x".repeat(FOUR_MIB)}"`, {
          headers: { "cache-control": "no-store", "content-type": "application/json" },
        }),
      ),
  });
  await assert.rejects(ordinaryClient.getAccountSettings(), ApiProtocolError);
});

test("B1 Blueprint aggregates reject responses beyond their dedicated budget", async () => {
  const oversizedBlueprint = blueprintWithAssessments(336);
  assert.ok(JSON.stringify(oversizedBlueprint).length > SIXTEEN_MIB);
  const client = createHttpApiClient({
    fetch: () => Promise.resolve(noStoreJson(oversizedBlueprint, '"3"')),
  });
  await assert.rejects(client.getBlueprintCourse("BP7K3M2Q"), ApiProtocolError);
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

// Regression: accepting the pre-lifecycle `available` alias would collapse
// Private and Public product behavior. Failure means restore the generated
// Private|Public|Archived decoder contract before release.
test("Blueprint lifecycle metadata accepts only the generated public states", () => {
  for (const availability of ["private", "public", "archived"]) {
    assert.equal(
      decodeBlueprintMetadataState({
        short_name: "Short",
        long_name: "Long",
        availability,
        metadata_etag: metadataEtag,
      }).availability,
      availability,
    );
  }
  assert.throws(
    () =>
      decodeBlueprintMetadataState({
        short_name: "Short",
        long_name: "Long",
        availability: "available",
        metadata_etag: metadataEtag,
      }),
    DecodeError,
  );
});

test("Blueprint Course decoder rejects an unexpected nested Assessment policy field", () => {
  const invalid = blueprint();
  invalid.modules[0].assessments[0].content.defaults.activity_rules.unexpectedPolicyField = true;

  assert.throws(() => decodeBlueprintCourseView(invalid), DecodeError);
});
