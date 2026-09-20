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
const blueprintEditNumber = "1";
const classification = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
};
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
        question_revision_tuple: publishedQuestion.questionRevisionTuple,
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
      blueprint_module_id: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
      label: "Week one",
      assessments: [
        {
          blueprint_assessment_id: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
          content: {
            ...contentInput(),
            entries: [
              {
                kind: "fixed",
                question: {
                  question_revision_tuple: publishedQuestion.questionRevisionTuple,
                  question_library: {
                    summary: publishedQuestion,
                    disciplineName: "Biology",
                    disciplineIsRetired: false,
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
    classification,
    id: "BP7K3M2QAF",
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    availability: "private",
    blueprint_edit_number: blueprintEditNumber,
    current_revision_tuple: { blueprintCourseId: "BP7K3M2QAF", revisionNumber: revision },
    fork_source_tuple: null,
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
    blueprint_assessment_id: `assessment-${index + 1}`,
  }));
  return { ...blueprint(), modules: [{ ...blueprintModule, assessments }] };
}

function creationInput() {
  return {
    classification,
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    modules: [{ label: "Week one", assessments: [contentInput()] }],
  };
}

function canonicalExchange() {
  return {
    metadata: {
      short_name: "Biochemistry",
      long_name: "Biochemistry sequence",
      classification,
    },
    modules: [{ label: "Week one", assessments: [contentInput()] }],
  };
}

function replacementInput() {
  return {
    modules: [
      {
        choice: {
          kind: "retained",
          blueprint_module_id: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
        },
        label: "Week one",
        assessments: [
          {
            choice: {
              kind: "retained",
              blueprint_assessment_id: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
            },
            content: contentInput(),
          },
        ],
      },
    ],
  };
}

test("Course classification metadata updates send explicit hierarchy with independent strong validators", async () => {
  const requests = [];
  const nextEtag = "2";
  const selected = { ...classification, tags: ["review"] };
  const client = createHttpApiClient({
    fetch: async (path, options) => {
      requests.push({ path, options });
      const body = path.includes("course-blueprints")
        ? {
            short_name: "Biochemistry",
            long_name: "Biochemistry sequence",
            availability: "private",
            classification: selected,
            blueprint_edit_number: nextEtag,
          }
        : { classification: selected, courseEditNumber: nextEtag, changed: true };
      return noStoreJson(body, `"${nextEtag}"`);
    },
  });
  const blueprintReceipt = await client.updateBlueprintCourseClassification(
    "BP7K3M2QAF",
    selected,
    blueprintEditNumber,
  );
  const instanceReceipt = await client.updateCourseInstanceClassification(
    "CI6F2R8TA0",
    selected,
    blueprintEditNumber,
  );
  assert.equal(blueprintReceipt.metadata.blueprint_edit_number, nextEtag);
  assert.equal(instanceReceipt.courseEditNumber, nextEtag);
  assert.equal(instanceReceipt.changed, true);
  assert.deepEqual(
    requests.map(({ path }) => path),
    [
      "/api/course-blueprints/BP7K3M2QAF/classification",
      "/api/course-instances/CI6F2R8TA0/classification",
    ],
  );
  for (const { options } of requests) {
    assert.equal(options.method, "PUT");
    assert.equal(options.headers["if-match"], `"${blueprintEditNumber}"`);
    assert.deepEqual(JSON.parse(options.body), selected);
  }
});

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
  assert.equal(decodeBlueprintCourseView(blueprint()).current_revision_tuple.revisionNumber, "3");
  assert.equal(decodeBlueprintCourseView(blueprint()).blueprint_edit_number, blueprintEditNumber);
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
  const numberUnderTuple = structuredClone(blueprint());
  numberUnderTuple.current_revision_tuple = "1";
  assert.throws(() => decodeBlueprintCourseView(numberUnderTuple), DecodeError);
});

// Protect optional discovery request shape; failure means repair request encoding, not normal discovery.
test("Blueprint discovery encodes only active optional discovery filters", async () => {
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
  await client.listBlueprintCourses(undefined, 50, false, "Genetics", true, true);
  await client.listBlueprintCourses(undefined, 50, false, "Genetics", true, false);
  assert.equal(paths[0].searchParams.has("includeArchived"), false);
  assert.equal(paths[1].searchParams.has("includeArchived"), false);
  assert.equal(paths[2].searchParams.get("includeArchived"), "true");
  assert.equal(paths[2].searchParams.get("pageSize"), "50");
  assert.equal(paths[3].searchParams.get("query"), "Biochem & %_+?");
  assert.equal(paths[3].searchParams.get("publicOnly"), "true");
  assert.equal(paths[3].searchParams.has("includeArchived"), false);
  assert.equal(paths[4].searchParams.get("promotedOnly"), "true");
  assert.equal(paths[5].searchParams.has("promotedOnly"), false);
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
  assert.equal(paths.length, 6);
});

test("Blueprint classification discovery encodes identity filters and rejects incomplete chains before fetching", async () => {
  const paths = [];
  const client = createHttpApiClient({
    fetch: async (input) => {
      paths.push(new URL(input.toString(), "https://ple.example"));
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  const empty = {
    disciplineUuid: null,
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
    crossDiscipline: false,
  };
  const filters = {
    disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
    subjectUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
    topicUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6f",
    subtopicUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d70",
    crossDiscipline: true,
  };
  await client.listBlueprintCourses(undefined, 50, false, "Biochem & %_+?", true, true, filters);
  assert.deepEqual(Object.fromEntries(paths[0].searchParams), {
    pageSize: "50",
    query: "Biochem & %_+?",
    publicOnly: "true",
    promotedOnly: "true",
    ...filters,
    crossDiscipline: "true",
  });
  await client.listBlueprintCourses(undefined, 50, false, "", true, false, empty);
  assert.deepEqual(Object.fromEntries(paths[1].searchParams), {
    pageSize: "50",
    query: "",
    publicOnly: "true",
  });
  for (const invalid of [
    { ...filters, disciplineUuid: "bad" },
    { ...filters, disciplineUuid: null },
    { ...filters, subjectUuid: null },
    { ...filters, topicUuid: null },
    { ...empty, crossDiscipline: true },
    { ...empty, crossDiscipline: "true" },
  ]) {
    await assert.rejects(
      client.listBlueprintCourses(undefined, 50, false, "", true, false, invalid),
    );
  }
  assert.equal(paths.length, 2);
});

test("B1 client sends Revision and metadata validators to their separate routes", async () => {
  const requests = [];
  const renamedMetadata = {
    classification,
    short_name: "Biochemistry",
    long_name: "Biochemistry sequence",
    availability: "private",
    blueprint_edit_number: "1",
  };
  const publishedMetadata = {
    ...renamedMetadata,
    availability: "public",
    blueprint_edit_number: "1",
  };
  const archivedMetadata = {
    ...publishedMetadata,
    availability: "archived",
    blueprint_edit_number: "1",
  };
  const restoredMetadata = {
    ...archivedMetadata,
    availability: "public",
    blueprint_edit_number: "1",
  };
  const privateMetadata = {
    ...restoredMetadata,
    availability: "private",
    blueprint_edit_number: "1",
  };
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/metadata"))
        return noStoreJson(renamedMetadata, `"${renamedMetadata.blueprint_edit_number}"`);
      if (path.endsWith("/publish"))
        return noStoreJson(publishedMetadata, `"${publishedMetadata.blueprint_edit_number}"`);
      if (path.endsWith("/archive"))
        return noStoreJson(archivedMetadata, `"${archivedMetadata.blueprint_edit_number}"`);
      if (path.endsWith("/restore"))
        return noStoreJson(restoredMetadata, `"${restoredMetadata.blueprint_edit_number}"`);
      if (path.endsWith("/return-to-private"))
        return noStoreJson(privateMetadata, `"${privateMetadata.blueprint_edit_number}"`);
      if (path.endsWith("/revisions/3"))
        return noStoreJson({
          blueprintRevisionTuple: { blueprintCourseId: "BP7K3M2QAF", revisionNumber: "3" },
          modules: modules(),
        });
      if (request.method === "GET" && path.endsWith("BP7K3M2QAF"))
        return noStoreJson(blueprint(), '"3"');
      if (request.method === "POST" && path.endsWith("course-blueprints"))
        return noStoreJson(blueprint("1"), '"1"', 201);
      if (request.method === "PUT" && path.endsWith("BP7K3M2QAF"))
        return noStoreJson({ blueprintCourse: blueprint("4"), changed: true }, '"4"');
      return noStoreJson({ items: [], nextCursor: null });
    },
  });
  const current = await client.getBlueprintCourse("BP7K3M2QAF");
  await client.createBlueprintCourse(creationInput(), "create-7");
  const saved = await client.saveBlueprintCourse(
    "BP7K3M2QAF",
    replacementInput(),
    current.blueprintCourse.current_revision_tuple.revisionNumber,
    "save-7",
  );
  const renamed = await client.renameBlueprintCourse(
    "BP7K3M2QAF",
    { short_name: "Biochemistry", long_name: "Biochemistry sequence" },
    blueprintEditNumber,
  );
  const published = await client.publishBlueprintCourse("BP7K3M2QAF", renamed.blueprintEditNumber);
  const archived = await client.archiveBlueprintCourse(
    "BP7K3M2QAF",
    "Biochemistry sequence",
    published.blueprintEditNumber,
  );
  const restored = await client.restoreBlueprintCourse("BP7K3M2QAF", archived.blueprintEditNumber);
  const returned = await client.returnBlueprintCourseToPrivate(
    "BP7K3M2QAF",
    restored.blueprintEditNumber,
  );
  const revision = await client.getBlueprintRevision("BP7K3M2QAF", "3");
  assert.equal(current.blueprintCourse.modules[0].assessments[0].content.assessment_type, "exam");
  assert.equal(saved.changed, true);
  assert.equal(saved.blueprintCourse.modules[0].assessments[0].content.assessment_type, "exam");
  assert.equal(saved.blueprintCourse.current_revision_tuple.revisionNumber, "4");
  assert.equal(returned.metadata.availability, "private");
  assert.equal(revision.blueprintRevisionTuple.revisionNumber, "3");
  const save = requests.find(
    (request) => request.method === "PUT" && request.url.endsWith("BP7K3M2QAF"),
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
        request.url.endsWith("/api/course-blueprints/BP7K3M2QAF/publish"),
    ),
  );
  assert.ok(
    requests.some(
      (request) =>
        request.method === "POST" &&
        request.url.endsWith("/api/course-blueprints/BP7K3M2QAF/return-to-private"),
    ),
  );
  await assert.rejects(
    client.saveBlueprintCourse("BP7K3M2QAF", replacementInput(), "07", "save-7"),
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

test("Canonical Blueprint exchange uses the one strict reusable-structure transport contract", async () => {
  const requests = [];
  const exchange = canonicalExchange();
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      if (request.method === "GET") return noStoreJson(exchange);
      return noStoreJson({ ...blueprint("1"), id: "BP7K3M2RAW" }, '"1"', 201);
    },
  });

  assert.deepEqual(await client.exportBlueprintCourse("BP7K3M2QAF"), exchange);
  const imported = await client.importBlueprintCourse(exchange, "import-7");
  assert.equal(imported.blueprintCourse.id, "BP7K3M2RAW");
  assert.equal(imported.blueprintCourse.availability, "private");
  assert.deepEqual(
    requests.map((request) => [request.method, new URL(request.url).pathname]),
    [
      ["GET", "/api/course-blueprints/BP7K3M2QAF/export"],
      ["POST", "/api/course-blueprints/import"],
    ],
  );
  assert.equal(requests[1].headers.get("idempotency-key"), "import-7");
  assert.deepEqual(await requests[1].json(), exchange);

  await assert.rejects(
    client.importBlueprintCourse({ ...exchange, unexpected: true }, "import-8"),
    DecodeError,
  );
});

