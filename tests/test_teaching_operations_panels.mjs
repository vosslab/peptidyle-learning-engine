import assert from "node:assert/strict";
import test from "node:test";

import { retentionStateCopy } from "../src/pages/teaching_operations/retention_state_copy.ts";
import {
  retentionActionAvailability,
  retentionFailureCopy,
  retentionOutcomeCopy,
  retentionReloadRequired,
} from "../src/pages/teaching_operations/retention_panel_model.ts";

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
