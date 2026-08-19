// Human navigation uses compact typed references while internal UUIDs remain API identities.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";
import {
  assignmentRouteReference,
  courseRouteReference,
  parseCourseGroupReference,
  parseAssignmentReference,
  parseCourseReference,
  parseProblemRouteReference,
  parsePublicRouteReference,
  parseRunReference,
  parseWorkspaceReference,
  problemRouteReference,
  runRouteReference,
  workspaceRouteReference,
} from "../src/navigation/public_route.ts";
import {
  resolveAssignmentRoute,
  resolveCourseRoute,
  resolveRunRoute,
  resolveWorkspaceRoute,
} from "../src/navigation/resolved_route.ts";

test("human route references are compact, typed, and bounded", () => {
  assert.equal(courseRouteReference("C-1"), "C-1");
  assert.equal(assignmentRouteReference("A-2147483647"), "A-2147483647");
  assert.equal(runRouteReference("R-30"), "R-30");
  assert.equal(workspaceRouteReference("W-40"), "W-40");
  assert.equal(problemRouteReference("7K3-M9QP"), "7K3-M9QP");

  for (const reference of ["C-1", "A-20", "R-30", "W-40"]) {
    assert.equal(parsePublicRouteReference(reference), reference);
  }
  for (const [parser, prefix] of [
    [parseCourseReference, "C"],
    [parseAssignmentReference, "A"],
    [parseRunReference, "R"],
    [parseWorkspaceReference, "W"],
    [parseCourseGroupReference, "G"],
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
  assert.equal(parseProblemRouteReference("7k3m9qp"), "7K3-M9QP");
  assert.equal(parseProblemRouteReference("OI0-001x"), "010-001X");
  assert.equal(parseProblemRouteReference("P-50-v3"), null);
  assert.equal(parseProblemRouteReference("7K3-M9QU"), null);
});

test("route resolution recovers protected API identities without weakening reference kinds", async () => {
  const client = createMockApiClient({ workspaceAuthoring: true });
  const fixture = publishedProblemFixture;

  assert.equal(await resolveCourseRoute(client, fixture.course.reference), fixture.course.id);
  assert.deepEqual(await resolveAssignmentRoute(client, fixture.assignment.reference), {
    kind: "assignment",
    courseId: fixture.course.id,
    assignmentId: fixture.assignment.id,
  });
  assert.equal(await resolveRunRoute(client, fixture.runs[0].reference), fixture.runs[0].id);
  const draft = (await client.listWorkspaceDrafts()).items[0];
  assert.notEqual(draft, undefined);
  assert.equal(await resolveWorkspaceRoute(client, draft.reference), draft.workspace);

  const wrongKindClient = {
    resolveNavigation: () =>
      Promise.resolve({
        kind: "assignment",
        courseId: fixture.course.id,
        assignmentId: fixture.assignment.id,
      }),
  };
  await assert.rejects(resolveCourseRoute(wrongKindClient, fixture.course.reference), {
    message: "Course reference resolved to another resource",
  });
  await assert.rejects(resolveCourseRoute(client, fixture.course.id), {
    message: "Course route is incomplete",
  });
});
