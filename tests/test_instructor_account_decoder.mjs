// Sysadmin account-list display context stays viewer-owned and strictly decoded.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeInstructorAccountList } from "../src/api/decoders/instructor_account.ts";
import { createDisplayDateTimeFormatter } from "../src/format_datetime.ts";
import { formatSignInLabel } from "../src/pages/instructor_account_model.ts";

const listResponse = {
  accounts: [
    {
      id: "U7K3M2PA0",
      state: "active",
      lastSuccessfulSignIn: 1768501800000,
      providedAvatarId: null,
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
  const formatEasternDateTime = createDisplayDateTimeFormatter("America/New_York");
  const formatPacificDateTime = createDisplayDateTimeFormatter("America/Los_Angeles");

  assert.equal(formatSignInLabel(instant, formatEasternDateTime), "Jan 15, 2026, 1:30 PM");
  assert.equal(formatSignInLabel(instant, formatPacificDateTime), "Jan 15, 2026, 10:30 AM");
});

test("Instructor Accounts formats sign-in times without naming the viewer zone", () => {
  const page = readFileSync(
    new URL("../src/pages/instructor_accounts_page.tsx", import.meta.url),
    "utf8",
  );

  assert.match(page, /createDisplayDateTimeFormatter\(list\.displayTimeZone\)/u);
  assert.match(page, /Last successful sign-in:/u);
  assert.doesNotMatch(page, /Last successful sign-in times use your time zone/u);
});
