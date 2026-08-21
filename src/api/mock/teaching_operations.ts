/* eslint-disable @typescript-eslint/explicit-function-return-type -- ApiClient supplies the public signatures. */
// Deterministic browser-demo state for the revisioned WP-PROF-T2 teaching operations.

import { publishedProblemFixture } from "../../../generated/fixtures/published_problem";
import type { ApiClient } from "../client";
import { ApiProtocolError, ApiRequestError } from "../http_client";
import {
  decodeAccountApprovalView,
  decodeAssignmentPolicyPatchUpdateRequest,
  decodeCoInstructorInvitationCreateRequest,
  decodeCoInstructorInvitationTerminalActionRequest,
  decodeCoInstructorTargetSearchPage,
  decodeCourseCoInstructorInvitationsPage,
  decodeCourseGroupCreateRequest,
  decodeCourseGroupDetailView,
  decodeCourseGroupListPage,
  decodeCourseGroupMembershipWarningView,
  decodeCourseGroupPurposePolicyUpdateRequest,
  decodeCourseGroupPurposePolicyView,
  decodeCourseGroupSummary,
  decodeCourseGroupUpdateRequest,
  decodeCourseStudentMembershipsPage,
  decodeGroupScheduleOffsetUpdateRequest,
  decodeIndividualPolicyPatchUpdateRequest,
  decodeInstructorMembershipRemovalRequest,
  decodeInstructorMembershipsPage,
  decodePendingCoInstructorInvitationsPage,
  decodeRetentionActionResponse,
  decodeRetentionArchiveRequest,
  decodeRetentionExtendRequest,
  decodeRetentionReadView,
  decodeTeachingOperationRevisionResponse,
  decodeTeachingPreviewView,
} from "../decoders";

type TeachingOperationsClient = Pick<
  ApiClient,
  | "listCourseGroups"
  | "getCourseGroup"
  | "createCourseGroup"
  | "updateCourseGroup"
  | "deleteCourseGroup"
  | "getCourseGroupPurposePolicy"
  | "updateCourseGroupPurposePolicy"
  | "getCourseGroupMembershipWarnings"
  | "listCourseStudentTargets"
  | "putGroupScheduleOffset"
  | "deleteGroupScheduleOffset"
  | "putGroupAccommodation"
  | "deleteGroupAccommodation"
  | "putIndividualPolicyException"
  | "deleteIndividualPolicyException"
  | "getTeachingPreview"
  | "approveInstructorAccount"
  | "revokeInstructorApproval"
  | "listCourseCoInstructorInvitations"
  | "searchCourseCoInstructorTargets"
  | "createCourseCoInstructorInvitation"
  | "revokeCourseCoInstructorInvitation"
  | "listPendingCoInstructorInvitations"
  | "respondToCoInstructorInvitation"
  | "listCourseInstructors"
  | "removeCourseInstructor"
  | "getCourseRetention"
  | "endCourseRetention"
  | "archiveCourseRetention"
  | "deleteCourseRetention"
  | "extendCourseRetention"
>;

const FIXTURE_COURSE = publishedProblemFixture.course.id;
const ACTIVE_STUDENT = "M-1";
const DEMO_ACCOUNT = "U-2";
const PENDING_ACCOUNT = "U-5";
const FIXTURE_TIME = 1_787_245_200_000;
const purposes = ["section", "lab", "cohort", "accommodation", "work"] as const;
const COURSE_TIME_ZONE = "America/Chicago";
const LOCAL_AVAILABLE_AT = "2026-08-24T09:00:00.000";
const LOCAL_DUE_AT = "2026-08-24T10:00:00.000";
const LOCAL_CLOSES_AT = "2026-08-24T11:00:00.000";

function clone<T>(value: T): T {
  return structuredClone(value);
}

