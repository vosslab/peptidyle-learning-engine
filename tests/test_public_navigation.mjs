// Human navigation uses compact typed references while internal UUIDs remain API identities.

import assert from "node:assert/strict";
import test from "node:test";

import {
  assignmentRouteReference,
  courseInstanceRouteReference,
  parseAssignmentReference,
  parseCourseInstanceReference,
  parseQuestionRouteReference,
  parsePublicRouteReference,
  parseAssignmentAttemptReference,
  parseAuthoringWorkspaceReference,
  parseDraftQuestionReference,
  questionRouteReference,
  assignmentAttemptRouteReference,
  authoringWorkspaceRouteReference,
  draftQuestionRouteReference,
} from "../src/navigation/public_route.ts";
import {
  resolveAssignmentRoute,
  resolveCourseRoute,
  resolveAssignmentAttemptRoute,
  resolveAssignmentAttemptIdentity,
  resolveCourseIdentity,
  resolveWorkspaceRoute,
} from "../src/navigation/resolved_route.ts";
import { isAssignmentReference, isCourseInstanceReference } from "./support/public_references.ts";

test("human route references are compact, typed, and bounded", () => {
  assert.equal(courseInstanceRouteReference("CI7K3M2Q"), "CI7K3M2Q");
  assert.equal(assignmentRouteReference("A9D2RX5"), "A9D2RX5");
  assert.equal(assignmentAttemptRouteReference("R-30"), "R-30");
  assert.equal(authoringWorkspaceRouteReference("W-40"), "W-40");
  assert.equal(draftQuestionRouteReference("D-50"), "D-50");
  assert.equal(questionRouteReference("7K3M-X9QP"), "7K3M-X9QP");
  assert.equal(isCourseInstanceReference("CI7K3M2Q"), true);
  assert.equal(isCourseInstanceReference("A9D2RX5"), false);
  assert.equal(isAssignmentReference("A9D2RX5"), true);
  assert.equal(isAssignmentReference("CI7K3M2Q"), false);

  for (const reference of ["CI7K3M2Q", "A9D2RX5", "R-30", "W-40", "D-50"]) {
    assert.equal(parsePublicRouteReference(reference), reference);
  }
  for (const [parser, valid, rejected] of [
    [parseCourseInstanceReference, "CI7K3M2Q", ["C-1", "CI7K3M2", "CI7K3M2I"]],
    [parseAssignmentReference, "A9D2RX5", ["A-1", "A9D2RX", "A9D2RXI"]],
    [parseAssignmentAttemptReference, "R"],
    [parseAuthoringWorkspaceReference, "W"],
    [parseDraftQuestionReference, "D"],
  ]) {
    if (Array.isArray(rejected)) {
      assert.equal(parser(valid), valid);
      for (const invalid of rejected) assert.equal(parser(invalid), null);
    } else {
      assert.equal(parser(`${valid}-1`), `${valid}-1`);
      for (const invalid of [
        `${valid}-0`,
        `${valid}-01`,
        `${valid}-2147483648`,
        "X-1",
        "0198e000-0000-7000-8000-000000000001",
      ]) {
        assert.equal(parser(invalid), null);
      }
    }
  }
  assert.equal(parseQuestionRouteReference("7k3mx9qp"), "7K3M-X9QP");
  assert.equal(parseQuestionRouteReference("OI0O-XOlx"), "0100-X01X");
  assert.equal(parseQuestionRouteReference("P-50-v3"), null);
  assert.equal(parseQuestionRouteReference("7K3-M9QU"), null);
});

test("route resolution recovers protected API identities without weakening reference kinds", async () => {
  const fixture = {
    course: { reference: "CI7K3M2Q", id: "course-id" },
    assignment: { reference: "A9D2RX5", id: "assignment-id" },
    assignmentAttempt: { reference: "R-1", id: "assignment-attempt-id" },
    workspace: { reference: "W-1", id: "workspace-id" },
  };
  const client = {
    resolveNavigation: async (reference) => {
      const values = {
        CI7K3M2Q: { kind: "course", courseId: fixture.course.id },
        A9D2RX5: {
          kind: "assignment",
          courseId: fixture.course.id,
          assignmentId: fixture.assignment.id,
        },
        "R-1": {
          kind: "assignmentAttempt",
          courseId: fixture.course.id,
          assignmentId: fixture.assignment.id,
          studentRecordId: "student-record-id",
          assignmentAttemptId: fixture.assignmentAttempt.id,
        },
        "W-1": { kind: "workspace", workspaceId: fixture.workspace.id },
      };
      return values[reference];
    },
  };

  assert.equal(await resolveCourseRoute(client, fixture.course.reference), fixture.course.id);
  const courseIdentity = await resolveCourseIdentity(client, fixture.course.reference);
  assert.deepEqual(courseIdentity, { courseId: fixture.course.id });
  assert.equal(Object.isFrozen(courseIdentity), true);
  assert.deepEqual(await resolveAssignmentRoute(client, fixture.assignment.reference), {
    kind: "assignment",
    courseId: fixture.course.id,
    assignmentId: fixture.assignment.id,
  });
  assert.equal(
    await resolveAssignmentAttemptRoute(client, fixture.assignmentAttempt.reference),
    fixture.assignmentAttempt.id,
  );
  const attemptIdentity = await resolveAssignmentAttemptIdentity(
    client,
    fixture.assignmentAttempt.reference,
  );
  assert.deepEqual(attemptIdentity, {
    courseId: fixture.course.id,
    assignmentId: fixture.assignment.id,
    assignmentAttemptId: fixture.assignmentAttempt.id,
  });
  assert.equal(Object.isFrozen(attemptIdentity), true);
  assert.equal(
    await resolveWorkspaceRoute(client, fixture.workspace.reference),
    fixture.workspace.id,
  );

  const wrongKindClient = {
    resolveNavigation: () =>
      Promise.resolve({
        kind: "assignment",
        courseId: fixture.course.id,
        assignmentId: fixture.assignment.id,
      }),
  };
  await assert.rejects(resolveCourseRoute(wrongKindClient, fixture.course.reference), {
    message: "Course Instance reference resolved to another resource",
  });
  await assert.rejects(resolveCourseRoute(client, fixture.course.id), {
    message: "Course route is incomplete",
  });
  await assert.rejects(resolveCourseIdentity(client, "CI7K3M2"), {
    message: "Course reference is invalid",
  });
  await assert.rejects(resolveAssignmentAttemptIdentity(client, "CI7K3M2Q"), {
    message: "Assignment Attempt route is incomplete",
  });
  await assert.rejects(resolveAssignmentAttemptIdentity(client, "R-01"), {
    message: "Assignment Attempt reference is invalid",
  });
  await assert.rejects(resolveAssignmentAttemptIdentity(wrongKindClient, "R-1"), {
    message: "Assignment Attempt reference resolved to another resource",
  });
});
