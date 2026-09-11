import assert from "node:assert/strict";
import test from "node:test";

import { decodeLiveAssignmentAccess } from "../src/api/decoders/assignment_attempt_issuance.ts";
import {
  decodeStudentAssignmentAttemptPresentation,
  decodeStudentAssignmentAttemptProgress,
} from "../src/api/decoders/assignment_attempt_navigation.ts";
import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { ROUTE_CONTRACT } from "../src/route_contract.ts";

function noStoreJson(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Student Assignment Attempt progress rejects answer-bearing and extra fields", () => {
  const projection = {
    assignmentAttempt: "R-12",
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
  assert.deepEqual(decodeStudentAssignmentAttemptProgress(projection), projection);
  assert.throws(() =>
    decodeStudentAssignmentAttemptProgress({ ...projection, answer: { correct: true } }),
  );
  assert.throws(() =>
    decodeStudentAssignmentAttemptProgress({
      ...projection,
      positions: [{ position: 1, responseState: "submitted", response: "secret" }],
    }),
  );
});

test("Assignment Access carries only its authorized answer-free facts and Attempt history", () => {
  const facts = {
    title: "Peptide structure practice",
    questionCount: 4,
    pointsPossible: 8,
    timeLimitSeconds: 900,
    previousAttempts: [
      {
        assignmentAttempt: "R-11",
        attemptNumber: 1,
        state: "submitted",
        score: { pointsEarned: 6, pointsPossible: 8 },
      },
    ],
  };
  const resumable = { startDecision: "may_start", activeAssignmentAttempt: "R-12", ...facts };
  const startable = { ...resumable, activeAssignmentAttempt: null, timeLimitSeconds: null };
  const closedWithoutReleasedQuestions = {
    ...startable,
    startDecision: "closed",
    questionCount: 0,
    pointsPossible: 0,
  };
  assert.deepEqual(decodeLiveAssignmentAccess(resumable), resumable);
  assert.deepEqual(decodeLiveAssignmentAccess(startable), startable);
  assert.deepEqual(
    decodeLiveAssignmentAccess(closedWithoutReleasedQuestions),
    closedWithoutReleasedQuestions,
  );
  assert.throws(() => decodeLiveAssignmentAccess({ startDecision: "may_start" }));
  assert.throws(() => decodeLiveAssignmentAccess({ ...resumable, activeAssignmentAttempt: "12" }));
  assert.throws(() => decodeLiveAssignmentAccess({ ...resumable, attemptId: "private" }));
  assert.throws(() =>
    decodeLiveAssignmentAccess({
      ...resumable,
      previousAttempts: [{ ...facts.previousAttempts[0], response: "secret" }],
    }),
  );
  assert.throws(() =>
    decodeLiveAssignmentAccess({
      ...resumable,
      previousAttempts: [{ ...facts.previousAttempts[0], score: null }],
    }),
  );
  assert.throws(() => decodeLiveAssignmentAccess({ ...resumable, answer: "secret" }));
});

test("selected presentation accepts only the exact existing public presentation contract", () => {
  const presentation = {
    position: 1,
    presentation: {
      prompt: [],
      response: { kind: "fillIn", maxCharacters: 10 },
    },
    savedResponse: null,
  };
  assert.deepEqual(decodeStudentAssignmentAttemptPresentation(presentation), presentation);
  assert.throws(() =>
    decodeStudentAssignmentAttemptPresentation({ ...presentation, checksum: "private" }),
  );
  assert.deepEqual(
    decodeStudentAssignmentAttemptPresentation({
      ...presentation,
      savedResponse: { kind: "shortText", text: "student working answer" },
    }).savedResponse,
    { kind: "shortText", text: "student working answer" },
  );
  assert.throws(() =>
    decodeStudentAssignmentAttemptPresentation({
      ...presentation,
      savedResponse: { kind: "shortText", text: "student working answer", correct: true },
    }),
  );
});

