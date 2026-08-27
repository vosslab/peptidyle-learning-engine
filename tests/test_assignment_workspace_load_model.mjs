import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, ApiRequestError } from "../src/api/http_client/error.ts";
import { assignmentWorkspaceLoadFailureState } from "../src/pages/assignment_workspace/assignment_workspace_load_model.ts";

test("workspace load keeps authority and missing-resource responses non-enumerating", () => {
  assert.equal(
    assignmentWorkspaceLoadFailureState(new ApiRequestError(401, "/workspace")),
    "denied",
  );
  assert.equal(
    assignmentWorkspaceLoadFailureState(new ApiRequestError(403, "/workspace")),
    "denied",
  );
  assert.equal(
    assignmentWorkspaceLoadFailureState(new ApiRequestError(404, "/workspace")),
    "unavailable",
  );
});

test("workspace load exposes protocol and unexpected failures for retry", () => {
  assert.equal(
    assignmentWorkspaceLoadFailureState(new ApiProtocolError("invalid response")),
    "error",
  );
  assert.equal(assignmentWorkspaceLoadFailureState(new Error("transport failed")), "error");
});
