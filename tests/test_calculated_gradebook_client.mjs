import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeCalculatedGradebookResult,
  decodeInspectedStudentWorkDetail,
} from "../src/api/decoders/calculated_gradebook.ts";
import {
  decodeGradebookSelectionResult,
  decodeSubmittedRunChoicesPage,
} from "../src/api/decoders/gradebook_selection.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";
import { createCalculatedGradebookClient } from "../src/api/http_client/calculated_gradebook.ts";

const COURSE_ID = "00000000-0000-0000-0000-000000000001";

function jsonResponse(body, headers = {}) {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "cache-control": "no-store", "content-type": "application/json", ...headers },
  });
}

function selectedStudent(assignment = "A-2") {
  return {
    kind: "singleStudent",
    membership: "M-1",
    assignment,
    inspectionChoice: {
      kind: "selectedRun",
      basis: "latest",
      run: "R-3",
      submittedAt: 1_700_000_000_000,
    },
  };
}

function inspectedDetail(returnContext = "gradingOperation", membership = "M-1") {
  const common = {
    course: "C-1",
    membership,
    assignment: "A-2",
  };
  const context =
    returnContext === "gradingOperation"
      ? {
          kind: "gradingOperation",
          ...common,
          operation: "GO-7",
          focus: {
            kind: "gradingOperationControl",
            membership,
            assignment: "A-2",
            operation: "GO-7",
          },
        }
      : {
          kind: "gradebook",
          ...common,
          focus: { kind: "gradebookCell", membership, assignment: "A-2" },
        };
  return {
    ...common,
    run: "R-3",
    studentDisplayLabel: "Ada Student",
    assignmentTitle: "Peptide Bonds: Guided Practice",
    submissions: [],
    returnContext: context,
  };
}

test("calculated Gradebook decoder accepts a nested page and rejects extra fields", () => {
  const page = {
    kind: "page",
    schemeRevision: 1,
    rosterRevision: 1,
    mode: "totalPoints",
    rounding: "fourDecimalPlacesHalfAwayFromZero",
    observationTime: 1_700_000_000_000,
    scoringWitnesses: [{ assignment: "A-2", generation: 1, status: "current" }],
    rows: [
      {
        membership: "M-1",
        displayLabel: "Ada Student",
        outcome: { status: "unavailable", reason: "noIncludedAssignments" },
        assignmentCells: [
          {
            assignment: "A-2",
            title: "Practice",
            included: true,
            category: null,
            availability: "unavailable",
            selectedScore: null,
            scoringStatus: "current",
            inspectionChoice: { kind: "noSubmittedRun" },
          },
        ],
      },
    ],
  };
  const decoded = decodeCalculatedGradebookResult(page);
  assert.equal(decoded.kind, "page");
  assert.equal(decoded.nextCursor, null);
  assert.equal(decoded.rows[0].assignmentCells[0].inspectionChoice.kind, "noSubmittedRun");
  assert.throws(() => decodeCalculatedGradebookResult({ ...page, extraField: true }), DecodeError);
});