test("Student Assignment Attempt save and final submission use closed no-store contracts", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      const body = request.body === null ? null : await request.text();
      requests.push({ request, body });
      const path = new URL(request.url).pathname;
      if (path.endsWith("/submission")) {
        return noStoreJson({ assignmentAttempt: "R-12", submissionState: "submitted" });
      }
      return noStoreJson({ assignmentAttempt: "R-12", position: 2, responseState: "saved" });
    },
  });

  await client.saveStudentAssignmentAttemptResponse("R-12", 2, {
    kind: "shortText",
    text: "student working answer",
  });
  await client.submitStudentAssignmentAttempt("R-12");

  assert.equal(requests[0].request.method, "PUT");
  assert.equal(
    requests[0].request.url,
    "https://ple.example/api/assignment-attempts/R-12/responses/2",
  );
  assert.equal(requests[0].request.cache, "no-store");
  assert.deepEqual(JSON.parse(requests[0].body), {
    response: { kind: "shortText", text: "student working answer" },
  });
  assert.equal(requests[1].request.method, "POST");
  assert.equal(
    requests[1].request.url,
    "https://ple.example/api/assignment-attempts/R-12/submission",
  );
  assert.equal(requests[1].request.cache, "no-store");
  assert.equal(requests[1].body, null);
});

test("Student Assignment Attempt mutations reject invalid requests and acknowledgements", async () => {
  const client = createHttpApiClient({
    fetch: () => Promise.reject(new Error("transport must not run")),
  });
  await assert.rejects(
    client.saveStudentAssignmentAttemptResponse("R-0", 1, { kind: "shortText", text: "x" }),
    ApiProtocolError,
  );
  await assert.rejects(
    client.saveStudentAssignmentAttemptResponse("R-12", 0, { kind: "shortText", text: "x" }),
    ApiProtocolError,
  );
  assert.throws(
    () => client.getStudentAssignmentAttemptPresentation("R-12", 2.5),
    ApiProtocolError,
  );

  const mismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({ assignmentAttempt: "R-13", position: 1, responseState: "saved" }),
      ),
  });
  await assert.rejects(
    () =>
      mismatch.saveStudentAssignmentAttemptResponse("R-12", 1, { kind: "shortText", text: "x" }),
    /attempt does not match/u,
  );

  const positionMismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({ assignmentAttempt: "R-12", position: 2, responseState: "saved" }),
      ),
  });
  await assert.rejects(
    () =>
      positionMismatch.saveStudentAssignmentAttemptResponse("R-12", 1, {
        kind: "shortText",
        text: "x",
      }),
    /position does not match/u,
  );

  const badFinalSubmission = createHttpApiClient({
    fetch: () =>
      Promise.resolve(noStoreJson({ assignmentAttempt: "R-12", submissionState: "saved" })),
  });
  await assert.rejects(
    () => badFinalSubmission.submitStudentAssignmentAttempt("R-12"),
    /submissionState/u,
  );
});

test("Student Assignment Attempt GET projections match their request", async () => {
  const progressMismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          assignmentAttempt: "R-13",
          questionCount: 1,
          recommendedPosition: 1,
          positions: [{ position: 1, responseState: "unanswered" }],
        }),
      ),
  });
  await assert.rejects(
    () => progressMismatch.getStudentAssignmentAttemptProgress("R-12"),
    /progress does not match/u,
  );

  const presentationPositionMismatch = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        noStoreJson({
          position: 2,
          presentation: {
            prompt: [],
            response: { kind: "fillIn", maxCharacters: 10 },
          },
          savedResponse: null,
        }),
      ),
  });
  await assert.rejects(
    () => presentationPositionMismatch.getStudentAssignmentAttemptPresentation("R-12", 1),
    /presentation position does not match/u,
  );
});

test("Attempt routes admit the Student product role", () => {
  for (const routeId of ["assignmentAttempt", "assignmentAttemptSummary"]) {
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
    assert.deepEqual(route?.requiredProductRoles, ["student"]);
  }
});
