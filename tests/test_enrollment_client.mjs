// Stable browser-boundary tests for passwordless enrollment and protected roster data.

import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeAccountAuthenticated,
  decodeAccountEmailChanged,
  decodeAccountCoursePage,
  decodeCourseInvitationAccepted,
  decodeCourseRosterPage,
  decodeLocalTeachingMemberAccepted,
  decodeRosterImportPreview,
} from "../src/api/enrollment.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { consumeTokenFragment } from "../src/auth/secret_fragment.ts";

const COURSE = "0198e000-0000-7000-8000-000000000014";
const ASSIGNMENT = "0198e000-0000-7000-8000-000000000006";
const INVITATION = "0198e000-0000-7000-8000-000000000601";
const IMPORT = "0198e000-0000-7000-8000-000000000604";
const EXPORT = "0198e000-0000-7000-8000-000000000606";
const REDEMPTION_PATH = `/course-invitations/redeem#token=${"A".repeat(43)}`;

function json(value, status = 200, headers = {}) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
      ...headers,
    },
  });
}

test("passwordless and roster decoders reject authority and secret fields", () => {
  assert.deepEqual(
    decodeAccountAuthenticated({ authenticated: true, passkeyEnrollmentSuggested: true }),
    { authenticated: true, passkeyEnrollmentSuggested: true },
  );
  assert.throws(
    () => decodeAccountAuthenticated({ authenticated: true, passkeyEnrollmentRequired: true }),
    /field allowed by this response contract/u,
  );

  assert.deepEqual(decodeAccountEmailChanged({ changed: true }), { changed: true });
  assert.throws(() => decodeAccountEmailChanged({ changed: false }), /true/u);
  assert.deepEqual(
    decodeAccountCoursePage({
      courses: [{ courseId: COURSE, title: "Biochemistry", role: "student" }],
      nextCursor: null,
    }),
    {
      courses: [{ courseId: COURSE, title: "Biochemistry", role: "student" }],
      nextCursor: null,
    },
  );
  assert.throws(
    () =>
      decodeAccountCoursePage({
        courses: [{ courseId: COURSE, title: "Biochemistry", role: "student", tenant: "hidden" }],
        nextCursor: null,
      }),
    /field allowed by this response contract/u,
  );

  const roster = {
    rosterMode: "emailEnrollment",
    members: [
      {
        memberId: "0198e000-0000-7000-8000-000000000602",
        displayName: "Student",
        rosterEmail: "student@example.edu",
        rosterId: "900123456",
        role: "student",
        status: "active",
      },
    ],
    pendingInvitations: [
      {
        invitationId: INVITATION,
        email: "pending@example.edu",
        rosterId: "900123457",
        status: "pending",
        expiresAt: 1_754_893_200_000,
      },
    ],
    allowedEmailDomains: [{ domain: "example.edu", includeSubdomains: false }],
    signupPosture: "invitationOnly",
    nextCursor: null,
    rosterRevision: 4,
  };
  assert.deepEqual(decodeCourseRosterPage(roster), roster);
  assert.throws(
    () => decodeCourseRosterPage({ ...roster, invitationToken: "must-not-cross" }),
    /field allowed by this response contract/u,
  );

  const localTeachingRoster = {
    rosterMode: "localTeaching",
    members: roster.members,
    localTeachingLearners: [{ alias: "student-local", displayName: "Mary Fake Student" }],
    nextCursor: null,
    rosterRevision: 4,
  };
  assert.deepEqual(decodeCourseRosterPage(localTeachingRoster), localTeachingRoster);
  assert.throws(
    () =>
      decodeCourseRosterPage({
        ...localTeachingRoster,
        pendingInvitations: [],
      }),
    /field allowed by this response contract/u,
  );
  assert.throws(
    () =>
      decodeCourseRosterPage({
        ...localTeachingRoster,
        localTeachingLearners: [
          { alias: "student-local", displayName: "Mary Fake Student", userId: COURSE },
        ],
      }),
    /field allowed by this response contract/u,
  );
  assert.deepEqual(
    decodeLocalTeachingMemberAccepted({ member: roster.members[0], rosterRevision: 4 }),
    { member: roster.members[0], rosterRevision: 4 },
  );

  const accepted = {
    invitation: roster.pendingInvitations[0],
    redemptionPath: REDEMPTION_PATH,
    emailDelivery: "notSent",
  };
  assert.deepEqual(decodeCourseInvitationAccepted(accepted), accepted);
  assert.throws(
    () =>
      decodeCourseInvitationAccepted({
        ...accepted,
        redemptionPath: `https://attacker.example/${REDEMPTION_PATH}`,
      }),
    /same-origin one-time invitation path/u,
  );
});

