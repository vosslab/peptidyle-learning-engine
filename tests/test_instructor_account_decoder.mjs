// Sysadmin account-list display context stays viewer-owned and strictly decoded.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeInstructorAccountList } from "../src/api/decoders/instructor_account.ts";
import { formatSignInLabel } from "../src/pages/instructor_account_model.ts";

const listResponse = {
  accounts: [
    {
      reference: "U-42",
      state: "active",
      lastSuccessfulSignIn: 1768501800000,
    },
  ],
  displayTimeZone: "America/New_York",
};

test("Sysadmin list carries only the viewer display zone outside closed target summaries", () => {
  assert.deepEqual(decodeInstructorAccountList(listResponse), listResponse);

  assert.throws(
    () =>
      decodeInstructorAccountList({
        ...listResponse,
        accounts: [{ ...listResponse.accounts[0], displayTimeZone: "America/Los_Angeles" }],
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeInstructorAccountList({ ...listResponse, targetTimeZone: "America/Chicago" }),
    DecodeError,
  );
  assert.throws(
    () => decodeInstructorAccountList({ ...listResponse, displayTimeZone: "not/a-zone" }),
    DecodeError,
  );
});

test("Sysadmin sign-in timestamps render in the supplied viewer zone", () => {
  const instant = Date.parse("2026-01-15T18:30:00Z");
  assert.equal(formatSignInLabel(instant, "America/New_York"), "Jan 15, 2026, 1:30 PM");
  assert.equal(formatSignInLabel(instant, "America/Los_Angeles"), "Jan 15, 2026, 10:30 AM");
});

test("Instructor Accounts visibly names the viewer zone for sign-in times", () => {
  const page = readFileSync(
    new URL("../src/pages/instructor_accounts_page.tsx", import.meta.url),
    "utf8",
  );

  assert.match(
    page,
    /Last successful sign-in times use your time zone:\s*\{list\.displayTimeZone\}\./u,
  );
});
