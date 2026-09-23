import assert from "node:assert/strict";
import test from "node:test";

import {
  appendPendingInvitationPage,
  invitationStateLabel,
  isPendingInvitation,
  serverExpiryCopy,
} from "../src/pages/pending_invitations_model.ts";
import { decodePendingCourseInvitationsPage } from "../src/api/decoders/teaching_operations.ts";
import { createDisplayDateTimeFormatter } from "../src/format_datetime.ts";

test("pending-invitation pagination keeps existing rows and excludes overlapping cursor rows", () => {
  const first = [{ id: "safe-one" }];
  const next = [{ id: "safe-one" }, { id: "safe-two" }];

  assert.deepEqual(appendPendingInvitationPage(first, next), [first[0], next[1]]);
});

test("pending-invitation copy exposes the current invitation state and viewer-zone expiry", () => {
  const displayTimeZone = "America/New_York";
  const formatDateTime = createDisplayDateTimeFormatter(displayTimeZone);

  assert.equal(invitationStateLabel("pending"), "Pending response");
  assert.equal(isPendingInvitation("expired"), false);
  assert.match(
    serverExpiryCopy(1_789_837_200_000, displayTimeZone, formatDateTime),
    /America\/New_York/u,
  );
  assert.doesNotMatch(
    serverExpiryCopy(1_789_837_200_000, displayTimeZone, formatDateTime),
    /1789837200000/u,
  );
});

test("invitation expiry uses the explicit viewer zone", () => {
  const instant = Date.parse("2026-01-15T18:30:00Z");
  const easternTimeZone = "America/New_York";
  const pacificTimeZone = "America/Los_Angeles";
  const eastern = serverExpiryCopy(
    instant,
    easternTimeZone,
    createDisplayDateTimeFormatter(easternTimeZone),
  );
  const pacific = serverExpiryCopy(
    instant,
    pacificTimeZone,
    createDisplayDateTimeFormatter(pacificTimeZone),
  );

  assert.match(eastern, /\(America\/New_York\)$/u);
  assert.match(pacific, /\(America\/Los_Angeles\)$/u);
});

test("pending invitation decoder requires an outer viewer zone and rejects target-zone leakage", () => {
  const pendingPage = {
    displayTimeZone: "America/New_York",
    invitations: [],
    nextCursor: null,
  };

  assert.equal(decodePendingCourseInvitationsPage(pendingPage).displayTimeZone, "America/New_York");
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
    () => decodePendingCourseInvitationsPage({ ...pendingPage, displayTimeZone: "not/a-zone" }),
    /browser-supported IANA/u,
  );
});
