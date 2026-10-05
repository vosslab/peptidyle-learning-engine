// Human navigation uses canonical typed public IDs while internal UUIDs remain API identities.

import assert from "node:assert/strict";
import test from "node:test";

import {
  assessmentRouteId,
  blueprintCourseRouteId,
  courseInstanceRouteId,
  parseAssessmentId,
  parseBlueprintCourseId,
  parseCourseInstanceId,
  parseQuestionRouteId,
  parsePublicRouteId,
  parseAssessmentAttemptId,
  parseAuthoringWorkspaceId,
  parseDraftQuestionId,
  questionRouteId,
  assessmentAttemptRouteId,
  authoringWorkspaceRouteId,
  draftQuestionRouteId,
} from "../src/navigation/public_route.ts";
import { routeParams, routeScopeKey } from "../src/navigation/route_params.ts";
import {
  resolveAssessmentRoute,
  resolveAssessmentAttemptRoute,
  resolveAssessmentAttemptIdentity,
  resolveWorkspaceRoute,
} from "../src/navigation/resolved_route.ts";
import { ROUTE_CONTRACT } from "../src/route_contract.ts";
import {
  normalizeHumanEnteredPublicId,
  normalizeHumanEnteredQuestionId,
} from "../src/question_id.ts";

test("human route IDs are canonical, typed, and bounded", () => {
  assert.equal(courseInstanceRouteId("CIABCDEFGS"), "CIABCDEFGS");
  assert.equal(assessmentRouteId("AABCDEFG8"), "AABCDEFG8");
  assert.equal(blueprintCourseRouteId("BPABCDEFGJ"), "BPABCDEFGJ");
  assert.equal(parseBlueprintCourseId("BPABCDEFGJ"), "BPABCDEFGJ");
  assert.equal(parseBlueprintCourseId("BPABCDEFG"), null);
  assert.equal(parseBlueprintCourseId("CIABCDEFGS"), null);
  assert.equal(
    assessmentAttemptRouteId("00000000-0000-0000-0000-00000000001e"),
    "00000000-0000-0000-0000-00000000001e",
  );
  assert.equal(
    authoringWorkspaceRouteId("00000000-0000-0000-0000-00000000000a"),
    "00000000-0000-0000-0000-00000000000a",
  );
  assert.equal(
    draftQuestionRouteId("0198e000-0000-7000-8000-000000000001"),
    "0198e000-0000-7000-8000-000000000001",
  );
  assert.equal(questionRouteId("7K3M-79QP"), "7K3M-79QP");

  for (const id of ["CIABCDEFGS", "AABCDEFG8", "00000000-0000-0000-0000-00000000001e"]) {
    assert.equal(parsePublicRouteId(id), id);
  }
  assert.equal(parsePublicRouteId("W-40"), null);
  assert.equal(parseAuthoringWorkspaceId("W-40"), null);
  assert.equal(
    parseAuthoringWorkspaceId("00000000-0000-0000-0000-00000000000a"),
    "00000000-0000-0000-0000-00000000000a",
  );
  for (const [parser, valid, rejected] of [
    [parseCourseInstanceId, "CIABCDEFGS", ["C-1", "CIABCDEFG", "CIABCDEFGT"]],
    [parseAssessmentId, "AABCDEFG8", ["A-1", "AABCDEFG", "AABCDEFG9"]],
    [parseAssessmentAttemptId, "00000000-0000-0000-0000-00000000001e", ["R-1", "R-30", "X-1"]],
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
  assert.equal(parseQuestionRouteId("7K3M-79QP"), "7K3M-79QP");
  assert.equal(parseQuestionRouteId("7k3m79qp"), null);
  assert.equal(parseQuestionRouteId("7K3M79QP"), null);
  assert.equal(parseQuestionRouteId("7K3M-89QP"), null);
  assert.equal(normalizeHumanEnteredQuestionId("7k3m79qp"), "7K3M-79QP");
  assert.equal(normalizeHumanEnteredQuestionId("O1OO-raIb"), "0100-RA1B");
  assert.equal(normalizeHumanEnteredQuestionId(" 7K3M79QP"), null);
  assert.equal(normalizeHumanEnteredPublicId("courseInstance", "ciabcdefgs"), "CIABCDEFGS");
  assert.equal(normalizeHumanEnteredPublicId("assessment", "aabcdefg8"), "AABCDEFG8");
  assert.equal(normalizeHumanEnteredPublicId("courseInstance", "CIABCDEFGT"), null);
  assert.equal(parseQuestionRouteId("P-50-v3"), null);
  assert.equal(parseQuestionRouteId("7K3-M9QU"), null);
});

const UUID_ROUTE_ID = "00000000-0000-0000-0000-00000000001e";
const CANONICAL_ROUTE_IDS = {
  courseInstanceId: "CIABCDEFGS",
  assessmentId: "AABCDEFG8",
  questionId: "7K3M-79QP",
  blueprintCourseId: "BPABCDEFGJ",
  assessmentAttemptId: UUID_ROUTE_ID,
  draftQuestionId: UUID_ROUTE_ID,
  proposalId: UUID_ROUTE_ID,
  membershipId: UUID_ROUTE_ID,
  accountId: "U00000009",
};

function fillRoute(path, overrides = {}) {
  return path.replace(/:([A-Za-z0-9]+)/g, (_, name) => {
    const value = overrides[name] ?? CANONICAL_ROUTE_IDS[name];
    assert.equal(typeof value, "string", name);
    return value;
  });
}

test("objects without a public ID use their UUID in routes and reject a secondary Id", () => {
  const seen = new Map();
  for (const route of ROUTE_CONTRACT) {
    const names = [...route.path.matchAll(/:([A-Za-z0-9]+)/g)].map((match) => match[1]);
    assert.equal(new Set(names).size, names.length, route.path);
    if (names.length === 0) continue;
    const pathname = fillRoute(route.path);
    const params = routeParams(route, pathname);
    assert.ok(params);
    assert.deepEqual(Object.keys(params).sort(), [...names].sort());
    const routeState =
      route.id === "assessmentAttempt" || route.id === "assessmentAttemptSummary"
        ? { assessmentAttemptId: UUID_ROUTE_ID }
        : undefined;
    assert.notEqual(routeScopeKey(pathname, routeState).kind, "invalid", route.path);
    for (const name of names) {
      assert.equal(params[name], CANONICAL_ROUTE_IDS[name]);
      const canonical = CANONICAL_ROUTE_IDS[name];
      assert.equal(
        routeScopeKey(fillRoute(route.path, { [name]: `${canonical}-2` })).kind,
        "invalid",
        `${route.path} ${name}`,
      );
      const otherKind = canonical === UUID_ROUTE_ID ? "CIABCDEFGS" : UUID_ROUTE_ID;
      assert.equal(
        routeScopeKey(fillRoute(route.path, { [name]: otherKind })).kind,
        "invalid",
        `${route.path} ${name}`,
      );
      if (canonical === UUID_ROUTE_ID) {
        for (const secondary of ["R-1", "D-50", "W-40"]) {
          assert.equal(
            routeScopeKey(fillRoute(route.path, { [name]: secondary })).kind,
            "invalid",
            `${route.path} ${secondary}`,
          );
        }
      }
      seen.set(name, canonical === UUID_ROUTE_ID ? "uuid" : "public");
    }
  }
  const uuidNames = [...seen.entries()]
    .filter(([, kind]) => kind === "uuid")
    .map(([name]) => name)
    .sort();
  assert.deepEqual(uuidNames, ["draftQuestionId", "proposalId"]);
  const attempt = routeScopeKey("/courses/CIABCDEFGS/attempt", {
    assessmentAttemptId: UUID_ROUTE_ID,
  });
  assert.deepEqual(attempt, {
    kind: "assessmentAttempt",
    courseInstanceId: "CIABCDEFGS",
    assessmentAttemptId: UUID_ROUTE_ID,
  });
  assert.deepEqual(routeScopeKey("/courses/CIABCDEFGS/attempt"), {
    kind: "invalid",
    scope: "assessmentAttempt",
  });
  assert.equal(routeScopeKey(`/assessment-attempts/${UUID_ROUTE_ID}`).kind, "invalid");
  assert.equal(Object.hasOwn(attempt, "attemptNumber"), false);
  assert.equal(Object.hasOwn(attempt, "attemptId"), false);
});

test("route resolution recovers protected API identities without weakening ID kinds", async () => {
  const fixture = {
    courseInstanceId: "course-id",
    assessment: { assessmentId: "AABCDEFG8", id: "assessment-id" },
    assessmentAttempt: {
      id: "00000000-0000-0000-0000-000000000001",
    },
    workspace: { id: "00000000-0000-0000-0000-00000000000a" },
  };
  const client = {
    resolveNavigation: async (id) => {
      const values = {
        AABCDEFG8: {
          kind: "assessment",
          courseInstanceId: fixture.courseInstanceId,
          assessmentId: fixture.assessment.id,
        },
        "00000000-0000-0000-0000-000000000001": {
          kind: "assessmentAttempt",
          courseInstanceId: fixture.courseInstanceId,
          assessmentId: fixture.assessment.id,
          studentRecordId: "student-record-id",
          assessmentAttemptId: fixture.assessmentAttempt.id,
        },
        "00000000-0000-0000-0000-00000000000a": {
          kind: "workspace",
          workspaceId: fixture.workspace.id,
        },
      };
      return values[id];
    },
  };

  assert.deepEqual(await resolveAssessmentRoute(client, fixture.assessment.assessmentId), {
    kind: "assessment",
    courseInstanceId: fixture.courseInstanceId,
    assessmentId: fixture.assessment.id,
  });
  assert.equal(
    await resolveAssessmentAttemptRoute(client, fixture.assessmentAttempt.id),
    fixture.assessmentAttempt.id,
  );
  const attemptIdentity = await resolveAssessmentAttemptIdentity(
    client,
    fixture.assessmentAttempt.id,
  );
  assert.deepEqual(attemptIdentity, {
    courseInstanceId: fixture.courseInstanceId,
    assessmentId: fixture.assessment.id,
    assessmentAttemptId: fixture.assessmentAttempt.id,
  });
  assert.equal(Object.isFrozen(attemptIdentity), true);
  assert.equal(await resolveWorkspaceRoute(client, fixture.workspace.id), fixture.workspace.id);

  const wrongKindClient = {
    resolveNavigation: () =>
      Promise.resolve({
        kind: "workspace",
        workspaceId: fixture.workspace.id,
      }),
  };
  await assert.rejects(resolveAssessmentRoute(wrongKindClient, fixture.assessment.assessmentId), {
    message: "Assessment ID resolved to another resource",
  });
  await assert.rejects(resolveAssessmentAttemptIdentity(client, undefined), {
    message: "Assessment Attempt route is incomplete",
  });
  await assert.rejects(resolveAssessmentAttemptIdentity(client, "CIABCDEFGS"), {
    message: "Assessment Attempt ID is invalid",
  });
  await assert.rejects(
    resolveAssessmentAttemptIdentity(wrongKindClient, fixture.assessmentAttempt.id),
    {
      message: "Assessment Attempt ID resolved to another resource",
    },
  );
});