test("account email change stays browser-bound and roster pagination uses the opaque cursor", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: (input, init = {}) => {
      const url = new URL(String(input), "https://ple.example");
      requests.push({ url, init });
      if (url.pathname.endsWith("/email/start"))
        return Promise.resolve(json({ accepted: true }, 202));
      if (url.pathname.endsWith("/email/complete")) {
        return Promise.resolve(json({ changed: true }));
      }
      if (url.pathname.endsWith("/roster")) {
        return Promise.resolve(
          json({
            rosterMode: "emailEnrollment",
            members: [],
            pendingInvitations: [],
            allowedEmailDomains: [],
            signupPosture: "invitationOnly",
            nextCursor: null,
            rosterRevision: 4,
          }),
        );
      }
      throw new Error(`unexpected request ${url.pathname}`);
    },
  });

  await client.startAccountEmailChange("new.student@example.edu");
  await client.completeAccountEmailChange("A".repeat(43));
  await client.listCourseRoster(COURSE, "opaque+cursor/value");

  assert.deepEqual(JSON.parse(requests[0]?.init.body), { email: "new.student@example.edu" });
  assert.deepEqual(JSON.parse(requests[1]?.init.body), { token: "A".repeat(43) });
  assert.equal(requests[2]?.url.searchParams.get("cursor"), "opaque+cursor/value");
  assert.equal(requests[0]?.init.credentials, "same-origin");
  assert.equal(requests[1]?.init.cache, "no-store");
});

test("roster import preview withholds invalid cells and keeps row selection explicit", () => {
  const preview = decodeRosterImportPreview({
    importId: IMPORT,
    state: "preview",
    expiresAt: 1_754_893_200_000,
    rosterRevision: 4,
    importRevision: 1,
    rows: [
      {
        rowNumber: 2,
        email: "student@example.edu",
        rosterId: "900123456",
        status: "readyToInvite",
      },
      { rowNumber: 3, email: null, rosterId: null, status: "invalid" },
    ],
  });
  assert.equal(preview.rows[1]?.email, null);
  assert.equal(preview.rows[1]?.rosterId, null);
  assert.throws(
    () =>
      decodeRosterImportPreview({
        ...preview,
        rows: [{ rowNumber: 3, email: "raw-invalid-cell", rosterId: null, status: "invalid" }],
      }),
    /response/u,
  );
});

test("roster mutations preserve revisions idempotency and protected export headers", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init = {}) => {
      const path = new URL(String(input), "https://ple.example").pathname;
      requests.push({ path, init });
      if (path.endsWith("/invitations")) {
        return json(
          {
            invitation: {
              invitationId: INVITATION,
              email: "student@example.edu",
              rosterId: "900123456",
              status: "pending",
              expiresAt: 1_754_893_200_000,
            },
            redemptionPath: REDEMPTION_PATH,
            emailDelivery: "notSent",
          },
          202,
        );
      }
      if (path.endsWith("/local-teaching-members")) {
        return json(
          {
            member: {
              memberId: "0198e000-0000-7000-8000-000000000602",
              displayName: "Mary Fake Student",
              rosterEmail: null,
              rosterId: null,
              role: "student",
              status: "active",
            },
            rosterRevision: 4,
          },
          200,
          { etag: '"4"' },
        );
      }
      if (path.endsWith("/roster-imports/preview")) {
        return json(
          {
            importId: IMPORT,
            state: "preview",
            expiresAt: 1_754_893_200_000,
            rosterRevision: 4,
            importRevision: 1,
            rows: [
              {
                rowNumber: 2,
                email: "student@example.edu",
                rosterId: "900123456",
                status: "readyToInvite",
              },
            ],
          },
          200,
          { etag: '"1"' },
        );
      }
      if (path.endsWith(`/assignments/${ASSIGNMENT}/grade-export.csv`)) {
        return new Response(
          "roster_id,email,display_name,score\r\n900123456,student@example.edu,Student,\r\n",
          {
            status: 200,
            headers: {
              "cache-control": "no-store",
              "content-type": "text/csv; charset=utf-8",
              "content-disposition": `attachment; filename=ple-grade-export-${ASSIGNMENT}.csv`,
              "x-ple-export-id": EXPORT,
            },
          },
        );
      }
      throw new Error(`unexpected request ${path}`);
    },
  });

  await client.inviteCourseMember(COURSE, "student@example.edu", "900123456", "invite-once");
  const activated = await client.addLocalTeachingMember(COURSE, "student-local");
  await client.previewRosterImport(
    COURSE,
    new Blob(["email,roster_id\nstudent@example.edu,900123456\n"], { type: "text/csv" }),
    4,
    "preview-once",
  );
  const exported = await client.createManualGradeExport(COURSE, ASSIGNMENT);

  assert.equal(exported.exportId, EXPORT);
  assert.equal(activated.member.displayName, "Mary Fake Student");
  assert.match(exported.csv.type, /^text\/csv(?:;|$)/u);
  assert.equal(requests[0]?.init.headers["idempotency-key"], "invite-once");
  assert.deepEqual(JSON.parse(requests[1]?.init.body), { learnerAlias: "student-local" });
  assert.equal(requests[1]?.init.headers["content-type"], "application/json");
  assert.equal(requests[2]?.init.headers["if-match"], '"4"');
  assert.equal(requests[2]?.init.headers["content-type"], "text/csv; charset=utf-8");
  assert.equal(requests[3]?.init.body, undefined);
});

test("one-time URL fragments are consumed into memory and immediately removed", () => {
  const token = "A".repeat(43);
  const replacements = [];
  const location = {
    hash: `#token=${token}`,
    pathname: "/course-invitations/redeem",
    search: "",
  };
  const history = {
    state: { navigation: "fixture" },
    replaceState: (...arguments_) => replacements.push(arguments_),
  };

  assert.equal(consumeTokenFragment(location, history), token);
  assert.deepEqual(replacements, [[{ navigation: "fixture" }, "", "/course-invitations/redeem"]]);
  assert.equal("localStorage" in location, false);
});
