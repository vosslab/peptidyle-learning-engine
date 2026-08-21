import assert from "node:assert/strict";
import test from "node:test";

import {
  appendInstructorCandidatePage,
  approvalFailureCopy,
  approvalReloadRequired,
  approvalSuccessCopy,
  candidateAction,
  candidateActionRevision,
  candidateApprovalLabel,
  isCandidateQueryEligible,
} from "../src/pages/teaching_operations/sysadmin_instructor_approval_model.ts";

function candidate(reference, display, state, revision) {
  return { account: { reference, display }, approval: { state, revision } };
}

test("Sysadmin instructor approval keeps the two-character display-name boundary", () => {
  assert.equal(isCandidateQueryEligible(" A "), false);
  assert.equal(isCandidateQueryEligible("  A\u0301  "), true);
  assert.equal(isCandidateQueryEligible(" Avery "), true);
});

test("Sysadmin instructor approval deduplicates opaque references while paging", () => {
  const first = candidate("U-1", "Avery Singh", "unapproved", null);
  const second = candidate("U-2", "Taylor Nguyen", "revoked", "7");
  assert.deepEqual(appendInstructorCandidatePage([first], [first, second]), [first, second]);
});

test("Sysadmin instructor approval preserves exact optimistic-revision semantics", () => {
  const unapproved = candidate("U-1", "Avery Singh", "unapproved", null);
  const revoked = candidate("U-2", "Taylor Nguyen", "revoked", "7");
  const approved = candidate("U-3", "Sam Rivera", "approved", "8");
  assert.equal(candidateAction(unapproved), "approve");
  assert.equal(candidateAction(revoked), "approve");
  assert.equal(candidateAction(approved), "revoke");
  assert.equal(candidateActionRevision(unapproved), undefined);
  assert.equal(candidateActionRevision(revoked), "7");
  assert.equal(candidateActionRevision(approved), "8");
  assert.equal(candidateApprovalLabel(approved), "Approved for invitations");
});

test("Sysadmin instructor approval uses safe conflict and outcome copy", () => {
  assert.match(
    approvalSuccessCopy("Avery Singh", "approve"),
    /did not add Avery Singh to a course/u,
  );
  assert.match(approvalSuccessCopy("Avery Singh", "revoke"), /no longer eligible/u);
  assert.match(approvalFailureCopy(403), /permission/u);
  assert.match(approvalFailureCopy(412), /Results were refreshed/u);
  assert.doesNotMatch(approvalFailureCopy(undefined), /API request|\/api\//u);
  assert.equal(approvalReloadRequired(409), true);
  assert.equal(approvalReloadRequired(412), true);
  assert.equal(approvalReloadRequired(403), false);
});
