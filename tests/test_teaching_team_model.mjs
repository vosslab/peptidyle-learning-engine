import assert from "node:assert/strict";
import test from "node:test";

import {
  appendTeachingTeamPage,
  finalInstructorConflictCopy,
  invitationStateLabel,
  isPendingInvitation,
  serverExpiryCopy,
} from "../src/pages/teaching_team_model.ts";

test("teaching-team pagination keeps existing rows and excludes overlapping cursor rows", () => {
  const first = [{ reference: "safe-one" }];
  const next = [{ reference: "safe-one" }, { reference: "safe-two" }];

  assert.deepEqual(appendTeachingTeamPage(first, next), [first[0], next[1]]);
});

test("teaching-team copy keeps approval distinct from final-instructor course authority", () => {
  assert.equal(invitationStateLabel("pending"), "Pending response");
  assert.equal(isPendingInvitation("expired"), false);
  assert.match(finalInstructorConflictCopy(), /keep one active instructor/u);
  assert.match(serverExpiryCopy(1_789_837_200_000), /server supplied/u);
  assert.doesNotMatch(serverExpiryCopy(1_789_837_200_000), /1789837200000/u);
});
