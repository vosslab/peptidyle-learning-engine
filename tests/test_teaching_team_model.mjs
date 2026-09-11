import assert from "node:assert/strict";
import test from "node:test";

import {
  appendTeachingTeamPage,
  finalInstructorConflictCopy,
  invitationStateLabel,
  isPendingInvitation,
  serverExpiryCopy,
} from "../src/pages/teaching_team_model.ts";
import {
  decodeInstructorCourseInvitationsPage,
  decodePendingCourseInvitationsPage,
} from "../src/api/decoders/teaching_operations.ts";

test("teaching-team pagination keeps existing rows and excludes overlapping cursor rows", () => {
  const first = [{ reference: "safe-one" }];
  const next = [{ reference: "safe-one" }, { reference: "safe-two" }];

  assert.deepEqual(appendTeachingTeamPage(first, next), [first[0], next[1]]);
});

test("teaching-team copy keeps Account State distinct from final-instructor course authority", () => {
  assert.equal(invitationStateLabel("pending"), "Pending response");
  assert.equal(isPendingInvitation("expired"), false);
  assert.match(finalInstructorConflictCopy(), /keep one active instructor/u);
  assert.match(serverExpiryCopy(1_789_837_200_000, "America/New_York"), /America\/New_York/u);
  assert.doesNotMatch(serverExpiryCopy(1_789_837_200_000, "America/New_York"), /1789837200000/u);
});

test("invitation expiry uses the explicit viewer zone on both invitation page envelopes", () => {
  const instant = Date.parse("2026-01-15T18:30:00Z");
  const eastern = serverExpiryCopy(instant, "America/New_York");
  const pacific = serverExpiryCopy(instant, "America/Los_Angeles");

  assert.match(eastern, /\(America\/New_York\)$/u);
  assert.match(pacific, /\(America\/Los_Angeles\)$/u);
  assert.doesNotMatch(eastern, /server supplied/u);
});

test("invitation page decoders require an outer viewer zone and reject target-zone leakage", () => {
  const instructorPage = {
    displayTimeZone: "America/New_York",
    invitations: [],
    nextCursor: null,
  };
  const pendingPage = {
    displayTimeZone: "America/New_York",
    invitations: [],
    nextCursor: null,
  };

  assert.equal(
    decodeInstructorCourseInvitationsPage(instructorPage).displayTimeZone,
    "America/New_York",
  );
  assert.equal(decodePendingCourseInvitationsPage(pendingPage).displayTimeZone, "America/New_York");
  assert.throws(
    () => decodeInstructorCourseInvitationsPage({ invitations: [], nextCursor: null }),
    /displayTimeZone/u,
  );
  assert.throws(
    () =>
      decodePendingCourseInvitationsPage({
        ...pendingPage,
        invitations: [
          {
            reference: "CI-1",
            courseLabel: "Biochemistry",
            state: "pending",
            expiresAt: 1_768_504_200_000,
            state_precondition: "1",
            displayTimeZone: "America/Chicago",
          },
        ],
      }),
    /field allowed/u,
  );
  assert.throws(
    () =>
      decodePendingCourseInvitationsPage({
        ...pendingPage,
        displayTimeZone: "not/a-zone",
      }),
    /browser-supported IANA/u,
  );
});
