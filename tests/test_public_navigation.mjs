// Human navigation uses compact typed references while internal UUIDs remain API identities.

import assert from "node:assert/strict";
import test from "node:test";

import {
  assignmentRouteReference,
  courseRouteReference,
  parseAssignmentReference,
  parseCourseReference,
  parseQuestionRouteReference,
  parsePublicRouteReference,
  parseAssignmentAttemptReference,
  parseWorkspaceReference,
  questionRouteReference,
  assignmentAttemptRouteReference,
  authoringWorkspaceRouteReference,
} from "../src/navigation/public_route.ts";
import {
  resolveAssignmentRoute,
  resolveCourseRoute,
  resolveAssignmentAttemptRoute,
  resolveWorkspaceRoute,
} from "../src/navigation/resolved_route.ts";
import { isAssignmentReference, isCourseReference } from "./support/public_references.ts";

test("human route references are compact, typed, and bounded", () => {
  assert.equal(courseRouteReference("C-1"), "C-1");
  assert.equal(assignmentRouteReference("A-2147483647"), "A-2147483647");
  assert.equal(assignmentAttemptRouteReference("R-30"), "R-30");
  assert.equal(authoringWorkspaceRouteReference("W-40"), "W-40");
  assert.equal(questionRouteReference("7K3-M9QP"), "7K3-M9QP");
  assert.equal(isCourseReference("C-1"), true);
  assert.equal(isCourseReference("A-1"), false);
  assert.equal(isAssignmentReference("A-1"), true);
  assert.equal(isAssignmentReference("C-1"), false);

  for (const reference of ["C-1", "A-20", "R-30", "W-40"]) {
    assert.equal(parsePublicRouteReference(reference), reference);
  }
  for (const [parser, prefix] of [
    [parseCourseReference, "C"],
    [parseAssignmentReference, "A"],
    [parseAssignmentAttemptReference, "R"],
    [parseWorkspaceReference, "W"],
  ]) {
    assert.equal(parser(`${prefix}-1`), `${prefix}-1`);
    for (const rejected of [
      `${prefix}-0`,
      `${prefix}-01`,
      `${prefix}-2147483648`,
      "X-1",
      "0198e000-0000-7000-8000-000000000001",
    ]) {
      assert.equal(parser(rejected), null);
    }
  }
  assert.equal(parseQuestionRouteReference("7k3m9qp"), "7K3-M9QP");
  assert.equal(parseQuestionRouteReference("OI0-001x"), "010-001X");
  assert.equal(parseQuestionRouteReference("P-50-v3"), null);
  assert.equal(parseQuestionRouteReference("7K3-M9QU"), null);
});

test("route resolution recovers protected API identities without weakening reference kinds", async () => {
  const fixture = {
    course: { reference: "C-1", id: "course-id" },
    assignment: { reference: "A-1", id: "assignment-id" },
    run: { reference: "R-1", id: "run-id" },
    workspace: { reference: "W-1", id: "workspace-id" },
  };
  const client = {
    resolveNavigation: async (reference) => {
      const values = {
        "C-1": { kind: "course", courseId: fixture.course.id },
        "A-1": {
          kind: "assignment",
          courseId: fixture.course.id,
          assignmentId: fixture.assignment.id,
        },
        "R-1": { kind: "assignmentAttempt", assignmentAttemptId: fixture.run.id },
        "W-1": { kind: "workspace", workspaceId: fixture.workspace.id },
      };
      return values[reference];
    },
  };

  assert.equal(await resolveCourseRoute(client, fixture.course.reference), fixture.course.id);
  assert.deepEqual(await resolveAssignmentRoute(client, fixture.assignment.reference), {
    kind: "assignment",
    courseId: fixture.course.id,
    assignmentId: fixture.assignment.id,
  });
  assert.equal(await resolveAssignmentAttemptRoute(client, fixture.run.reference), fixture.run.id);
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
});
