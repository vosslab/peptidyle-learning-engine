// Human navigation uses compact typed references while internal UUIDs remain API identities.

import assert from "node:assert/strict";
import test from "node:test";

import {
  assessmentRouteReference,
  courseInstanceRouteReference,
  parseAssessmentReference,
  parseCourseInstanceReference,
  parseQuestionRouteReference,
  parsePublicRouteReference,
  parseAssessmentAttemptReference,
  parseAuthoringWorkspaceReference,
  parseDraftQuestionReference,
  questionRouteReference,
  assessmentAttemptRouteReference,
  authoringWorkspaceRouteReference,
  draftQuestionRouteReference,
} from "../src/navigation/public_route.ts";
import {
  resolveAssessmentRoute,
  resolveAssessmentAttemptRoute,
  resolveAssessmentAttemptIdentity,
  resolveWorkspaceRoute,
} from "../src/navigation/resolved_route.ts";

test("human route references are compact, typed, and bounded", () => {
  assert.equal(courseInstanceRouteReference("CI7K3M2Q"), "CI7K3M2Q");
  assert.equal(assessmentRouteReference("A9D2RX5"), "A9D2RX5");
  assert.equal(assessmentAttemptRouteReference("R-30"), "R-30");
  assert.equal(authoringWorkspaceRouteReference("W-40"), "W-40");
  assert.equal(draftQuestionRouteReference("D-50"), "D-50");
  assert.equal(questionRouteReference("7K3M-X9QP"), "7K3M-X9QP");

  for (const reference of ["CI7K3M2Q", "A9D2RX5", "R-30", "W-40", "D-50"]) {
    assert.equal(parsePublicRouteReference(reference), reference);
  }
  for (const [parser, valid, rejected] of [
    [parseCourseInstanceReference, "CI7K3M2Q", ["C-1", "CI7K3M2", "CI7K3M2I"]],
    [parseAssessmentReference, "A9D2RX5", ["A-1", "A9D2RX", "A9D2RXI"]],
    [parseAssessmentAttemptReference, "R"],
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
    courseId: "course-id",
    assessment: { reference: "A9D2RX5", id: "assessment-id" },
    assessmentAttempt: { reference: "R-1", id: "assessment-attempt-id" },
    workspace: { reference: "W-1", id: "workspace-id" },
  };
  const client = {
    resolveNavigation: async (reference) => {
      const values = {
        A9D2RX5: {
          kind: "assessment",
          courseId: fixture.courseId,
          assessmentId: fixture.assessment.id,
        },
        "R-1": {
          kind: "assessmentAttempt",
          courseId: fixture.courseId,
          assessmentId: fixture.assessment.id,
          studentRecordId: "student-record-id",
          assessmentAttemptId: fixture.assessmentAttempt.id,
        },
        "W-1": { kind: "workspace", workspaceId: fixture.workspace.id },
      };
      return values[reference];
    },
  };

  assert.deepEqual(await resolveAssessmentRoute(client, fixture.assessment.reference), {
    kind: "assessment",
    courseId: fixture.courseId,
    assessmentId: fixture.assessment.id,
  });
  assert.equal(
    await resolveAssessmentAttemptRoute(client, fixture.assessmentAttempt.reference),
    fixture.assessmentAttempt.id,
  );
  const attemptIdentity = await resolveAssessmentAttemptIdentity(
    client,
    fixture.assessmentAttempt.reference,
  );
  assert.deepEqual(attemptIdentity, {
    courseId: fixture.courseId,
    assessmentId: fixture.assessment.id,
    assessmentAttemptId: fixture.assessmentAttempt.id,
  });
  assert.equal(Object.isFrozen(attemptIdentity), true);
  assert.equal(
    await resolveWorkspaceRoute(client, fixture.workspace.reference),
    fixture.workspace.id,
  );

  const wrongKindClient = {
    resolveNavigation: () =>
      Promise.resolve({
        kind: "workspace",
        workspaceId: fixture.workspace.id,
      }),
  };
  await assert.rejects(resolveAssessmentRoute(wrongKindClient, fixture.assessment.reference), {
    message: "Assessment reference resolved to another resource",
  });
  await assert.rejects(resolveAssessmentAttemptIdentity(client, "CI7K3M2Q"), {
    message: "Assessment Attempt route is incomplete",
  });
  await assert.rejects(resolveAssessmentAttemptIdentity(client, "R-01"), {
    message: "Assessment Attempt reference is invalid",
  });
  await assert.rejects(resolveAssessmentAttemptIdentity(wrongKindClient, "R-1"), {
    message: "Assessment Attempt reference resolved to another resource",
  });
});
