import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const question = {
  ...publishedQuestionFixture.publishedQuestion,
  questionFormat: "pleQuestionJson",
};

function noStoreJson(value, etag) {
  return new Response(JSON.stringify(value), {
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
      questionRevisionTuple: { questionId: question.questionId, revisionNumber },
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
      return noStoreJson({ summary: question, viewerMayArchive: true }, '"5"');
    },
  });

  const lineage = await client.getQuestionLineage(question.questionId);
  const resolved = await client.resolveQuestion(question.questionId);
  const exact = await client.getQuestionRevision({
    questionId: question.questionId,
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
  assert.equal(exact.summary.questionRevisionTuple.revisionNumber, 2);
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
      return noStoreJson({ summary: question, viewerMayArchive: true }, '"05"');
    },
  });
  await assert.rejects(client.getQuestionLineage(question.questionId), ApiProtocolError);
  await assert.rejects(
    client.getQuestionRevision({ questionId: question.questionId, revisionNumber: 2 }),
    ApiProtocolError,
  );
});
