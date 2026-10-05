// Browser email-code wire contract and UI recovery policy.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeCompletedEmailCodeSignIn,
  decodeStartedEmailCodeSignIn,
} from "../src/api/email_code.ts";
import { ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import {
  EMAIL_CODE_LENGTH,
  emailCodeCompletionFailure,
  emailCodeStartFailure,
} from "../src/pages/email_code_sign_in_model.ts";

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

function createRecordingFetch(requests) {
  return async (input, init) => {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    requests.push(request.clone());
    if (request.url.endsWith("/start")) {
      return jsonResponse({
        emailCodeRequested: true,
        challengeId: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
      });
    }
    return jsonResponse({ authenticated: true });
  };
}

test("email-code responses are closed browser contracts", () => {
  const started = {
    emailCodeRequested: true,
    challengeId: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  };
  assert.deepEqual(decodeStartedEmailCodeSignIn(started), started);
  assert.deepEqual(decodeCompletedEmailCodeSignIn({ authenticated: true }), {
    authenticated: true,
  });
  assert.throws(
    () => decodeStartedEmailCodeSignIn({ ...started, accountId: "U0000035E" }),
    DecodeError,
  );
  assert.throws(() => decodeCompletedEmailCodeSignIn({ authenticated: false }), DecodeError);
});

test("email-code sign-in stays same-origin and sends only each ceremony input", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/ple",
    fetch: createRecordingFetch(requests),
  });
  const started = await client.startEmailCodeSignIn("student@example.edu");
  await client.completeEmailCodeSignIn(started.challengeId, "a".repeat(EMAIL_CODE_LENGTH));

  assert.deepEqual(
    requests.map((request) => new URL(request.url).pathname),
    ["/ple/api/auth/email-code/start", `/ple/api/auth/email-code/complete/${started.challengeId}`],
  );
  for (const request of requests) {
    assert.equal(request.method, "POST");
    assert.equal(request.credentials, "same-origin");
    assert.equal(request.cache, "no-store");
    assert.equal(request.headers.get("content-type"), "application/json");
  }
  assert.equal(await requests[0].text(), '{"email":"student@example.edu"}');
  assert.equal(await requests[1].text(), JSON.stringify({ code: "a".repeat(EMAIL_CODE_LENGTH) }));
  assert.throws(() => client.completeEmailCodeSignIn("not-a-uuid", "code"), DecodeError);
});

test("email sign-in UI uses unavailable feedback without claiming delivery", () => {
  assert.equal(emailCodeStartFailure({ status: 404 }), "error");
  assert.equal(emailCodeStartFailure({}), "error");
  const startUnavailable = new ApiRequestError(404, "/api/auth/email-code/start");
  assert.equal(emailCodeStartFailure(startUnavailable), "unavailable");
  const invalidEmail = new ApiRequestError(422, "/api/auth/email-code/start");
  assert.equal(emailCodeStartFailure(invalidEmail), "invalidEmail");
  const invalidCode = new ApiRequestError(401, "/api/auth/email-code/complete/challenge");
  assert.equal(emailCodeCompletionFailure(invalidCode), "invalidCode");
  assert.equal(emailCodeCompletionFailure(startUnavailable), "unavailable");
});