test("Canonical Blueprint import rejects a receipt that is not a new actor-owned Private root", async () => {
  const exchange = canonicalExchange();
  const malformedReceipts = [
    { availability: "public" },
    { read_access: "active_instructor" },
    { fork_source_tuple: { blueprintCourseId: "BP7K3M2QAF", revisionNumber: "1" } },
    { current_revision_tuple: { blueprintCourseId: "BP7K3M2RAW", revisionNumber: "2" } },
  ];

  for (const changes of malformedReceipts) {
    const receipt = { ...blueprint("1"), id: "BP7K3M2RAW", ...changes };
    const client = createHttpApiClient({
      fetch: () =>
        Promise.resolve(
          noStoreJson(receipt, `"${receipt.current_revision_tuple.revisionNumber}"`, 201),
        ),
    });
    await assert.rejects(
      client.importBlueprintCourse(exchange, crypto.randomUUID()),
      ApiProtocolError,
    );
  }
});

test("B1 client gives a typed conflict for a stale Blueprint Revision Save", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    client.saveBlueprintCourse("BP7K3M2QAF", replacementInput(), "3", "save-7"),
    BlueprintCourseConflictError,
  );
});

test("B1 client accepts a canonical Save no-op at the current Blueprint Revision", async () => {
  const client = createHttpApiClient({
    fetch: () =>
      Promise.resolve(noStoreJson({ blueprintCourse: blueprint("3"), changed: false }, '"3"')),
  });
  const saved = await client.saveBlueprintCourse(
    "BP7K3M2QAF",
    replacementInput(),
    "3",
    "save-no-op",
  );
  assert.equal(saved.changed, false);
  assert.equal(saved.blueprintCourse.current_revision_tuple.revisionNumber, "3");
});

