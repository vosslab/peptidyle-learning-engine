import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeInstructorGradingOperationsPage } from "../src/api/decoders/grading_operations.ts";
import { createGradingOperationsClient } from "../src/api/http_client/grading_operations.ts";

const COURSE_ID = "00000000-0000-0000-0000-000000000001";
const ASSIGNMENT_ID = "00000000-0000-0000-0000-000000000002";
const RETRY_TOKEN = "00000000-0000-0000-0000-000000000003";

function page() {
  return {
    items: [
      {
        operation: {
          reference: "GO-7",
          reason: "grader_execution_failure",
          state: "actionable",
          revision: 3,
          nextAction: "retry",
        },
        subject: { kind: "question", questionId: "ABC-1234", title: "Protein folding" },
        affectedStudentCount: 2,
        trustGeneration: { kind: "execution", generation: 4 },
      },
    ],
    nextCursor: null,
  };
}

function jsonResponse(body, headers = {}) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "cache-control": "no-store", "content-type": "application/json", ...headers },
  });
}

test("grading operation decoder accepts safe closed metadata and rejects additional fields", () => {
  const decoded = decodeInstructorGradingOperationsPage(page());
  assert.equal(decoded.items[0].subject.kind, "question");
  assert.equal(decoded.items[0].operation.reference, "GO-7");

  const withExtraField = page();
  withExtraField.items[0].operation.answer = "private";
  assert.throws(() => decodeInstructorGradingOperationsPage(withExtraField), DecodeError);

  const withOverflowReference = page();
  withOverflowReference.items[0].operation.reference = "GO-4294967296";
  assert.throws(() => decodeInstructorGradingOperationsPage(withOverflowReference), DecodeError);

  const withOverflowCount = page();
  withOverflowCount.items[0].affectedStudentCount = 4_294_967_296;
  assert.throws(() => decodeInstructorGradingOperationsPage(withOverflowCount), DecodeError);
});

test("grading operations client sends no-store list and empty guarded action requests", async () => {
  const calls = [];
  const fetchImplementation = async (input, init) => {
    calls.push({ input: String(input), init });
    if (init.method === "GET") return jsonResponse(page());
    if (String(input).endsWith("/retry")) {
      return jsonResponse(
        {
          kind: "retry",
          retry_token: RETRY_TOKEN,
          operation: "GO-7",
          resulting_operation_revision: 4,
          occurred_at: 1_700_000_000_000,
        },
        { etag: '"4"' },
      );
    }
    return jsonResponse(
      {
        kind: "recalculation",
        retry_token: RETRY_TOKEN,
        operation: "GO-8",
        resulting_operation_revision: 1,
        assignment_revision: 9,
        scoring_generation: 6,
        occurred_at: 1_700_000_000_001,
      },
      { etag: '"9"' },
    );
  };
  const client = createGradingOperationsClient(fetchImplementation, "/live");

  await client.listInstructorGradingOperations(COURSE_ID, ASSIGNMENT_ID, "student", "next", 25);
  await client.retryInstructorGradingOperation(
    COURSE_ID,
    ASSIGNMENT_ID,
    "GO-7",
    '"3"',
    RETRY_TOKEN,
  );
  await client.retryInstructorGradingOperation(
    COURSE_ID,
    ASSIGNMENT_ID,
    "GO-7",
    '"3"',
    RETRY_TOKEN,
  );
  await client.recalculateInstructorAssignment(COURSE_ID, ASSIGNMENT_ID, '"8"', RETRY_TOKEN);

  assert.equal(
    calls[0].input,
    `/live/api/courses/${COURSE_ID}/assignments/${ASSIGNMENT_ID}/grading-operations?focus=student&cursor=next&pageSize=25`,
  );
  assert.equal(calls[0].init.cache, "no-store");
  assert.equal(calls[1].init.method, "POST");
  assert.equal(calls[1].init.body, undefined);
  assert.equal(calls[1].init.headers["if-match"], '"3"');
  assert.equal(calls[1].init.headers["idempotency-key"], RETRY_TOKEN);
  assert.equal(calls[2].init.headers["idempotency-key"], RETRY_TOKEN);
  assert.equal(calls[3].init.body, undefined);
  assert.equal(calls[3].init.headers["if-match"], '"8"');
});

test("grading operations client rejects malformed and mismatched Retry Tokens", async () => {
  const client = createGradingOperationsClient(
    async () =>
      jsonResponse(
        {
          kind: "retry",
          retry_token: "00000000-0000-0000-0000-000000000004",
          operation: "GO-7",
          resulting_operation_revision: 4,
          occurred_at: 1_700_000_000_000,
        },
        { etag: '"4"' },
      ),
    "/live",
  );

  await assert.rejects(
    client.retryInstructorGradingOperation(COURSE_ID, ASSIGNMENT_ID, "GO-7", '"3"', "not-a-token"),
    /instructorGradingOperationRetryToken/,
  );
  await assert.rejects(
    client.retryInstructorGradingOperation(COURSE_ID, ASSIGNMENT_ID, "GO-7", '"3"', RETRY_TOKEN),
    /Retry Token must match the request Instructor Grading Operation Retry Token/,
  );
});

test("grading operation decoder rejects retired action and malformed Retry Token receipt fields", async () => {
  const receipt = {
    kind: "retry",
    retry_token: RETRY_TOKEN,
    operation: "GO-7",
    resulting_operation_revision: 4,
    occurred_at: 1_700_000_000_000,
  };
  const withRetiredAction = { ...receipt, action: RETRY_TOKEN };
  const withMalformedRetryToken = { ...receipt, retry_token: "not-a-token" };

  const client = createGradingOperationsClient(
    async () => jsonResponse(withRetiredAction, { etag: '"4"' }),
    "/live",
  );
  await assert.rejects(
    client.retryInstructorGradingOperation(COURSE_ID, ASSIGNMENT_ID, "GO-7", '"3"', RETRY_TOKEN),
    DecodeError,
  );

  const malformedClient = createGradingOperationsClient(
    async () => jsonResponse(withMalformedRetryToken, { etag: '"4"' }),
    "/live",
  );
  await assert.rejects(
    malformedClient.retryInstructorGradingOperation(
      COURSE_ID,
      ASSIGNMENT_ID,
      "GO-7",
      '"3"',
      RETRY_TOKEN,
    ),
    DecodeError,
  );
});
