// onboarding_preflight.spec.ts - offline redacted canonical-account readiness checks.

import { expect, test } from "@playwright/test";

import {
  onboardingPreflightFromEnvironment,
  type OnboardingEnvironment,
  type OnboardingPreflightResult,
  type SecretFileMetadata,
} from "./onboarding_preflight";

const SECRET = "/operator/smtp-password";
const SAFE_SECRET: SecretFileMetadata = { isFile: true, isSymbolicLink: false, mode: 0o600 };
const READY: OnboardingEnvironment = {
  PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH: "0",
  PLE_SMTP_RELAY: "smtp.example.test",
  PLE_SMTP_PORT: "587",
  PLE_SMTP_TLS_MODE: "starttls",
  PLE_SMTP_USERNAME: "operator@example.test",
  PLE_SMTP_PASSWORD_HOST_FILE: SECRET,
  PLE_SMTP_FROM: "no-reply@example.test",
  PLE_PUBLIC_APP_BASE_URL: "https://ple.example.test",
  PLE_UI_WALKTHROUGH_ONBOARDING_MAILBOX_READY: "1",
  PLE_UI_WALKTHROUGH_ONBOARDING_DELIVERED_LINK_READY: "1",
};

function check(
  environment: OnboardingEnvironment,
  metadata = SAFE_SECRET,
): OnboardingPreflightResult {
  return onboardingPreflightFromEnvironment(environment, () => metadata);
}

test("local development is deterministically blocked before SMTP readiness", () => {
  expect(check({ ...READY, PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH: "1", PLE_SMTP_PORT: "bad" })).toEqual(
    {
      outcome: "BLOCKED",
      reasonCode: "LOCAL_DEVELOPMENT_AUTH",
    },
  );
  expect(check({ PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH: "1" })).toEqual({
    outcome: "BLOCKED",
    reasonCode: "LOCAL_DEVELOPMENT_AUTH",
  });
});

test("missing external provider is blocked, while explicit malformed configuration fails", () => {
  expect(check({ PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH: "0" })).toEqual({
    outcome: "BLOCKED",
    reasonCode: "SMTP_PROVIDER_UNAVAILABLE",
  });
  expect(check({ ...READY, PLE_SMTP_PORT: "0" })).toEqual({
    outcome: "FAIL",
    reasonCode: "SMTP_CONFIGURATION_INVALID",
  });
  expect(check({ ...READY, PLE_PUBLIC_APP_BASE_URL: "http://ple.example.test" })).toEqual({
    outcome: "FAIL",
    reasonCode: "SMTP_CONFIGURATION_INVALID",
  });
  expect(check({ ...READY, PLE_AUTH_PROVIDER: "local-file" })).toEqual({
    outcome: "FAIL",
    reasonCode: "SMTP_CONFIGURATION_INVALID",
  });
});

test("secret-file metadata is required but its contents are never read", () => {
  for (const metadata of [
    { isFile: false, isSymbolicLink: false, mode: 0o600 },
    { isFile: true, isSymbolicLink: true, mode: 0o600 },
    { isFile: true, isSymbolicLink: false, mode: 0o640 },
  ]) {
    expect(check(READY, metadata)).toEqual({
      outcome: "FAIL",
      reasonCode: "SMTP_PASSWORD_FILE_UNSAFE",
    });
  }
});

test("operator confirmations block in order and malformed explicit flags fail", () => {
  expect(check({ ...READY, PLE_UI_WALKTHROUGH_ONBOARDING_MAILBOX_READY: "0" })).toEqual({
    outcome: "BLOCKED",
    reasonCode: "TEST_MAILBOX_UNCONFIRMED",
  });
  expect(
    check({
      ...READY,
      PLE_UI_WALKTHROUGH_ONBOARDING_DELIVERED_LINK_READY: "0",
    }),
  ).toEqual({ outcome: "BLOCKED", reasonCode: "DELIVERED_LINK_UNCONFIRMED" });
  expect(check({ ...READY, PLE_UI_WALKTHROUGH_ONBOARDING_MAILBOX_READY: "yes" })).toEqual({
    outcome: "FAIL",
    reasonCode: "SMTP_CONFIGURATION_INVALID",
  });
});

test("ready production posture is bounded and secret-free", () => {
  const secret = "operator-secret-that-must-not-appear";
  const output = check(READY);
  expect(output).toEqual({ outcome: "PASS", reasonCode: "READY_FOR_CANONICAL_ONBOARDING" });
  expect(JSON.stringify(output)).not.toContain(secret);
  expect(() =>
    onboardingPreflightFromEnvironment(READY, () => {
      throw new Error(secret);
    }),
  ).not.toThrow();
  expect(
    onboardingPreflightFromEnvironment(READY, () => {
      throw new Error(secret);
    }),
  ).toEqual({ outcome: "FAIL", reasonCode: "SMTP_PASSWORD_FILE_UNSAFE" });
});