test("B1 Blueprint aggregates have a dedicated bounded response budget", async () => {
  const largeBlueprint = blueprintWithAssessments(85);
  const largeBlueprintJson = JSON.stringify(largeBlueprint);
  assert.ok(largeBlueprintJson.length > FOUR_MIB);
  assert.ok(largeBlueprintJson.length <= SIXTEEN_MIB);
  const blueprintClient = createHttpApiClient({
    fetch: () => Promise.resolve(noStoreJson(largeBlueprint, '"3"')),
  });
  const loaded = await blueprintClient.getBlueprintCourse("BP7K3M2QAF");
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
  await assert.rejects(client.getBlueprintCourse("BP7K3M2QAF"), ApiProtocolError);
});

test("B1 metadata decoder rejects non-opaque validators", () => {
  assert.throws(
    () =>
      decodeBlueprintMetadataState({
        classification,
        short_name: "Short",
        long_name: "Long",
        availability: "available",
        blueprint_edit_number: "7",
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
        classification,
        short_name: "Short",
        long_name: "Long",
        availability,
        blueprint_edit_number: blueprintEditNumber,
      }).availability,
      availability,
    );
  }
  assert.throws(
    () =>
      decodeBlueprintMetadataState({
        classification,
        short_name: "Short",
        long_name: "Long",
        availability: "available",
        blueprint_edit_number: blueprintEditNumber,
      }),
    DecodeError,
  );
});

test("Blueprint Course decoder rejects an unexpected nested Assessment policy field", () => {
  const invalid = blueprint();
  invalid.modules[0].assessments[0].content.defaults.activity_rules.unexpectedPolicyField = true;

  assert.throws(() => decodeBlueprintCourseView(invalid), DecodeError);
});
