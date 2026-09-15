import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, ApiRequestError } from "../src/api/http_client/error.ts";
import { assessmentWorkspaceLoadFailureState } from "../src/pages/assessment_workspace/assessment_workspace_load_model.ts";

test("workspace load keeps authority and missing-resource responses non-enumerating", () => {
  assert.equal(
    assessmentWorkspaceLoadFailureState(new ApiRequestError(401, "/workspace")),
    "denied",
  );
  assert.equal(
    assessmentWorkspaceLoadFailureState(new ApiRequestError(403, "/workspace")),
    "denied",
  );
  assert.equal(
    assessmentWorkspaceLoadFailureState(new ApiRequestError(404, "/workspace")),
    "unavailable",
  );
});

test("workspace load exposes protocol and unexpected failures for retry", () => {
  assert.equal(
    assessmentWorkspaceLoadFailureState(new ApiProtocolError("invalid response")),
    "error",
  );
  assert.equal(assessmentWorkspaceLoadFailureState(new Error("transport failed")), "error");
});
