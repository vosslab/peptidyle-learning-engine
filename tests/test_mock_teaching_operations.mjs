import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";

const COURSE = publishedProblemFixture.course.id;
const ASSIGNMENT = publishedProblemFixture.assignment.id;
const STUDENT = "M-1";

test("teaching-operations mock mode is a fixed instructor fixture and defaults remain student", async () => {
  const student = createMockApiClient();
  const instructor = createMockApiClient({ teachingOperationsAuthoring: true });
  const invitedAccount = createMockApiClient({ teachingAccountPendingInvitation: true });

  assert.deepEqual((await student.getSession()).user.roles, ["student"]);
  assert.equal((await student.listCourses()).items[0]?.role, "student");
  assert.deepEqual((await instructor.getSession()).user.roles, ["instructor"]);
  assert.equal((await instructor.listCourses()).items[0]?.role, "instructor");
  assert.equal((await instructor.getCourse(COURSE)).role, "instructor");
  assert.deepEqual((await invitedAccount.getSession()).user.roles, ["student"]);
  assert.equal((await invitedAccount.getSession()).user.displayName, "Invited Colleague");
  const pending = await invitedAccount.listPendingCoInstructorInvitations();
  assert.equal(pending.invitations.length, 1);
  assert.equal(pending.invitations[0]?.courseLabel, "Demo course");
  assert.doesNotMatch(JSON.stringify(pending), /U-5|email|uuid/u);
});

function patch() {
  return {
    mode: "override",
    patch: {
      availableAt: { kind: "inherit" },
      dueAt: { kind: "set", value: "2026-08-24T10:00:00.000" },
      closesAt: { kind: "unrestricted" },
      timeLimitSeconds: { kind: "set", value: 3600 },
      attemptLimit: { kind: "set", value: 2 },
    },
  };
}

test("mock teaching groups retain policy warnings and revision-CAS behavior", async () => {
  const client = createMockApiClient();
  const groups = await client.listCourseGroups(COURSE);
  assert.equal(groups.groups.length, 3);
  assert.deepEqual(await client.getCourseGroupMembershipWarnings(COURSE), {
    disposition: "allowed",
    warningCount: 0,
  });

  const created = await client.createCourseGroup(COURSE, {
    title: "Section B",
    purpose: "section",
    members: [STUDENT],
  });
  assert.equal(created.reference, "G-4");
  assert.deepEqual(await client.getCourseGroupMembershipWarnings(COURSE), {
    disposition: "allowedWithWarning",
    warningCount: 1,
  });
  const policy = await client.getCourseGroupPurposePolicy(COURSE, "section");
  const updatedPolicy = await client.updateCourseGroupPurposePolicy(
    COURSE,
    "section",
    { multipleMembership: "allow" },
    policy.revision,
  );
  assert.equal(updatedPolicy.multipleMembership, "allow");
  assert.throws(
    () =>
      client.updateCourseGroup(
        COURSE,
        "G-4",
        { title: "Section B", purpose: "section", members: [] },
        "1",
      ),
    { status: 412 },
  );
});

test("mock teaching M2-M4 mutations advance a generated revision response and previews stay sealed", async () => {
  const client = createMockApiClient();
  const offset = await client.putGroupScheduleOffset(
    COURSE,
    ASSIGNMENT,
    "G-2",
    { offsetSeconds: 900 },
    "1",
  );
  const accommodation = await client.putGroupAccommodation(
    COURSE,
    ASSIGNMENT,
    "G-3",
    patch(),
    offset.revision,
  );
  const individual = await client.putIndividualPolicyException(
    COURSE,
    ASSIGNMENT,
    STUDENT,
    patch(),
    accommodation.revision,
  );
  assert.equal(individual.revision, "4");
  const allowed = await client.getTeachingPreview(COURSE, ASSIGNMENT, STUDENT);
  assert.equal(allowed.entitlement, "allowed");
  if (allowed.entitlement === "allowed") {
    assert.equal(allowed.timeZone, "America/Chicago");
    assert.equal(allowed.dueAt.value, "2026-08-24T10:00:00.000");
    assert.equal(allowed.dueAt.source.kind, "membership");
    assert.doesNotMatch(JSON.stringify(allowed), /1_756|1756/u);
  }
  assert.deepEqual(await client.getTeachingPreview(COURSE, ASSIGNMENT, "M-2"), {
    entitlement: "denied",
    reason: "notEntitled",
  });
});