export interface MockTeachingOperationsConfig {
  /** Focused browser fixture: make exactly one modifier write report a stale revision. */
  readonly modifierConflictOnce?: boolean;
  /** Focused browser fixture: make exactly one retention write report a stale revision. */
  readonly retentionConflictOnce?: boolean;
  /** Focused browser fixture: make one group deletion report a referenced-group refusal. */
  readonly groupDeleteConflictOnce?: boolean;
  /** Focused browser fixture: make one archive action report permission refusal. */
  readonly retentionArchiveForbiddenOnce?: boolean;
  /** Focused browser fixture: make one delete action report temporary unavailability. */
  readonly retentionDeleteUnavailableOnce?: boolean;
  /** Fixed account-owned pending invitation used only by the role-neutral account demo. */
  readonly accountPendingInvitation?: boolean;
  /** Keeps the assignment-editor fixture revision aligned with teaching-operation CAS changes. */
  readonly onRevisionChange?: (revision: string) => void;
}

export function createMockTeachingOperationsClient(
  config: MockTeachingOperationsConfig = {},
): TeachingOperationsClient {
  let revision = 1;
  let nextGroup = 4;
  let nextInvitation = config.accountPendingInvitation === true ? 2 : 1;
  let approved = false;
  let retentionState:
    "active" | "notificationDue" | "studentRecordsArchived" | "studentRecordsDeleted" = "active";
  let retentionDisposition: "retain" | "delete" = "retain";
  let retentionRevision = 1;
  const groups = new Map<
    string,
    { title: string; purpose: (typeof purposes)[number]; members: string[] }
  >([
    ["G-1", { title: "Section A", purpose: "section", members: [ACTIVE_STUDENT] }],
    ["G-2", { title: "Thursday lab", purpose: "lab", members: [ACTIVE_STUDENT] }],
    [
      "G-3",
      { title: "Accessibility extensions", purpose: "accommodation", members: [ACTIVE_STUDENT] },
    ],
  ]);
  const policies = new Map<(typeof purposes)[number], "allow" | "warn">(
    purposes.map((purpose) => [purpose, purpose === "section" ? "warn" : "allow"]),
  );
  const policyRevisions = new Map<(typeof purposes)[number], number>(
    purposes.map((purpose) => [purpose, 1]),
  );
  const scheduleOffsets = new Set<string>();
  const groupAccommodations = new Map<
    string,
    ReturnType<typeof decodeAssignmentPolicyPatchUpdateRequest>
  >();
  const individualExceptions = new Map<
    string,
    ReturnType<typeof decodeIndividualPolicyPatchUpdateRequest>
  >();
  let modifierConflictPending = config.modifierConflictOnce === true;
  let retentionConflictPending = config.retentionConflictOnce === true;
  let groupDeleteConflictPending = config.groupDeleteConflictOnce === true;
  let retentionArchiveForbiddenPending = config.retentionArchiveForbiddenOnce === true;
  let retentionDeleteUnavailablePending = config.retentionDeleteUnavailableOnce === true;
  const invitations = new Map<
    string,
    { target: string; state: "pending" | "accepted" | "declined" | "revoked" }
  >(
    config.accountPendingInvitation === true
      ? [["CI-1", { target: PENDING_ACCOUNT, state: "pending" }]]
      : [],
  );
  const instructors = new Map<string, { account: string; display: string }>([
    ["M-10", { account: "U-1", display: "Course Instructor" }],
  ]);
  const students = [
    {
      reference: "M-1",
      display: "Demo learner",
      role: "student" as const,
      status: "active" as const,
    },
    {
      reference: "M-2",
      display: "Ada Student",
      role: "student" as const,
      status: "active" as const,
    },
    {
      reference: "M-3",
      display: "Former Student",
      role: "student" as const,
      status: "revoked" as const,
    },
  ];
  const accounts = [
    { reference: "U-1", display: "Course Instructor", approved: true },
    { reference: "U-2", display: "Demo co-instructor", approved: false },
    { reference: "U-3", display: "Taylor Mentor", approved: true },
    { reference: "U-4", display: "Taylor Reserve", approved: true },
    { reference: PENDING_ACCOUNT, display: "Invited Colleague", approved: true },
  ];

  function accountDisplay(reference: string): string {
    return (
      accounts.find((account) => account.reference === reference)?.display ?? "Unknown account"
    );
  }

  function requireCourse(courseId: string): void {
    if (courseId !== FIXTURE_COURSE)
      throw new ApiProtocolError("Mock teaching course is not found");
  }

  function requireObservedRevision(observed: string, current: number): void {
    if (observed !== String(current) && observed !== `"${current}"`)
      throw new ApiRequestError(412, "/api/mock/teaching-operations");
  }

  function requireRetentionRevision(observed: string): void {
    if (retentionConflictPending) {
      retentionConflictPending = false;
      retentionRevision += 1;
      throw new ApiRequestError(412, "/api/mock/teaching-operations");
    }
    requireObservedRevision(observed, retentionRevision);
  }

  function requireRevision(observed: string): void {
    requireObservedRevision(observed, revision);
  }

  function requireModifierRevision(observed: string): void {
    if (modifierConflictPending) {
      modifierConflictPending = false;
      advance();
      throw new ApiRequestError(412, "/api/mock/teaching-operations");
    }
    requireRevision(observed);
  }

  function advance(): string {
    revision += 1;
    const next = String(revision);
    config.onRevisionChange?.(next);
    return next;
  }

  function groupSummary(reference: string) {
    const group = groups.get(reference);
    if (group === undefined) throw new ApiProtocolError("Mock course group is not found");
    return decodeCourseGroupSummary({
      reference,
      title: group.title,
      purpose: group.purpose,
      revision: String(revision),
      memberCount: group.members.length,
    });
  }

  function mutationResponse() {
    return decodeTeachingOperationRevisionResponse({ revision: advance() });
  }

  function modifierKey(assignmentId: string, subject: string): string {
    return `${assignmentId}/${subject}`;
  }

  function page<T>(items: readonly T[], cursor: string | undefined, pageSize: number | undefined) {
    const size = pageSize ?? 100;
    if (!Number.isSafeInteger(size) || size < 1 || size > 100)
      throw new ApiProtocolError("Mock teaching page size must be an integer from 1 through 100");
    const start = cursor === undefined ? 0 : Number(cursor);
    if (
      !Number.isSafeInteger(start) ||
      start < 0 ||
      (cursor !== undefined && String(start) !== cursor)
    )
      throw new ApiProtocolError("Mock teaching cursor is not valid");
    const values = items.slice(start, start + size);
    const next = start + values.length;
    return { values, nextCursor: next < items.length ? String(next) : null };
  }

  function targetSearch(query: string): string {
    if (query.trim() !== query || Array.from(query).length < 2 || Array.from(query).length > 100)
      throw new ApiProtocolError(
        "Mock co-instructor target query must be trimmed and contain 2 to 100 characters",
      );
    return query.toLocaleLowerCase();
  }

  function allowedPreview(assignmentId: string, student: string) {
    const offsetGroups = [...groups.entries()]
      .filter(([group]) => scheduleOffsets.has(modifierKey(assignmentId, group)))
      .map(([group, value]) => ({ group, label: value.title }));
    const accommodationGroups = [...groups.entries()]
      .filter(([group]) => groupAccommodations.has(modifierKey(assignmentId, group)))
      .map(([group, value]) => ({ group, label: value.title }));
    const individual = individualExceptions.has(modifierKey(assignmentId, student));
    const firstAccommodation = accommodationGroups[0];
    const patch = individual
      ? individualExceptions.get(modifierKey(assignmentId, student))?.patch
      : firstAccommodation !== undefined
        ? groupAccommodations.get(modifierKey(assignmentId, firstAccommodation.group))?.patch
        : undefined;
    const resolveTime = (base: string, field: "availableAt" | "dueAt" | "closesAt") => {
      const update = patch?.[field];
      if (update?.kind === "set") return update.value;
      return update?.kind === "unrestricted" ? null : base;
    };
    const source = individual
      ? { kind: "membership" as const, membership: student, label: "Individual exception" }
      : accommodationGroups.length > 0
        ? { kind: "groupAccommodations" as const, groups: accommodationGroups }
        : offsetGroups.length > 0
          ? { kind: "groupScheduleOffsets" as const, groups: offsetGroups }
          : { kind: "base" as const, label: "Course policy" };
    return decodeTeachingPreviewView({
      entitlement: "allowed",
      timeZone: COURSE_TIME_ZONE,
      start: { kind: "mayStart", late: "onTime" },
      availableAt: { value: resolveTime(LOCAL_AVAILABLE_AT, "availableAt"), source },
      dueAt: { value: resolveTime(LOCAL_DUE_AT, "dueAt"), source },
      closesAt: { value: resolveTime(LOCAL_CLOSES_AT, "closesAt"), source },
      timeLimitSeconds: { value: 3600, source },
      attemptLimit: { value: 2, source },
      lateSubmission: { value: "accept", source },
      deadlineBehavior: { value: "autoSubmit", source },
    });
  }

  function retentionRead() {
    const notification =
      retentionState === "active"
        ? undefined
        : {
            intent: "extend" as const,
            createdAt: FIXTURE_TIME,
            copy: "Retention action recorded.",
          };
    return decodeRetentionReadView({
      state: retentionState,
      assignmentDefinitions: retentionDisposition,
      revision: String(retentionRevision),
      ...(notification === undefined ? {} : { notification }),
    });
  }

  return {
    listCourseGroups: (courseId) => {
      requireCourse(courseId);
      return Promise.resolve(
        decodeCourseGroupListPage({
          groups: [...groups.keys()].map(groupSummary),
          nextCursor: null,
        }),
      );
    },
    getCourseGroup: (courseId, group) => {
      requireCourse(courseId);
      const current = groups.get(group);
      if (current === undefined)
        return Promise.reject(new ApiProtocolError("Mock course group is not found"));
      return Promise.resolve(
        decodeCourseGroupDetailView({
          group: groupSummary(group),
          members: current.members.map((reference) => ({
            reference,
            display:
              students.find((student) => student.reference === reference)?.display ??
              "Demo learner",
            role: "student",
            status: "active",
          })),
          nextCursor: null,
        }),
      );
    },
    createCourseGroup: (courseId, request) => {
      requireCourse(courseId);
      const body = decodeCourseGroupCreateRequest(request, "request");
      const group = `G-${nextGroup++}`;
      groups.set(group, clone(body));
      advance();
      return Promise.resolve(groupSummary(group));
    },
    updateCourseGroup: (courseId, group, request, observed) => {
      requireCourse(courseId);
      requireRevision(observed);
      if (!groups.has(group))
        return Promise.reject(new ApiProtocolError("Mock course group is not found"));
      groups.set(group, clone(decodeCourseGroupUpdateRequest(request, "request")));
      advance();
      return Promise.resolve(groupSummary(group));
    },
    deleteCourseGroup: (courseId, group, observed) => {
      requireCourse(courseId);
      if (groupDeleteConflictPending) {
        groupDeleteConflictPending = false;
        return Promise.reject(new ApiRequestError(409, "/api/mock/teaching-operations"));
      }
      requireRevision(observed);
      if (
        scheduleOffsets.has(modifierKey(publishedProblemFixture.assignment.id, group)) ||
        groupAccommodations.has(modifierKey(publishedProblemFixture.assignment.id, group))
      )
        return Promise.reject(new ApiProtocolError("Mock course group is still referenced"));
      if (!groups.delete(group))
        return Promise.reject(new ApiProtocolError("Mock course group is not found"));
      advance();
      return Promise.resolve();
    },
    getCourseGroupPurposePolicy: (courseId, purpose) => {
      requireCourse(courseId);
      return Promise.resolve(
        decodeCourseGroupPurposePolicyView({
          purpose,
          multipleMembership: policies.get(purpose),
          revision: String(policyRevisions.get(purpose)),
        }),
      );
    },
    updateCourseGroupPurposePolicy: (courseId, purpose, request, observed) => {
      requireCourse(courseId);
      const currentRevision = policyRevisions.get(purpose) ?? 1;
      requireObservedRevision(observed, currentRevision);
      const body = decodeCourseGroupPurposePolicyUpdateRequest(request, "request");
      policies.set(purpose, body.multipleMembership);
      const nextRevision = currentRevision + 1;
      policyRevisions.set(purpose, nextRevision);
      return Promise.resolve(
        decodeCourseGroupPurposePolicyView({
          purpose,
          multipleMembership: body.multipleMembership,
          revision: String(nextRevision),
        }),
      );
    },
    getCourseGroupMembershipWarnings: (courseId) => {
      requireCourse(courseId);
      let warningCount = 0;
      for (const purpose of purposes) {
        if (policies.get(purpose) !== "warn") continue;
        const seen = new Map<string, number>();
        for (const group of groups.values())
          if (group.purpose === purpose)
            for (const member of group.members) seen.set(member, (seen.get(member) ?? 0) + 1);
        warningCount += [...seen.values()].filter((count) => count > 1).length;
      }
      return Promise.resolve(
        decodeCourseGroupMembershipWarningView({
          disposition: warningCount === 0 ? "allowed" : "allowedWithWarning",
          warningCount,
        }),
      );
    },
    listCourseStudentTargets: (courseId, cursor, pageSize) => {
      requireCourse(courseId);
      const current = page(
        students.filter((student) => student.status === "active"),
        cursor,
        pageSize,
      );
      return Promise.resolve(
        decodeCourseStudentMembershipsPage({
          students: current.values,
          nextCursor: current.nextCursor,
        }),
      );
    },
    putGroupScheduleOffset: (courseId, assignmentId, group, request, observed) => {
      requireCourse(courseId);
      requireModifierRevision(observed);
      decodeGroupScheduleOffsetUpdateRequest(request, "request");
      if (!groups.has(group))
        return Promise.reject(new ApiProtocolError("Mock course group is not found"));
      scheduleOffsets.add(modifierKey(assignmentId, group));
      return Promise.resolve(mutationResponse());
    },
    deleteGroupScheduleOffset: (courseId, assignmentId, group, observed) => {
      requireCourse(courseId);
      requireModifierRevision(observed);
      scheduleOffsets.delete(modifierKey(assignmentId, group));
      return Promise.resolve(mutationResponse());
    },
    putGroupAccommodation: (courseId, assignmentId, group, request, observed) => {
      requireCourse(courseId);
      requireModifierRevision(observed);
      const body = decodeAssignmentPolicyPatchUpdateRequest(request, "request");
      if (groups.get(group)?.purpose !== "accommodation")
        return Promise.reject(new ApiProtocolError("Mock course group is not found"));
      groupAccommodations.set(modifierKey(assignmentId, group), body);
      return Promise.resolve(mutationResponse());
    },
    deleteGroupAccommodation: (courseId, assignmentId, group, observed) => {
      requireCourse(courseId);
      requireModifierRevision(observed);
      groupAccommodations.delete(modifierKey(assignmentId, group));
      return Promise.resolve(mutationResponse());
    },
    putIndividualPolicyException: (courseId, assignmentId, student, request, observed) => {
      requireCourse(courseId);
      requireModifierRevision(observed);
      const body = decodeIndividualPolicyPatchUpdateRequest(request, "request");
      individualExceptions.set(modifierKey(assignmentId, student), body);
      return Promise.resolve(mutationResponse());
    },
    deleteIndividualPolicyException: (courseId, assignmentId, student, observed) => {
      requireCourse(courseId);
      requireModifierRevision(observed);
      individualExceptions.delete(modifierKey(assignmentId, student));
      return Promise.resolve(mutationResponse());
    },
    getTeachingPreview: (courseId, assignmentId, student) => {
      requireCourse(courseId);
      return Promise.resolve(
        student === ACTIVE_STUDENT
          ? allowedPreview(assignmentId, student)
          : decodeTeachingPreviewView({ entitlement: "denied", reason: "notEntitled" }),
      );
    },
    approveInstructorAccount: (account, observed) => {
      if (observed !== undefined) requireRevision(observed);
      if (account !== DEMO_ACCOUNT)
        return Promise.reject(new ApiProtocolError("Mock account is not found"));
      approved = true;
      return Promise.resolve(decodeAccountApprovalView({ state: "approved", revision: advance() }));
    },
    revokeInstructorApproval: (account, observed) => {
      requireRevision(observed);
      if (account !== DEMO_ACCOUNT)
        return Promise.reject(new ApiProtocolError("Mock account is not found"));
      approved = false;
      return Promise.resolve(decodeAccountApprovalView({ state: "revoked", revision: advance() }));
    },
    listCourseCoInstructorInvitations: (courseId) => {
      requireCourse(courseId);
      const page = {
        invitations: [...invitations.entries()].map(([reference, invitation]) => ({
          reference,
          target: {
            account: { reference: invitation.target, display: accountDisplay(invitation.target) },
            approval: { state: approved ? "approved" : "revoked", revision: String(revision) },
          },
          state: invitation.state,
          createdAt: FIXTURE_TIME,
          expiresAt: FIXTURE_TIME + 2_592_000_000,
          revision: String(revision),
        })),
        nextCursor: null,
      };
      return Promise.resolve(decodeCourseCoInstructorInvitationsPage(page));
    },
    searchCourseCoInstructorTargets: (courseId, query, cursor, pageSize) => {
      requireCourse(courseId);
      const needle = targetSearch(query);
      const pendingTargets = new Set(
        [...invitations.values()]
          .filter((invitation) => invitation.state === "pending")
          .map((invitation) => invitation.target),
      );
      const activeInstructors = new Set(
        [...instructors.values()].map((instructor) => instructor.account),
      );
      const targets = accounts
        .filter((account) => (account.reference === DEMO_ACCOUNT ? approved : account.approved))
        .filter(
          (account) =>
            !activeInstructors.has(account.reference) && !pendingTargets.has(account.reference),
        )
        .filter((account) => account.display.toLocaleLowerCase().includes(needle))
        .map((account) => ({
          account: { reference: account.reference, display: account.display },
          approval: { state: "approved", revision: String(revision) },
        }));
      const current = page(targets, cursor, pageSize);
      return Promise.resolve(
        decodeCoInstructorTargetSearchPage({
          targets: current.values,
          nextCursor: current.nextCursor,
        }),
      );
    },
    createCourseCoInstructorInvitation: (courseId, request) => {
      requireCourse(courseId);
      const body = decodeCoInstructorInvitationCreateRequest(request, "request");
      const account = accounts.find((candidate) => candidate.reference === body.target);
      const effectivelyApproved =
        account?.reference === DEMO_ACCOUNT ? approved : account?.approved === true;
      if (!effectivelyApproved)
        return Promise.reject(new ApiProtocolError("Mock invitation target is not approved"));
      const reference = `CI-${nextInvitation++}`;
      invitations.set(reference, { target: body.target, state: "pending" });
      advance();
      return Promise.resolve(reference);
    },
    revokeCourseCoInstructorInvitation: (courseId, invitation, observed) => {
      requireCourse(courseId);
      requireRevision(observed);
      const current = invitations.get(invitation);
      if (current === undefined)
        return Promise.reject(new ApiProtocolError("Mock invitation is not found"));
      current.state = "revoked";
      advance();
      return Promise.resolve();
    },
    listPendingCoInstructorInvitations: () =>
      Promise.resolve(
        decodePendingCoInstructorInvitationsPage({
          invitations: [...invitations.entries()]
            .filter(
              ([, invitation]) =>
                invitation.state === "pending" &&
                (config.accountPendingInvitation !== true || invitation.target === PENDING_ACCOUNT),
            )
            .map(([reference]) => ({
              reference,
              courseLabel: "Demo course",
              state: "pending",
              expiresAt: FIXTURE_TIME + 2_592_000_000,
              revision: String(revision),
            })),
          nextCursor: null,
        }),
      ),
    respondToCoInstructorInvitation: (invitation, request, observed) => {
      requireRevision(observed);
      const body = decodeCoInstructorInvitationTerminalActionRequest(request, "request");
      const current = invitations.get(invitation);
      if (current === undefined || current.state !== "pending")
        return Promise.reject(new ApiProtocolError("Mock invitation is not pending"));
      if (config.accountPendingInvitation === true && current.target !== PENDING_ACCOUNT)
        return Promise.reject(new ApiProtocolError("Mock invitation belongs to another account"));
      current.state = body.action === "accept" ? "accepted" : "declined";
      if (current.state === "accepted")
        instructors.set("M-11", {
          account: current.target,
          display: accountDisplay(current.target),
        });
      advance();
      return Promise.resolve();
    },
    listCourseInstructors: (courseId) => {
      requireCourse(courseId);
      return Promise.resolve(
        decodeInstructorMembershipsPage({
          instructors: [...instructors.entries()].map(([membership, account]) => ({
            membership,
            account: { reference: account.account, display: account.display },
            status: "active",
          })),
          nextCursor: null,
          rosterRevision: String(revision),
        }),
      );
    },
    removeCourseInstructor: (courseId, membership, request, observed) => {
      requireCourse(courseId);
      requireRevision(observed);
      decodeInstructorMembershipRemovalRequest(request, "request");
      if (instructors.size <= 1)
        return Promise.reject(new ApiRequestError(409, "/api/mock/teaching-operations"));
      if (!instructors.delete(membership))
        return Promise.reject(new ApiProtocolError("Mock Instructor membership is not found"));
      advance();
      return Promise.resolve();
    },
    getCourseRetention: (courseId) => {
      requireCourse(courseId);
      return Promise.resolve(retentionRead());
    },
    endCourseRetention: (courseId) => {
      requireCourse(courseId);
      retentionState = "notificationDue";
      retentionRevision += 1;
      return Promise.resolve(retentionRead());
    },
    archiveCourseRetention: (courseId, request, observed) => {
      requireCourse(courseId);
      if (retentionArchiveForbiddenPending) {
        retentionArchiveForbiddenPending = false;
        return Promise.reject(new ApiRequestError(403, "/api/mock/teaching-operations"));
      }
      requireRetentionRevision(observed);
      retentionDisposition = decodeRetentionArchiveRequest(
        request,
        "request",
      ).assignmentDefinitions;
      retentionState = "studentRecordsArchived";
      retentionRevision += 1;
      return Promise.resolve(
        decodeRetentionActionResponse({
          state: retentionState,
          assignmentDefinitions: retentionDisposition,
          revision: String(retentionRevision),
          outcome: "completed",
        }),
      );
    },
    deleteCourseRetention: (courseId, observed) => {
      requireCourse(courseId);
      if (retentionDeleteUnavailablePending) {
        retentionDeleteUnavailablePending = false;
        return Promise.reject(new Error("Mock retention service is unavailable"));
      }
      requireRetentionRevision(observed);
      retentionState = "studentRecordsDeleted";
      retentionRevision += 1;
      return Promise.resolve(
        decodeRetentionActionResponse({
          state: retentionState,
          assignmentDefinitions: retentionDisposition,
          revision: String(retentionRevision),
          outcome: "completed",
        }),
      );
    },
    extendCourseRetention: (courseId, request, observed) => {
      requireCourse(courseId);
      requireRetentionRevision(observed);
      decodeRetentionExtendRequest(request, "request");
      retentionState = "notificationDue";
      retentionRevision += 1;
      return Promise.resolve(retentionRead());
    },
  };
}
