// Human navigation uses canonical typed references while internal UUIDs remain API identities.

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
  parseDraftQuestionId,
  questionRouteReference,
  assessmentAttemptRouteReference,
  authoringWorkspaceRouteReference,
  draftQuestionRouteId,
} from "../src/navigation/public_route.ts";
import {
  resolveAssessmentRoute,
  resolveAssessmentAttemptRoute,
  resolveAssessmentAttemptIdentity,
  resolveWorkspaceRoute,
} from "../src/navigation/resolved_route.ts";
import { normalizeHumanEnteredQuestionId } from "../src/question_id.ts";

test("human route references are canonical, typed, and bounded", () => {
  assert.equal(courseInstanceRouteReference("CIABCDEFGS"), "CIABCDEFGS");
  assert.equal(assessmentRouteReference("AABCDEFG8"), "AABCDEFG8");
  assert.equal(
    assessmentAttemptRouteReference("00000000-0000-0000-0000-00000000001e"),
    "00000000-0000-0000-0000-00000000001e",
  );
  assert.equal(authoringWorkspaceRouteReference("W-40"), "W-40");
  assert.equal(
    draftQuestionRouteId("0198e000-0000-7000-8000-000000000001"),
    "0198e000-0000-7000-8000-000000000001",
  );
  assert.equal(questionRouteReference("7K3M-79QP"), "7K3M-79QP");

  for (const reference of [
    "CIABCDEFGS",
    "AABCDEFG8",
    "00000000-0000-0000-0000-00000000001e",
    "W-40",
  ]) {
    assert.equal(parsePublicRouteReference(reference), reference);
  }
  for (const [parser, valid, rejected] of [
    [parseCourseInstanceReference, "CIABCDEFGS", ["C-1", "CIABCDEFG", "CIABCDEFGT"]],
    [parseAssessmentReference, "AABCDEFG8", ["A-1", "AABCDEFG", "AABCDEFG9"]],
    [
      parseAssessmentAttemptReference,
      "00000000-0000-0000-0000-00000000001e",
      ["R-1", "R-30", "X-1"],
    ],
    [parseAuthoringWorkspaceReference, "W"],
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
  assert.equal(
    parseDraftQuestionId("0198e000-0000-7000-8000-000000000001"),
    "0198e000-0000-7000-8000-000000000001",
  );
  assert.equal(parseDraftQuestionId("0198E000-0000-7000-8000-000000000001"), null);
  assert.equal(parseDraftQuestionId("D-50"), null);
  assert.equal(parseQuestionRouteReference("7K3M-79QP"), "7K3M-79QP");
  assert.equal(parseQuestionRouteReference("7k3m79qp"), null);
  assert.equal(parseQuestionRouteReference("7K3M79QP"), null);
  assert.equal(parseQuestionRouteReference("7K3M-89QP"), null);
  assert.equal(normalizeHumanEnteredQuestionId("7k3m79qp"), "7K3M-79QP");
  assert.equal(normalizeHumanEnteredQuestionId("O1OO-raIb"), "0100-RA1B");
  assert.equal(normalizeHumanEnteredQuestionId(" 7K3M79QP"), null);
  assert.equal(parseQuestionRouteReference("P-50-v3"), null);
  assert.equal(parseQuestionRouteReference("7K3-M9QU"), null);
});

test("route resolution recovers protected API identities without weakening reference kinds", async () => {
  const fixture = {
    courseId: "course-id",
    assessment: { reference: "AABCDEFG8", id: "assessment-id" },
    assessmentAttempt: {
      reference: "00000000-0000-0000-0000-000000000001",
      id: "00000000-0000-0000-0000-000000000001",
    },
    workspace: { reference: "W-1", id: "workspace-id" },
  };
  const client = {
    resolveNavigation: async (reference) => {
      const values = {
        AABCDEFG8: {
          kind: "assessment",
          courseId: fixture.courseId,
          assessmentId: fixture.assessment.id,
        },
        "00000000-0000-0000-0000-000000000001": {
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
  await assert.rejects(resolveAssessmentAttemptIdentity(client, undefined), {
    message: "Assessment Attempt route is incomplete",
  });
  await assert.rejects(resolveAssessmentAttemptIdentity(client, "CIABCDEFGS"), {
    message: "Assessment Attempt reference is invalid",
  });
  await assert.rejects(
    resolveAssessmentAttemptIdentity(wrongKindClient, fixture.assessmentAttempt.reference),
    {
      message: "Assessment Attempt reference resolved to another resource",
    },
  );
});
