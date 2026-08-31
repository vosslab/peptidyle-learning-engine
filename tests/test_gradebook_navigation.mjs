import assert from "node:assert/strict";
import test from "node:test";

import {
  gradebookCellFocusId,
  gradebookQueryForFilter,
  gradebookReturnUrl,
  gradingOperationControlFocusId,
  gradingOperationReturnUrl,
  inspectedStudentWorkUrl,
  inspectedStudentWorkReturnUrl,
  operationGradebookUrl,
  parseGradebookRouteSearch,
  parseInspectedStudentWorkRouteSearch,
} from "../src/pages/gradebook_navigation.ts";

test("Gradebook navigation keeps each public context and its exact return target", () => {
  const assignment = parseGradebookRouteSearch("?assignmentRef=A-2");
  const student = parseGradebookRouteSearch("?membershipRef=M-3");
  const operation = parseGradebookRouteSearch("?operationRef=GO-7");

  assert.deepEqual(
    gradebookQueryForFilter(assignment.kind === "valid" ? assignment.filter : undefined),
    { filter: { kind: "assignment", assignment: "A-2" } },
  );
  assert.deepEqual(gradebookQueryForFilter(student.kind === "valid" ? student.filter : undefined), {
    filter: { kind: "student", membership: "M-3" },
  });
  assert.equal(
    gradebookQueryForFilter(operation.kind === "valid" ? operation.filter : undefined).filter?.kind,
    "operation",
  );
  assert.equal(gradebookCellFocusId("M-3", "A-2"), "gradebook-cell-M-3-A-2");
  assert.equal(
    gradebookReturnUrl("C-1", "M-3", "A-2"),
    "/instructor/courses/C-1/gradebook?membershipRef=M-3#gradebook-cell-M-3-A-2",
  );
});

test("operation and detail URLs retain only canonical public references", () => {
  assert.equal(
    operationGradebookUrl("C-1", "GO-7"),
    "/instructor/courses/C-1/gradebook?operationRef=GO-7",
  );
  assert.equal(gradingOperationControlFocusId("GO-7"), "grading-operation-control-GO-7");
  assert.equal(
    gradingOperationReturnUrl("C-1", "A-2", "GO-7"),
    "/instructor/courses/C-1/assignments/A-2/grading-operations#grading-operation-control-GO-7",
  );
  assert.equal(
    inspectedStudentWorkUrl("C-1", "M-3", "A-2", "R-4"),
    "/instructor/courses/C-1/gradebook/students/M-3/assignments/A-2/assignment-attempts/R-4",
  );
  assert.equal(
    inspectedStudentWorkUrl("C-1", "M-3", "A-2", "R-4", "GO-7"),
    "/instructor/courses/C-1/gradebook/students/M-3/assignments/A-2/assignment-attempts/R-4?operationRef=GO-7",
  );
});

test("Gradebook search rejects duplicate, unknown, and malformed filters", () => {
  assert.deepEqual(parseGradebookRouteSearch("?membershipRef=M-3&membershipRef=M-3"), {
    kind: "invalid",
    reason: "duplicateKey",
    key: "membershipRef",
  });
  assert.deepEqual(parseGradebookRouteSearch("?student=M-3"), {
    kind: "invalid",
    reason: "unknownKey",
    key: "student",
  });
  assert.deepEqual(parseGradebookRouteSearch("?assignmentRef=A-2&membershipRef=M-3"), {
    kind: "invalid",
    reason: "multipleFilters",
  });
  assert.deepEqual(parseGradebookRouteSearch("?assignmentRef=A-0"), {
    kind: "invalid",
    reason: "invalidReference",
    key: "assignmentRef",
  });
  assert.deepEqual(parseGradebookRouteSearch("?operationRef=GO-0"), {
    kind: "invalid",
    reason: "invalidReference",
    key: "operationRef",
  });
});

test("inspected work accepts one operation origin and returns through the verified context", () => {
  assert.deepEqual(parseInspectedStudentWorkRouteSearch(""), {
    kind: "valid",
    operation: undefined,
  });
  assert.deepEqual(parseInspectedStudentWorkRouteSearch("?operationRef=GO-7"), {
    kind: "valid",
    operation: "GO-7",
  });
  assert.deepEqual(parseInspectedStudentWorkRouteSearch("?operationRef=GO-7&operationRef=GO-8"), {
    kind: "invalid",
    reason: "duplicateKey",
    key: "operationRef",
  });
  assert.deepEqual(parseInspectedStudentWorkRouteSearch("?membershipRef=M-3"), {
    kind: "invalid",
    reason: "unknownKey",
    key: "membershipRef",
  });
  assert.equal(
    inspectedStudentWorkReturnUrl({
      kind: "gradebook",
      course: "C-1",
      membership: "M-3",
      assignment: "A-2",
      focus: { kind: "gradebookCell", membership: "M-3", assignment: "A-2" },
    }),
    "/instructor/courses/C-1/gradebook?membershipRef=M-3#gradebook-cell-M-3-A-2",
  );
  assert.equal(
    inspectedStudentWorkReturnUrl({
      kind: "gradingOperation",
      course: "C-1",
      membership: "M-3",
      assignment: "A-2",
      operation: "GO-7",
      focus: {
        kind: "gradingOperationControl",
        membership: "M-3",
        assignment: "A-2",
        operation: "GO-7",
      },
    }),
    "/instructor/courses/C-1/assignments/A-2/grading-operations#grading-operation-control-GO-7",
  );
});