test("Gradebook selection and submitted-run decoders keep closed public-reference pages", () => {
  const selection = decodeGradebookSelectionResult({
    kind: "studentSelection",
    rows: [
      {
        membership: "M-1",
        displayLabel: "Ada Student",
        assignment: "A-2",
        inspectionChoice: { kind: "chooseRun", completedRunCount: 2 },
      },
    ],
    nextCursor: "next-page",
  });
  assert.equal(selection.kind, "studentSelection");
  assert.equal(selection.nextCursor, "next-page");

  const choices = decodeSubmittedRunChoicesPage({
    rosterRevision: 4,
    rows: [{ run: "R-3", submittedAt: 1_700_000_000_000, scoreSelected: true }],
  });
  assert.equal(choices.nextCursor, null);
  assert.equal(choices.rows[0].run, "R-3");

  assert.throws(
    () => decodeGradebookSelectionResult({ ...selectedStudent(), privateStudentId: "hidden" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeGradebookSelectionResult({
        kind: "studentSelection",
        rows: [],
        nextCursor: "",
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSubmittedRunChoicesPage({
        rosterRevision: 4,
        rows: [{ run: "R-03", submittedAt: 1_700_000_000_000, scoreSelected: true }],
      }),
    DecodeError,
  );
});

test("inspection decoder keeps required presentation labels outside its return identity", () => {
  const decoded = decodeInspectedStudentWorkDetail(inspectedDetail());
  assert.equal(decoded.studentDisplayLabel, "Ada Student");
  assert.equal(decoded.assignmentTitle, "Peptide Bonds: Guided Practice");
  assert.equal(
    decodeInspectedStudentWorkDetail(inspectedDetail("gradebook")).returnContext.kind,
    "gradebook",
  );

  const mismatchedOperation = inspectedDetail();
  mismatchedOperation.returnContext.focus.operation = "GO-8";
  assert.throws(() => decodeInspectedStudentWorkDetail(mismatchedOperation), DecodeError);

  const contaminated = inspectedDetail("gradebook");
  contaminated.returnContext.answerKey = "private";
  assert.throws(() => decodeInspectedStudentWorkDetail(contaminated), DecodeError);

  const withoutStudentLabel = inspectedDetail();
  delete withoutStudentLabel.studentDisplayLabel;
  assert.throws(() => decodeInspectedStudentWorkDetail(withoutStudentLabel), DecodeError);
});

test("inspection decoder rejects malformed current presentation labels", () => {
  const malformedStudentLabel = inspectedDetail();
  malformedStudentLabel.studentDisplayLabel = " ";
  assert.throws(() => decodeInspectedStudentWorkDetail(malformedStudentLabel), DecodeError);

  const malformedAssignmentTitle = inspectedDetail();
  malformedAssignmentTitle.assignmentTitle = "";
  assert.throws(() => decodeInspectedStudentWorkDetail(malformedAssignmentTitle), DecodeError);
});

test("calculated Gradebook clients use same-origin no-store lowerCamelCase routes", async () => {
  const calls = [];
  const fetchImplementation = async (input, init) => {
    const path = String(input);
    calls.push({ path, init });
    if (path.includes("/selection?")) return jsonResponse(selectedStudent());
    if (path.endsWith("/assignment-attempts/R-3?operationRef=GO-7")) {
      return jsonResponse(inspectedDetail());
    }
    if (path.includes("/assignment-attempts?")) {
      return jsonResponse({
        rosterRevision: 4,
        rows: [{ run: "R-3", submittedAt: 1_700_000_000_000, scoreSelected: true }],
      });
    }
    return jsonResponse({ kind: "reloadRequired", reason: "filterChanged" });
  };
  const client = createCalculatedGradebookClient(fetchImplementation, "/live");

  await client.getCalculatedGradebook(COURSE_ID, {
    filter: { kind: "operation", operation: "GO-7" },
  });
  await client.getCalculatedGradebook(COURSE_ID, {
    cursor: "next-page",
    pageSize: 10,
    filter: { kind: "assignment", assignment: "A-2" },
  });
  await client.getCalculatedGradebook(COURSE_ID, {
    cursor: "next-page",
    pageSize: 10,
    filter: { kind: "student", membership: "M-1" },
  });
  await client.getGradebookSelection(COURSE_ID, {
    filter: { kind: "operation", operation: "GO-7" },
    pageSize: 25,
  });
  await client.getSubmittedRunChoices(COURSE_ID, "M-1", "A-2", {
    cursor: "next-page",
    pageSize: 10,
    operationRef: "GO-7",
  });
  await client.getInspectedStudentWork(COURSE_ID, "M-1", "A-2", "R-3", "GO-7");

  const root = `/live/api/courses/${COURSE_ID}/gradebook`;
  assert.equal(calls[0].path, `${root}?operationRef=GO-7`);
  assert.equal(calls[1].path, `${root}?cursor=next-page&pageSize=10&assignmentRef=A-2`);
  assert.equal(calls[2].path, `${root}?cursor=next-page&pageSize=10&membershipRef=M-1`);
  assert.equal(calls[3].path, `${root}/selection?pageSize=25&operationRef=GO-7`);
  assert.equal(
    calls[4].path,
    `${root}/students/M-1/assignments/A-2/assignment-attempts?cursor=next-page&pageSize=10&operationRef=GO-7`,
  );
  assert.equal(
    calls[5].path,
    `${root}/students/M-1/assignments/A-2/assignment-attempts/R-3?operationRef=GO-7`,
  );
  for (const call of calls) {
    assert.equal(call.init.credentials, "same-origin");
    assert.equal(call.init.cache, "no-store");
    assert.equal(call.init.method, "GET");
  }
});

test("Gradebook clients reject malformed inputs, cache drift, and echoed identity drift", async () => {
  let calls = 0;
  const client = createCalculatedGradebookClient(async () => {
    calls += 1;
    return jsonResponse({ kind: "reloadRequired", reason: "filterChanged" });
  }, "");
  assert.throws(
    () =>
      client.getCalculatedGradebook(COURSE_ID, {
        filter: { kind: "operation", operation: "GO-07" },
      }),
    DecodeError,
  );
  assert.throws(() => client.getSubmittedRunChoices(COURSE_ID, "M-01", "A-2"), ApiProtocolError);
  await assert.rejects(
    client.getGradebookSelection(COURSE_ID, {
      filter: { kind: "assignment", assignment: "A-2" },
      cursor: "",
    }),
    DecodeError,
  );
  assert.equal(calls, 0);

  const cacheable = createCalculatedGradebookClient(
    async () => jsonResponse(selectedStudent(), { "cache-control": "private, max-age=60" }),
    "",
  );
  await assert.rejects(
    cacheable.getGradebookSelection(COURSE_ID, {
      filter: { kind: "assignment", assignment: "A-2" },
    }),
    ApiProtocolError,
  );

  const mismatchedSelection = createCalculatedGradebookClient(
    async () => jsonResponse(selectedStudent("A-9")),
    "",
  );
  await assert.rejects(
    mismatchedSelection.getGradebookSelection(COURSE_ID, {
      filter: { kind: "assignment", assignment: "A-2" },
    }),
    ApiProtocolError,
  );

  const mismatchedInspection = createCalculatedGradebookClient(
    async () => jsonResponse(inspectedDetail("gradebook", "M-9")),
    "",
  );
  await assert.rejects(
    mismatchedInspection.getInspectedStudentWork(COURSE_ID, "M-1", "A-2", "R-3", "GO-7"),
    ApiProtocolError,
  );
});
