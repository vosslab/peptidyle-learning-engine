// Deterministic passwordless and roster fixture capability for browser previews.

import { publishedProblemFixture } from "../../../generated/fixtures/published_problem";
import type {
  AccountPresentationPreference,
  CourseRosterClient,
  EmailEnrollmentRosterPage,
  PasskeySummary,
} from "../enrollment";

const ACCOUNT_COURSE = {
  courseId: publishedProblemFixture.course.id,
  courseReference: publishedProblemFixture.course.reference,
  title: publishedProblemFixture.course.title,
  role: "instructor" as const,
};

const MOCK_INVITATION_ID = "0198e000-0000-7000-8000-000000000601";
const MOCK_MEMBER_ID = "0198e000-0000-7000-8000-000000000602";
const MOCK_PASSKEY_ID = "0198e000-0000-7000-8000-000000000603";
const MOCK_IMPORT_ID = "0198e000-0000-7000-8000-000000000604";
const MOCK_CEREMONY_ID = "0198e000-0000-7000-8000-000000000605";
const MOCK_EXPORT_ID = "0198e000-0000-7000-8000-000000000606";
const MOCK_REDEMPTION_PATH = `/course-invitations/redeem#token=${"A".repeat(43)}`;

export function createMockEnrollmentClient(): CourseRosterClient {
  let presentation: AccountPresentationPreference = { contrast: "standard" };
  let rosterRevision = 1;
  let passkeys: ReadonlyArray<PasskeySummary> = [
    {
      id: MOCK_PASSKEY_ID,
      label: "Fixture laptop",
      createdAtMillis: 1_754_806_800_000,
      lastUsedAtMillis: null,
    },
  ];
  let roster: EmailEnrollmentRosterPage = {
    rosterMode: "emailEnrollment",
    members: [
      {
        memberId: MOCK_MEMBER_ID,
        displayName: "Fixture Student",
        rosterEmail: "student@mail.roosevelt.edu",
        rosterId: "900123456",
        role: "student",
        status: "active",
      },
    ],
    pendingInvitations: [],
    allowedEmailDomains: [{ domain: "mail.roosevelt.edu", includeSubdomains: false }],
    signupPosture: "invitationOnly",
    nextCursor: null,
    rosterRevision,
  };

  function replaceRoster(
    next: Omit<EmailEnrollmentRosterPage, "rosterRevision">,
  ): EmailEnrollmentRosterPage {
    rosterRevision += 1;
    roster = { ...next, rosterRevision };
    return roster;
  }

  return {
    getAccountPresentation: () => Promise.resolve(presentation),
    saveAccountPresentation: (preference): Promise<AccountPresentationPreference> => {
      presentation = { contrast: preference.contrast };
      return Promise.resolve(presentation);
    },
    startEmailAuthentication: () => Promise.resolve({ accepted: true }),
    completeEmailAuthentication: () =>
      Promise.resolve({ authenticated: true, passkeyEnrollmentSuggested: passkeys.length === 0 }),
    startAccountEmailChange: () => Promise.resolve({ accepted: true }),
    completeAccountEmailChange: () => Promise.resolve({ changed: true }),
    listAccountCourses: () => Promise.resolve({ courses: [ACCOUNT_COURSE], nextCursor: null }),
    selectAccountCourse: (courseId) =>
      Promise.resolve({ authenticated: true, courseId, role: "instructor" }),
    redeemCourseInvitation: () =>
      Promise.resolve({
        courseId: ACCOUNT_COURSE.courseId,
        courseReference: ACCOUNT_COURSE.courseReference,
        membershipStatus: "active",
      }),
    startPasskeyRegistration: () => Promise.resolve({ ceremonyId: MOCK_CEREMONY_ID, options: {} }),
    completePasskeyRegistration: (
      _ceremonyId,
      label,
    ): ReturnType<CourseRosterClient["completePasskeyRegistration"]> => {
      const passkey = {
        id: MOCK_PASSKEY_ID,
        label,
        createdAtMillis: 1_754_806_800_000,
        lastUsedAtMillis: null,
      } satisfies PasskeySummary;
      passkeys = [passkey];
      return Promise.resolve(passkey);
    },
    startPasskeyAuthentication: () =>
      Promise.resolve({ ceremonyId: MOCK_CEREMONY_ID, options: {} }),
    completePasskeyAuthentication: () => Promise.resolve({ authenticated: true }),
    listPasskeys: () => Promise.resolve(passkeys),
    revokePasskey: (passkeyId): ReturnType<CourseRosterClient["revokePasskey"]> => {
      passkeys = passkeys.filter((passkey) => passkey.id !== passkeyId);
      return Promise.resolve();
    },
    listCourseRoster: () => Promise.resolve(roster),
    addLocalTeachingMember: () =>
      Promise.reject(new Error("Local teaching roster enrollment is unavailable in this preview.")),
    inviteCourseMember: (
      _courseId,
      email,
      rosterId,
    ): ReturnType<CourseRosterClient["inviteCourseMember"]> => {
      const invitation = {
        invitationId: MOCK_INVITATION_ID,
        email,
        rosterId,
        status: "pending" as const,
        expiresAt: 1_755_411_600_000,
      };
      replaceRoster({
        ...roster,
        pendingInvitations: [...roster.pendingInvitations, invitation],
      });
      return Promise.resolve({
        invitation,
        redemptionPath: MOCK_REDEMPTION_PATH,
        emailDelivery: "queued",
      });
    },
    revokeCourseInvitation: (
      _courseId,
      invitationId,
    ): ReturnType<CourseRosterClient["revokeCourseInvitation"]> => {
      replaceRoster({
        ...roster,
        pendingInvitations: roster.pendingInvitations.filter(
          (invitation) => invitation.invitationId !== invitationId,
        ),
      });
      return Promise.resolve({ rosterRevision });
    },
    revokeCourseMember: (
      _courseId,
      memberId,
    ): ReturnType<CourseRosterClient["revokeCourseMember"]> => {
      replaceRoster({
        ...roster,
        members: roster.members.map((member) =>
          member.memberId === memberId ? { ...member, status: "revoked" } : member,
        ),
      });
      return Promise.resolve({ rosterRevision });
    },
    replaceCourseEnrollmentPolicy: (
      _courseId,
      policy,
    ): ReturnType<CourseRosterClient["replaceCourseEnrollmentPolicy"]> => {
      replaceRoster({
        ...roster,
        allowedEmailDomains: policy.allowedEmailDomains,
        signupPosture: policy.signupPosture,
      });
      return Promise.resolve({ ...policy, rosterRevision });
    },
    previewRosterImport: () =>
      Promise.resolve({
        importId: MOCK_IMPORT_ID,
        state: "preview",
        expiresAt: 1_754_810_400_000,
        rosterRevision,
        importRevision: 1,
        rows: [
          {
            rowNumber: 2,
            email: "new.student@mail.roosevelt.edu",
            rosterId: "900123457",
            status: "readyToInvite",
            reason: "ready",
          },
        ],
      }),
    commitRosterImport: (_courseId, preview, rowNumbers) =>
      Promise.resolve({
        importId: preview.importId,
        importRevision: preview.importRevision + 1,
        rosterRevision: ++rosterRevision,
        invitationsCreated: rowNumbers.length,
        delivery: rowNumbers.map((rowNumber) => ({ rowNumber, outcome: "queued" })),
      }),
    createManualGradeExport: (_courseId, assignmentId) =>
      Promise.resolve({
        assignmentId,
        exportId: MOCK_EXPORT_ID,
        filename: `ple-grade-export-${assignmentId}.csv`,
        csv: new Blob(
          [
            "roster_id,email,display_name,score\r\n900123456,student@mail.roosevelt.edu,Fixture Student,\r\n",
          ],
          { type: "text/csv" },
        ),
      }),
  };
}
