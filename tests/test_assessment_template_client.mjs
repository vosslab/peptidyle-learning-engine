import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const templateId = "00000000-0000-0000-0000-000000000123";

function template(editNumber = "1") {
  return {
    id: templateId,
    name: "Timed genetics quiz",
    assessmentType: "quiz",
    settings: {
      instructions: "Show your reasoning.",
      assessmentAttemptTimeLimitSeconds: 1800,
      attemptLimit: 2,
      lateWorkRule: "reject",
      activityRules: {
        assessmentAttemptGradeRule: "highest",
        questionPoolReuseRule: "reuseSelection",
        questionVariationRule: "reuseVariation",
        assessmentAttemptResumeRule: "resumable",
        assessmentQuestionDisplayRule: "allQuestions",
        assessmentNavigationRule: "freeNavigation",
        assessmentQuestionOrderRule: "authoredOrder",
      },
      studentFeedbackReleaseRule: {
        score: "after_submit",
        per_item_correctness: "after_due",
        submitted_response: "after_submit",
        question_feedback: "never",
        question_answer: "never",
        question_answer_explanation: "never",
        class_statistics: "after_close",
      },
    },
    editNumber,
  };
}

function workspace(editNumber = "3") {
  return {
    reference: "A8H4N6P",
    editNumber,
    status: "unreleased",
    origin: { kind: "direct" },
    assessmentType: "quiz",
    title: "Genetics practice",
    instructions: "Show your reasoning.",
    dueAt: null,
    availableAt: null,
    closesAt: null,
    lateWorkRule: "reject",
    assessmentAttemptTimeLimitSeconds: null,
    attemptLimit: null,
    activityRules: {
      assessmentAttemptGradeRule: "highest",
      questionPoolReuseRule: "reuseSelection",
      questionVariationRule: "reuseVariation",
      assessmentAttemptResumeRule: "resumable",
      assessmentQuestionDisplayRule: "allQuestions",
      assessmentNavigationRule: "freeNavigation",
      assessmentQuestionOrderRule: "authoredOrder",
    },
    studentFeedbackReleaseRule: {
      score: "after_submit",
      per_item_correctness: "after_due",
      submitted_response: "after_submit",
      question_feedback: "never",
      question_answer: "never",
      question_answer_explanation: "never",
      class_statistics: "after_close",
    },
    displayTimeZone: "America/Chicago",
    entries: [],
    questions: [],
  };
}

function noStoreJson(value, status = 200, etag) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json; charset=utf-8",
      ...(etag === undefined ? {} : { etag }),
    },
  });
}

test("Assessment Template client uses the closed private CRUD transport and replacement ETags", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async (request) => {
    if (request.method === "GET" && new URL(request.url).pathname.endsWith(templateId)) {
      return noStoreJson(template("2"), 200, '"2"');
    }
    if (request.method === "GET") return noStoreJson({ items: [template()] });
    if (request.method === "POST") return noStoreJson(template(), 201, '"1"');
    return noStoreJson(template("2"), 200, '"2"');
  });
  const client = createHttpApiClient({ fetch: recordingFetch });
  const listed = await client.listAssessmentTemplates();
  const created = await client.createAssessmentTemplate({
    name: "Timed genetics quiz",
    assessmentType: "quiz",
  });
  const loaded = await client.getAssessmentTemplate(templateId);
  const saved = await client.saveAssessmentTemplate(
    templateId,
    {
      name: "Timed genetics quiz",
      assessmentType: "quiz",
      settings: template().settings,
    },
    '"2"',
  );

  assert.equal(listed[0].id, templateId);
  assert.equal(created.etag, '"1"');
  assert.equal(loaded.etag, '"2"');
  assert.equal(saved.etag, '"2"');
  assert.equal(new URL(requests[0].url).pathname, "/api/assessment-templates");
  assert.equal(requests[1].method, "POST");
  assert.deepEqual(JSON.parse(await requests[1].text()), {
    name: "Timed genetics quiz",
    assessmentType: "quiz",
  });
  assert.equal(new URL(requests[2].url).pathname, `/api/assessment-templates/${templateId}`);
  assert.equal(requests[3].method, "PUT");
  assert.equal(requests[3].headers.get("if-match"), '"2"');
  assert.deepEqual(Object.keys(JSON.parse(await requests[3].text())).sort(), [
    "assessmentType",
    "name",
    "settings",
  ]);
});