test("mock teaching one-time modifier conflict advances the revision that assignment reload reads", async () => {
  const client = createMockApiClient({ teachingModifierConflictOnce: true });
  const before = await client.getAssignmentEditor(ASSIGNMENT);
  assert.equal(before.revision, '"1"');
  assert.throws(
    () =>
      client.putGroupScheduleOffset(
        COURSE,
        ASSIGNMENT,
        "G-1",
        { offsetSeconds: 900 },
        before.revision,
      ),
    { status: 412 },
  );
  const reloaded = await client.getAssignmentEditor(ASSIGNMENT);
  assert.equal(reloaded.revision, '"2"');
  const accepted = await client.putGroupScheduleOffset(
    COURSE,
    ASSIGNMENT,
    "G-1",
    { offsetSeconds: 900 },
    reloaded.revision,
  );
  assert.equal(accepted.revision, "3");
});

test("mock teaching target pickers are bounded, display-only, and preserve CAS state", async () => {
  const client = createMockApiClient();
  assert.throws(() => client.searchCourseCoInstructorTargets(COURSE, "T"), /2 to 100/u);
  const first = await client.searchCourseCoInstructorTargets(COURSE, "TaY", undefined, 1);
  assert.deepEqual(
    first.targets.map((target) => target.account.display),
    ["Taylor Mentor"],
  );
  assert.equal(first.nextCursor, "1");
  const secondTarget = await client.searchCourseCoInstructorTargets(COURSE, "TaY", "1", 1);
  assert.deepEqual(
    secondTarget.targets.map((target) => target.account.display),
    ["Taylor Reserve"],
  );
  assert.equal(secondTarget.nextCursor, null);
  assert.doesNotMatch(JSON.stringify(first), /email|uuid/u);

  const approval = await client.approveInstructorAccount("U-2");
  const candidates = await client.searchCourseCoInstructorTargets(COURSE, "demo");
  assert.equal(candidates.targets[0]?.account.display, "Demo co-instructor");
  const invitation = await client.createCourseCoInstructorInvitation(COURSE, { target: "U-2" });
  assert.deepEqual((await client.searchCourseCoInstructorTargets(COURSE, "demo")).targets, []);
  const pending = await client.listPendingCoInstructorInvitations();
  await client.respondToCoInstructorInvitation(
    invitation,
    { action: "accept" },
    pending.invitations[0].revision,
  );
  assert.deepEqual((await client.searchCourseCoInstructorTargets(COURSE, "demo")).targets, []);
  assert.throws(() => client.revokeInstructorApproval("U-2", approval.revision), { status: 412 });

  const students = await client.listCourseStudentTargets(COURSE, undefined, 1);
  assert.equal(students.students[0]?.role, "student");
  assert.equal(students.students[0]?.status, "active");
  assert.equal(students.nextCursor, "1");
  const second = await client.listCourseStudentTargets(COURSE, students.nextCursor, 1);
  assert.equal(second.students[0]?.display, "Ada Student");
  assert.equal(second.nextCursor, null);
  assert.doesNotMatch(JSON.stringify(second), /email|uuid|1756/u);
});

test("mock teaching approval, invitations, instructor removal, and retention remain deterministic", async () => {
  const client = createMockApiClient();
  const approval = await client.approveInstructorAccount("U-2");
  assert.equal(approval.state, "approved");
  const invitation = await client.createCourseCoInstructorInvitation(COURSE, { target: "U-2" });
  assert.equal(invitation, "CI-1");
  const pending = await client.listPendingCoInstructorInvitations();
  assert.equal(pending.invitations[0]?.courseLabel, "Demo course");
  await client.respondToCoInstructorInvitation(
    invitation,
    { action: "accept" },
    pending.invitations[0].revision,
  );
  const instructors = await client.listCourseInstructors(COURSE);
  assert.equal(instructors.instructors.length, 2);
  await client.removeCourseInstructor(COURSE, "M-11", {}, instructors.rosterRevision);
  const finalRoster = await client.listCourseInstructors(COURSE);
  await assert.rejects(
    client.removeCourseInstructor(COURSE, "M-10", {}, finalRoster.rosterRevision),
    { status: 409 },
  );

  const ending = await client.endCourseRetention(COURSE);
  assert.equal(ending.state, "notificationDue");
  const archived = await client.archiveCourseRetention(
    COURSE,
    { assignmentDefinitions: "retain" },
    ending.revision,
  );
  assert.equal(archived.state, "studentRecordsArchived");
  const deleted = await client.deleteCourseRetention(COURSE, archived.revision);
  assert.equal(deleted.state, "studentRecordsDeleted");
  const extended = await client.extendCourseRetention(
    COURSE,
    { additionalDays: 7 },
    deleted.revision,
  );
  assert.equal(extended.state, "notificationDue");
});
