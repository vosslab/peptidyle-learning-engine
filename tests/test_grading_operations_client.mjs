import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeInstructorGradingOperationsPage } from "../src/api/decoders/grading_operations.ts";
import { createGradingOperationsClient } from "../src/api/http_client/grading_operations.ts";

const COURSE_ID = "00000000-0000-0000-0000-000000000001";
const ASSIGNMENT_ID = "00000000-0000-0000-0000-000000000002";
const ACTION_ID = "00000000-0000-0000-0000-000000000003";

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
        group: { kind: "question", questionId: "ABC-1234", title: "Protein folding" },
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
  assert.equal(decoded.items[0].group.kind, "question");
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
          action: ACTION_ID,
          operation: "GO-7",
          resultingOperationRevision: 4,
          occurredAt: 1_700_000_000_000,
        },
        { etag: '"4"' },
      );
    }
    return jsonResponse(
      {
        kind: "recalculation",
        action: ACTION_ID,
        operation: "GO-8",
        resultingOperationRevision: 1,
        assignmentRevision: 9,
        scoringGeneration: 6,
        occurredAt: 1_700_000_000_001,
      },
      { etag: '"9"' },
    );
  };
  const client = createGradingOperationsClient(fetchImplementation, "/live");

  await client.listInstructorGradingOperations(COURSE_ID, ASSIGNMENT_ID, "student", "next", 25);
  await client.retryInstructorGradingOperation(COURSE_ID, ASSIGNMENT_ID, "GO-7", '"3"', ACTION_ID);
  await client.recalculateInstructorAssignment(COURSE_ID, ASSIGNMENT_ID, '"8"', ACTION_ID);

  assert.equal(
    calls[0].input,
    `/live/api/courses/${COURSE_ID}/assignments/${ASSIGNMENT_ID}/grading-operations?groupBy=student&cursor=next&pageSize=25`,
  );
  assert.equal(calls[0].init.cache, "no-store");
  assert.equal(calls[1].init.method, "POST");
  assert.equal(calls[1].init.body, undefined);
  assert.equal(calls[1].init.headers["if-match"], '"3"');
  assert.equal(calls[1].init.headers["idempotency-key"], ACTION_ID);
  assert.equal(calls[2].init.body, undefined);
  assert.equal(calls[2].init.headers["if-match"], '"8"');
});
