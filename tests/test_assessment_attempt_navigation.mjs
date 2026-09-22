import assert from "node:assert/strict";
import test from "node:test";

import { decodeLiveAssessmentAccess } from "../src/api/decoders/assessment_attempt_issuance.ts";
import {
  decodeStudentAssessmentAttemptContext,
  decodeStudentAssessmentAttemptPresentation,
  decodeStudentAssessmentAttemptProgress,
  decodeStudentAssessmentAttemptSubmissionResult,
} from "../src/api/decoders/assessment_attempt_navigation.ts";
import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { ROUTE_CONTRACT } from "../src/route_contract.ts";

function noStoreJson(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("author-content document URLs select only canonical Attempts and positive positions", () => {
  const client = createHttpApiClient({
    basePath: "/ple",
    fetch: () => assert.fail("an iframe URL must not fetch source into the parent"),
  });
  assert.equal(
    client.studentAuthorContentDocumentUrl("00000000-0000-0000-0000-00000000000c", 2),
    "/ple/api/assessment-attempts/00000000-0000-0000-0000-00000000000c/questions/2/author-content-document",
  );
  for (const attempt of [
    "R-0",
    "R-012",
    "R-2147483648",
    "00000000-0000-0000-0000-00000000000c/../context",
    "https://other.test",
  ]) {
    assert.throws(() => client.studentAuthorContentDocumentUrl(attempt, 2), ApiProtocolError);
  }
  for (const position of [0, -1, 1.5, NaN, Infinity, 2147483648]) {
    assert.throws(
      () =>
        client.studentAuthorContentDocumentUrl("00000000-0000-0000-0000-00000000000c", position),
      ApiProtocolError,
    );
  }
});

test("Assessment Attempt context retains one strict server expiry and display zone", () => {
  const context = {
    assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
    attemptNumber: 2,
    displayTimeZone: "America/Chicago",
    expiresAt: 1_768_507_200_000,
    timerRemainingMilliseconds: 15_000,
    course: {
      id: "CI7K3M2QAZ",
      shortName: "BCHM 301",
      longName: "Biochemistry 301: Proteins and Peptides",
      theme: "grass",
    },
    assessment: {
      id: "A7K3M2QAS",
      assessmentType: "practice_question_assignment",
      title: "Peptide structure practice",
    },
  };
  assert.deepEqual(decodeStudentAssessmentAttemptContext(context), context);
  assert.throws(() => decodeStudentAssessmentAttemptContext({ ...context, expiresAt: -1 }));
  assert.throws(() =>
    decodeStudentAssessmentAttemptContext({ ...context, displayTimeZone: "not/a-zone" }),
  );
});

test("Student Assessment Attempt progress rejects answer-bearing and extra fields", () => {
  const projection = {
    assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
    questionCount: 2,
    recommendedPosition: 2,
    positions: [
      { position: 1, responseState: "unanswered" },
      { position: 2, responseState: "saved" },
      { position: 3, responseState: "submitted" },
      { position: 4, responseState: "closed" },
    ],
  };
  projection.questionCount = 4;
  assert.deepEqual(decodeStudentAssessmentAttemptProgress(projection), projection);
  assert.throws(() =>
    decodeStudentAssessmentAttemptProgress({ ...projection, answer: { correct: true } }),
  );
  assert.throws(() =>
    decodeStudentAssessmentAttemptProgress({
      ...projection,
      positions: [{ position: 1, responseState: "submitted", response: "secret" }],
    }),
  );
});

test("Assessment Access carries only its authorized answer-free facts and Attempt history", () => {
  const decision = {
    availableAt: 1_000,
    dueAt: 2_000,
    closesAt: 3_000,
    timeLimitSeconds: 900,
    attemptLimit: 2,
    lateWorkRule: "reject",
    displayTimeZone: "America/Chicago",
    evaluatedAt: 1_500,
    startDecision: "may_start",
    publicReason: null,
  };
  const facts = {
    decision,
    title: "Peptide structure practice",
    assessmentType: "regular_assignment",
    questionCount: 4,
    pointsPossible: 8,
    previousAttempts: [
      {
        assessmentAttemptId: "00000000-0000-0000-0000-00000000000b",
        attemptNumber: 1,
        state: "submitted",
        score: { pointsEarned: 6, pointsPossible: 8 },
      },
    ],
  };
  const resumable = { activeAssessmentAttemptId: "00000000-0000-0000-0000-00000000000c", ...facts };
  const startable = {
    ...resumable,
    activeAssessmentAttemptId: null,
    decision: { ...decision, timeLimitSeconds: null },
  };
  const closedWithoutReleasedQuestions = {
    ...startable,
    decision: {
      ...startable.decision,
      startDecision: "closed",
      publicReason: "This Assessment is closed for new work.",
    },
    questionCount: 0,
    pointsPossible: 0,
  };
  assert.deepEqual(decodeLiveAssessmentAccess(resumable), resumable);
  assert.deepEqual(decodeLiveAssessmentAccess(startable), startable);
  assert.deepEqual(
    decodeLiveAssessmentAccess(closedWithoutReleasedQuestions),
    closedWithoutReleasedQuestions,
  );
  assert.throws(() => decodeLiveAssessmentAccess({ decision }));
  assert.throws(() => decodeLiveAssessmentAccess({ ...resumable, assessmentType: undefined }));
  assert.throws(() => decodeLiveAssessmentAccess({ ...resumable, assessmentType: "project" }));
  assert.throws(() =>
    decodeLiveAssessmentAccess({ ...resumable, activeAssessmentAttemptId: "12" }),
  );
  assert.throws(() => decodeLiveAssessmentAccess({ ...resumable, attemptId: "private" }));
  assert.throws(() =>
    decodeLiveAssessmentAccess({
      ...resumable,
      previousAttempts: [{ ...facts.previousAttempts[0], response: "secret" }],
    }),
  );
  assert.throws(() =>
    decodeLiveAssessmentAccess({
      ...resumable,
      previousAttempts: [{ ...facts.previousAttempts[0], score: null }],
    }),
  );
  assert.throws(() => decodeLiveAssessmentAccess({ ...resumable, answer: "secret" }));
  assert.throws(() =>
    decodeLiveAssessmentAccess({
      ...resumable,
      decision: { ...decision, accommodationId: "private" },
    }),
  );
  assert.throws(() =>
    decodeLiveAssessmentAccess({
      ...resumable,
      decision: { ...decision, publicReason: "Different browser-owned wording" },
    }),
  );
  assert.throws(() =>
    decodeLiveAssessmentAccess({
      ...resumable,
      decision: { ...decision, availableAt: 2_500 },
    }),
  );
});

test("selected presentation accepts only the exact existing public presentation contract", () => {
  const presentation = {
    position: 1,
    presentation: {
      publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
      prompt: [],
      response: { kind: "fillIn", maxCharacters: 10 },
    },
    savedResponse: null,
  };
  assert.deepEqual(decodeStudentAssessmentAttemptPresentation(presentation), presentation);
  assert.throws(() =>
    decodeStudentAssessmentAttemptPresentation({ ...presentation, checksum: "private" }),
  );
  assert.deepEqual(
    decodeStudentAssessmentAttemptPresentation({
      ...presentation,
      savedResponse: { kind: "shortText", text: "student working answer" },
    }).savedResponse,
    { kind: "shortText", text: "student working answer" },
  );
  assert.throws(() =>
    decodeStudentAssessmentAttemptPresentation({
      ...presentation,
      savedResponse: { kind: "shortText", text: "student working answer", correct: true },
    }),
  );
});

test("Student Assessment Attempt save and final submission use closed no-store contracts", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      const body = request.body === null ? null : await request.text();
      requests.push({ request, body });
      const path = new URL(request.url).pathname;
      if (path.endsWith("/submission")) {
        return noStoreJson({
          assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
          submissionState: "submitted",
        });
      }
      return noStoreJson({
        assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
        position: 2,
        responseState: "saved",
      });
    },
  });

  await client.saveStudentAssessmentAttemptResponse("00000000-0000-0000-0000-00000000000c", 2, {
    kind: "shortText",
    text: "student working answer",
  });
  assert.deepEqual(
    await client.submitStudentAssessmentAttempt("00000000-0000-0000-0000-00000000000c"),
    {
      assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
      submissionState: "submitted",
    },
  );

  assert.equal(requests[0].request.method, "PUT");
  assert.equal(
    requests[0].request.url,
    "https://ple.example/api/assessment-attempts/00000000-0000-0000-0000-00000000000c/responses/2",
  );
  assert.equal(requests[0].request.cache, "no-store");
  assert.deepEqual(JSON.parse(requests[0].body), {
    response: { kind: "shortText", text: "student working answer" },
  });
  assert.equal(requests[1].request.method, "POST");
  assert.equal(
    requests[1].request.url,
    "https://ple.example/api/assessment-attempts/00000000-0000-0000-0000-00000000000c/submission",
  );
  assert.equal(requests[1].request.cache, "no-store");
  assert.equal(requests[1].body, null);
});

