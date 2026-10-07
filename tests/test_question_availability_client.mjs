import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";

const question = {
  questionId: "7K3M-79QP",
  publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 1 },
  parentPublishedQuestionRevisionTuple: null,
  backend: "ple",
  questionFormat: "pleQuestionJson",
  questionType: "multipleChoice",
  capabilities: ["clientRendering", "serverGrading"],
  metadata: {
    questionTitle: "Peptide bond resonance",
    questionDescription: "Identify the bond with partial double-bond character.",
    tags: [],
    questionLicense: null,
    questionCitation: null,
    language: null,
  },
  authorship: { authors: [{ displayName: "Test Author", accountId: null }] },
  availability: { availability: "available" },
  publishedAt: 1_786_000_000_000,
  bloom: null,
};

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

function details(revisionNumber, disciplineIsRetired = false) {
  return {
    summary: {
      ...question,
      publishedQuestionRevisionTuple: { publishedQuestionId: question.questionId, revisionNumber },
    },
    disciplineName: "Biology",
    subjectName: "Genetics",
    disciplineIsRetired,
    prompt: { kind: "static", blocks: [] },
    responsePreview: { kind: "shortText" },
    evidence: { state: "unavailable" },
    usage: {
      summary: {
        globalCourseCount: 0,
        globalAssessmentCount: 0,
        ownCourseCount: 0,
        ownAssessmentCount: 0,
      },
      ownCourses: [],
      ownCoursesTruncated: false,
    },
  };
}

test("Question availability client keeps current lineage transitions and exact revisions distinct", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/revisions/2")) return noStoreJson(details(2, true), '"5"');
      if (path.endsWith("/archive")) {
        return noStoreJson(
          { availability: { availability: "archived" }, questionAvailabilityEditNumber: "6" },
          '"6"',
        );
      }
      if (path.endsWith("/restore")) {
        return noStoreJson(
          { availability: { availability: "available" }, questionAvailabilityEditNumber: "7" },
          '"7"',
        );
      }
      return noStoreJson(
        { summary: question, viewerMayArchive: true, viewerMayEditMetadata: true },
        '"5"',
      );
    },
  });

  const lineage = await client.getQuestionLineage(question.questionId);
  const resolved = await client.resolveQuestion(question.questionId);
  const exact = await client.getQuestionRevision({
    publishedQuestionId: question.questionId,
    revisionNumber: 2,
  });
  const archived = await client.archiveQuestion(
    question.questionId,
    question.metadata.questionTitle,
    "5",
  );
  const restored = await client.restoreQuestion(
    question.questionId,
    archived.questionAvailabilityEditNumber,
  );

  assert.equal(lineage.questionAvailabilityEditNumber, "5");
  assert.equal(lineage.viewerMayArchive, true);
  assert.equal(resolved.questionId, question.questionId);
  assert.equal(exact.summary.publishedQuestionRevisionTuple.revisionNumber, 2);
  assert.equal(exact.disciplineName, "Biology");
  assert.equal(exact.subjectName, "Genetics");
  assert.equal(exact.disciplineIsRetired, true);
  assert.equal(archived.availability, "archived");
  assert.equal(restored.availability, "available");
  assert.equal(requests[3].headers.get("if-match"), '"5"');
  assert.equal(requests[4].headers.get("if-match"), '"6"');
});

test("Question availability client rejects an ETag or exact revision identity mismatch", async () => {
  const client = createHttpApiClient({
    fetch: async (input) => {
      const path = new URL(input.toString(), "https://ple.example").pathname;
      if (path.endsWith("/revisions/2")) return noStoreJson(details(1), '"5"');
      return noStoreJson(
        { summary: question, viewerMayArchive: true, viewerMayEditMetadata: true },
        '"05"',
      );
    },
  });
  await assert.rejects(client.getQuestionLineage(question.questionId), ApiProtocolError);
  await assert.rejects(
    client.getQuestionRevision({ publishedQuestionId: question.questionId, revisionNumber: 2 }),
    ApiProtocolError,
  );
});

test("Owner correction opens an ordinary Draft from the current exact Revision", async () => {
  const requests = [];
  const parent = {
    publishedQuestionId: question.questionId,
    revisionNumber: 3,
  };
  const draftQuestion = "0190a0b0-c0d0-7e10-8a20-304050607080";
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return noStoreJson({ draftQuestion, publishedQuestionRevisionTuple: parent }, undefined, 201);
    },
  });

  const created = await client.createCorrectionDraft(parent);
  const request = requests[0];
  assert.equal(created.draftQuestion, draftQuestion);
  assert.deepEqual(created.publishedQuestionRevisionTuple, parent);
  assert.equal(request.method, "POST");
  assert.equal(
    new URL(request.url).pathname,
    `/api/questions/by-id/${encodeURIComponent(question.questionId)}/revisions/3/correction-draft`,
  );
  assert.equal(request.headers.get("content-type"), null);
  assert.equal(await request.text(), "");
});
