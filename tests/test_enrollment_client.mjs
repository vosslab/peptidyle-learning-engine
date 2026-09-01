// Stable browser-boundary tests for passwordless enrollment and protected roster data.

import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeCourseInvitationAccepted,
  decodeCourseRosterPage,
  decodeRosterImportCommitResult,
  decodeRosterImportPreview,
} from "../src/api/enrollment.ts";
import { rosterImportTemplateCsv } from "../src/pages/roster_import_template.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { consumeTokenFragment } from "../src/auth/secret_fragment.ts";

const COURSE = "0198e000-0000-7000-8000-000000000014";
const INVITATION = "0198e000-0000-7000-8000-000000000601";
const IMPORT = "0198e000-0000-7000-8000-000000000604";
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

test("course roster decoders reject authority and secret fields", () => {
  const roster = {
    rosterMode: "courseInvitations",
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
    nextCursor: null,
    rosterRevision: 4,
  };
  assert.deepEqual(decodeCourseRosterPage(roster), roster);
  assert.throws(
    () => decodeCourseRosterPage({ ...roster, invitationToken: "must-not-cross" }),
    /field allowed by this response contract/u,
  );

  const accepted = {
    invitation: roster.pendingInvitations[0],
    redemptionPath: REDEMPTION_PATH,
    emailDelivery: "queued",
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

test("roster pagination preserves the opaque cursor", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: (input, init = {}) => {
      const url = new URL(String(input), "https://ple.example");
      requests.push({ url, init });
      if (url.pathname.endsWith("/roster")) {
        return Promise.resolve(
          json({
            rosterMode: "courseInvitations",
            members: [],
            pendingInvitations: [],
            allowedEmailDomains: [],
            nextCursor: null,
            rosterRevision: 4,
          }),
        );
      }
      throw new Error(`unexpected request ${url.pathname}`);
    },
  });

  await client.listCourseRoster(COURSE, "opaque+cursor/value");

  assert.equal(requests[0]?.url.searchParams.get("cursor"), "opaque+cursor/value");
  assert.equal(requests[0]?.init.credentials, "same-origin");
  assert.equal(requests[0]?.init.cache, "no-store");
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
        result: "readyToInvite",
        reason: "ready",
      },
      {
        rowNumber: 3,
        email: null,
        rosterId: null,
        result: "invalid",
        reason: "correctEmailOrRosterId",
      },
    ],
  });
  assert.equal(preview.rows[1]?.email, null);
  assert.equal(preview.rows[1]?.rosterId, null);
  assert.throws(
    () =>
      decodeRosterImportPreview({
        ...preview,
        rows: [
          {
            rowNumber: 3,
            email: "raw-invalid-cell",
            rosterId: null,
            result: "invalid",
            reason: "correctEmailOrRosterId",
          },
        ],
      }),
    /response/u,
  );
  assert.throws(
    () =>
      decodeRosterImportPreview({
        ...preview,
        rows: [
          {
            rowNumber: 2,
            email: "student@example.edu",
            rosterId: "900123456",
            result: "readyToInvite",
            reason: "alreadyOnRoster",
          },
        ],
      }),
    /safe category/u,
  );
});

test("roster import template has only generic headers and an example row", () => {
  assert.equal(rosterImportTemplateCsv(), "email,roster_id\nstudent@example.edu,900123456\n");
});

test("bulk delivery keeps only row numbers and coarse outcomes", () => {
  const result = decodeRosterImportCommitResult({
    importId: IMPORT,
    importRevision: 2,
    rosterRevision: 5,
    invitationsCreated: 2,
    delivery: [
      { rowNumber: 2, outcome: "queued" },
      { rowNumber: 4, outcome: "needsAttention" },
    ],
  });
  assert.deepEqual(result.delivery, [
    { rowNumber: 2, outcome: "queued" },
    { rowNumber: 4, outcome: "needsAttention" },
  ]);
  assert.throws(
    () =>
      decodeRosterImportCommitResult({
        ...result,
        delivery: [{ rowNumber: 2, outcome: "sentToProvider", recipient: "student@example.edu" }],
      }),
    /field allowed/u,
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
            emailDelivery: "sentToProvider",
          },
          202,
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
                result: "readyToInvite",
                reason: "ready",
              },
            ],
          },
          200,
          { etag: '"1"' },
        );
      }
      if (path.endsWith("/invitation-email-rule")) {
        return json(
          {
            allowedEmailDomains: [{ domain: "example.edu", includeSubdomains: false }],
            rosterRevision: 5,
          },
          200,
          { etag: '"5"' },
        );
      }
      throw new Error(`unexpected request ${path}`);
    },
  });

  await client.inviteCourseMember(COURSE, "student@example.edu", "900123456", "invite-once");
  await client.previewRosterImport(
    COURSE,
    new Blob(["email,roster_id\nstudent@example.edu,900123456\n"], { type: "text/csv" }),
    4,
    "preview-once",
  );
  await client.replaceCourseInvitationEmailRule(
    COURSE,
    { allowedEmailDomains: [{ domain: "example.edu", includeSubdomains: false }] },
    4,
  );
  assert.equal(requests[0]?.init.headers["idempotency-key"], "invite-once");
  assert.equal(requests[1]?.init.headers["if-match"], '"4"');
  assert.equal(requests[1]?.init.headers["content-type"], "text/csv; charset=utf-8");
  assert.match(requests[2]?.path ?? "", /\/invitation-email-rule$/u);
  assert.equal(requests[2]?.init.headers["if-match"], '"4"');
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