test("final submission reports completion without grading data", () => {
  assert.deepEqual(
    decodeStudentAssessmentAttemptSubmissionResult({
      assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
      submissionState: "submitted",
    }),
    { assessmentAttemptId: "00000000-0000-0000-0000-00000000000c", submissionState: "submitted" },
  );
  assert.throws(() =>
    decodeStudentAssessmentAttemptSubmissionResult({
      assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
      submissionState: "submitted",
      score: { pointsEarned: 1, pointsPossible: 2 },
    }),
  );
});

test("Student Assessment Attempt mutations reject invalid requests and acknowledgements", async () => {
  const client = createHttpApiClient({
    fetch: () => Promise.reject(new Error("transport must not run")),
  });
  await assert.rejects(
    client.saveStudentAssessmentAttemptResponse("R-0", 1, { kind: "shortText", text: "x" }),
    ApiProtocolError,
  );
  await assert.rejects(
    client.saveStudentAssessmentAttemptResponse("00000000-0000-0000-0000-00000000000c", 0, {
      kind: "shortText",
      text: "x",
    }),
    ApiProtocolError,
  );
  assert.throws(
    () =>
      client.getStudentAssessmentAttemptPresentation("00000000-0000-0000-0000-00000000000c", 2.5),
    ApiProtocolError,
  );

  const mismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          assessmentAttemptId: "00000000-0000-0000-0000-00000000000d",
          position: 1,
          responseState: "saved",
        }),
      ),
  });
  await assert.rejects(
    () =>
      mismatch.saveStudentAssessmentAttemptResponse("00000000-0000-0000-0000-00000000000c", 1, {
        kind: "shortText",
        text: "x",
      }),
    /attempt does not match/u,
  );

  const positionMismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
          position: 2,
          responseState: "saved",
        }),
      ),
  });
  await assert.rejects(
    () =>
      positionMismatch.saveStudentAssessmentAttemptResponse(
        "00000000-0000-0000-0000-00000000000c",
        1,
        {
          kind: "shortText",
          text: "x",
        },
      ),
    /position does not match/u,
  );

  const badFinalSubmission = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
          submissionState: "saved",
        }),
      ),
  });
  await assert.rejects(
    () => badFinalSubmission.submitStudentAssessmentAttempt("00000000-0000-0000-0000-00000000000c"),
    /submissionState/u,
  );
});

test("Student Assessment Attempt GET projections match their request", async () => {
  const progressMismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          assessmentAttemptId: "00000000-0000-0000-0000-00000000000d",
          questionCount: 1,
          recommendedPosition: 1,
          positions: [{ position: 1, responseState: "unanswered" }],
        }),
      ),
  });
  await assert.rejects(
    () =>
      progressMismatch.getStudentAssessmentAttemptProgress("00000000-0000-0000-0000-00000000000c"),
    /progress does not match/u,
  );

  const presentationPositionMismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          position: 2,
          presentation: {
            publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
            prompt: [],
            response: { kind: "fillIn", maxCharacters: 10 },
          },
          savedResponse: null,
        }),
      ),
  });
  await assert.rejects(
    () =>
      presentationPositionMismatch.getStudentAssessmentAttemptPresentation(
        "00000000-0000-0000-0000-00000000000c",
        1,
      ),
    /presentation position does not match/u,
  );
});

test("Attempt routes admit the Student product role", () => {
  for (const routeId of ["assessmentAttempt", "assessmentAttemptSummary"]) {
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
    assert.deepEqual(route?.requiredProductRoles, ["student"]);
  }
});