test("Assessment Template client rejects unexpected aggregate or settings fields and ETag disagreement", async () => {
  const unknownField = createHttpApiClient({
    fetch: async () => noStoreJson({ ...template(), ownerAccountId: "not-visible" }, 200, '"1"'),
  });
  await assert.rejects(unknownField.getAssessmentTemplate(templateId));

  const unknownSettingsField = createHttpApiClient({
    fetch: async () =>
      noStoreJson(
        { ...template(), settings: { ...template().settings, dueAt: "2026-09-15T12:00:00.000" } },
        200,
        '"1"',
      ),
  });
  await assert.rejects(unknownSettingsField.getAssessmentTemplate(templateId));

  const unknownActivityRule = createHttpApiClient({
    fetch: async () =>
      noStoreJson(
        {
          ...template(),
          settings: {
            ...template().settings,
            activityRules: {
              ...template().settings.activityRules,
              unexpectedPolicyField: true,
            },
          },
        },
        200,
        '"1"',
      ),
  });
  await assert.rejects(unknownActivityRule.getAssessmentTemplate(templateId));

  const disagreeingEtag = createHttpApiClient({
    fetch: async () => noStoreJson(template(), 200, '"2"'),
  });
  await assert.rejects(disagreeingEtag.getAssessmentTemplate(templateId), ApiProtocolError);

  const createdListResponse = createHttpApiClient({
    fetch: async () => noStoreJson({ items: [template()] }, 201),
  });
  await assert.rejects(createdListResponse.listAssessmentTemplates(), ApiProtocolError);

  const nameBoundary = createHttpApiClient({
    fetch: async () => noStoreJson(template(), 201, '"1"'),
  });
  await assert.rejects(
    nameBoundary.createAssessmentTemplate({
      name: "\u0085Timed genetics quiz",
      assessmentType: "quiz",
    }),
  );
  await assert.rejects(
    nameBoundary.createAssessmentTemplate({
      name: "Timed genetics quiz\u0085",
      assessmentType: "quiz",
    }),
  );
  await assert.doesNotReject(
    nameBoundary.createAssessmentTemplate({
      name: "\ufeffTimed genetics quiz",
      assessmentType: "quiz",
    }),
  );
});

test("Assessment Template client copies a Template through the closed Course Assessment boundary", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    noStoreJson(workspace(), 201, '"3"'),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });
  const created = await client.createAssessmentFromTemplate("CI7K3M2Q", {
    templateId,
    title: "Genetics practice",
  });

  assert.equal(created.workspace.reference, "A8H4N6P");
  assert.equal(created.etag, '"3"');
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/CI7K3M2Q/assessments/from-template",
  );
  assert.equal(requests[0].method, "POST");
  assert.deepEqual(JSON.parse(await requests[0].text()), {
    templateId,
    title: "Genetics practice",
  });

  const mismatchedEtag = createHttpApiClient({
    fetch: async () => noStoreJson(workspace(), 201, '"4"'),
  });
  await assert.rejects(
    mismatchedEtag.createAssessmentFromTemplate("CI7K3M2Q", {
      templateId,
      title: "Genetics practice",
    }),
    ApiProtocolError,
  );

  await assert.rejects(
    client.createAssessmentFromTemplate("CI7K3M2Q", {
      templateId: "not-a-uuid",
      title: "Genetics practice",
    }),
  );
  await assert.rejects(
    client.createAssessmentFromTemplate("CI7K3M2Q", {
      templateId,
      title: "   ",
    }),
  );
});
