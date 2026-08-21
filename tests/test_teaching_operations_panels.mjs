import assert from "node:assert/strict";
import test from "node:test";

import {
  groupConflictCopy,
  appendGroupPage,
  membershipWarningCopy,
  policyCopy,
  purposeLabel,
  referencedGroupCopy,
  retentionStateCopy,
} from "../src/pages/teaching_operations/course_groups_panel_model.ts";
import {
  retentionActionAvailability,
  retentionFailureCopy,
  retentionOutcomeCopy,
  retentionReloadRequired,
} from "../src/pages/teaching_operations/retention_panel_model.ts";

test("teaching-operation group policy copy and aggregate multi-purpose warnings stay safe", () => {
  assert.equal(purposeLabel("section"), "Section");
  assert.equal(purposeLabel("accommodation"), "Accommodation");
  assert.match(policyCopy("warn"), /never blocks/u);
  assert.match(groupConflictCopy(), /draft is preserved/u);
  assert.match(referencedGroupCopy(), /assignment audience or policy modifier/u);
  assert.equal(
    membershipWarningCopy({ disposition: "allowed", warningCount: 0 }),
    "Course-group membership check: allowed. No overlapping memberships need attention.",
  );
  assert.equal(
    membershipWarningCopy({ disposition: "allowedWithWarning", warningCount: 2 }),
    "Course-group membership check: allowed with warning. 2 overlapping memberships need attention.",
  );
  assert.doesNotMatch(
    membershipWarningCopy({ disposition: "allowedWithWarning", warningCount: 2 }),
    /section|lab|cohort|accommodation|work/u,
  );
});

test("teaching-operation group cursor append preserves existing rows and ignores overlap", () => {
  const first = [
    { reference: "G-1", title: "Section A", purpose: "section", revision: "1", memberCount: 2 },
  ];
  const second = [
    { reference: "G-1", title: "Section A", purpose: "section", revision: "1", memberCount: 2 },
    { reference: "G-2", title: "Section B", purpose: "section", revision: "1", memberCount: 3 },
  ];
  assert.deepEqual(appendGroupPage(first, second), [first[0], second[1]]);
});

test("teaching-operation retention copy keeps server states and failure recovery clear", () => {
  assert.equal(
    retentionStateCopy("studentRecordsDeleted"),
    "Student records have been permanently deleted.",
  );
  assert.equal(retentionOutcomeCopy("inProgress"), "The retention action is already in progress.");
  assert.match(retentionFailureCopy(403), /permission/u);
  assert.match(retentionFailureCopy(undefined), /offline/u);
  assert.equal(retentionReloadRequired(412), true);
  assert.equal(retentionReloadRequired(403), false);
  assert.deepEqual(retentionActionAvailability("notificationDue"), {
    archive: true,
    delete: true,
    extend: true,
  });
  assert.deepEqual(retentionActionAvailability("studentRecordsArchived"), {
    archive: false,
    delete: false,
    extend: false,
  });
});
