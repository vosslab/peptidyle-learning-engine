// onboarding_preflight.ts - redacted readiness gate for canonical account walkthroughs.

import { lstatSync } from "node:fs";

export type OnboardingPreflightOutcome = "PASS" | "BLOCKED" | "FAIL";

export type OnboardingPreflightReasonCode =
  | "LOCAL_DEVELOPMENT_AUTH"
  | "SMTP_PROVIDER_UNAVAILABLE"
  | "SMTP_CONFIGURATION_INVALID"
  | "SMTP_PASSWORD_FILE_UNSAFE"
  | "TEST_MAILBOX_UNCONFIRMED"
  | "DELIVERED_LINK_UNCONFIRMED"
  | "READY_FOR_CANONICAL_ONBOARDING";

export interface OnboardingPreflightResult {
  readonly outcome: OnboardingPreflightOutcome;
  readonly reasonCode: OnboardingPreflightReasonCode;
}

export type OnboardingEnvironment = Readonly<Record<string, string | undefined>>;

export interface SecretFileMetadata {
  readonly isFile: boolean;
  readonly isSymbolicLink: boolean;
  readonly mode: number;
}

export type SecretFileInspector = (path: string) => SecretFileMetadata;

const SMTP_FIELDS = [
  "PLE_SMTP_RELAY",
  "PLE_SMTP_PORT",
  "PLE_SMTP_TLS_MODE",
  "PLE_SMTP_USERNAME",
  "PLE_SMTP_PASSWORD_HOST_FILE",
  "PLE_SMTP_FROM",
  "PLE_PUBLIC_APP_BASE_URL",
] as const;

const MAILBOX_READY = "PLE_UI_WALKTHROUGH_ONBOARDING_MAILBOX_READY";
const DELIVERED_LINK_READY = "PLE_UI_WALKTHROUGH_ONBOARDING_DELIVERED_LINK_READY";

function defaultSecretFileInspector(path: string): SecretFileMetadata {
  const metadata = lstatSync(path);
  return {
    isFile: metadata.isFile(),
    isSymbolicLink: metadata.isSymbolicLink(),
    mode: metadata.mode,
  };
}

function result(
  outcome: OnboardingPreflightOutcome,
  reasonCode: OnboardingPreflightReasonCode,
): OnboardingPreflightResult {
  return { outcome, reasonCode };
}

function value(environment: OnboardingEnvironment, name: string): string | undefined {
  const raw = environment[name];
  if (raw === undefined) return undefined;
  const trimmed = raw.trim();
  return trimmed === "" ? undefined : trimmed;
}

function localDevelopmentResult(
  environment: OnboardingEnvironment,
): OnboardingPreflightResult | undefined {
  const flag = value(environment, "PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH");
  if (flag === undefined || flag === "0") return undefined;
  if (flag === "1") return result("BLOCKED", "LOCAL_DEVELOPMENT_AUTH");
  return result("FAIL", "SMTP_CONFIGURATION_INVALID");
}

function smtpConfigured(environment: OnboardingEnvironment): boolean {
  return SMTP_FIELDS.some((name) => value(environment, name) !== undefined);
}

function validSmtpConfiguration(environment: OnboardingEnvironment): boolean {
  const relay = value(environment, "PLE_SMTP_RELAY");
  const port = value(environment, "PLE_SMTP_PORT");
  const tlsMode = value(environment, "PLE_SMTP_TLS_MODE");
  const username = value(environment, "PLE_SMTP_USERNAME");
  const from = value(environment, "PLE_SMTP_FROM");
  if (
    relay === undefined ||
    port === undefined ||
    tlsMode === undefined ||
    username === undefined ||
    from === undefined
  ) {
    return false;
  }
  if (!/^[A-Za-z0-9](?:[A-Za-z0-9.-]{0,251}[A-Za-z0-9])?$/u.test(relay)) return false;
  const numericPort = Number(port);
  if (
    !/^[0-9]+$/u.test(port) ||
    !Number.isSafeInteger(numericPort) ||
    numericPort < 1 ||
    numericPort > 65535
  ) {
    return false;
  }
  if (tlsMode !== "starttls" && tlsMode !== "implicit-tls") return false;
  if (/\s/u.test(username) || username.length > 320) return false;
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/u.test(from) || from.length > 320) return false;
  return validPublicHttpsOrigin(value(environment, "PLE_PUBLIC_APP_BASE_URL"));
}

function validPublicHttpsOrigin(raw: string | undefined): boolean {
  if (raw === undefined) return false;
  try {
    const parsed = new URL(raw);
    return (
      parsed.protocol === "https:" &&
      parsed.username === "" &&
      parsed.password === "" &&
      parsed.pathname === "/" &&
      parsed.search === "" &&
      parsed.hash === ""
    );
  } catch {
    return false;
  }
}

function safePasswordFile(
  environment: OnboardingEnvironment,
  inspectSecretFile: SecretFileInspector,
): boolean {
  const path = value(environment, "PLE_SMTP_PASSWORD_HOST_FILE");
  if (path === undefined || !path.startsWith("/")) return false;
  try {
    const metadata = inspectSecretFile(path);
    return (
      metadata.isFile &&
      !metadata.isSymbolicLink &&
      (process.platform === "win32" || (metadata.mode & 0o777) === 0o600)
    );
  } catch {
    return false;
  }
}

function confirmationResult(
  environment: OnboardingEnvironment,
  name: string,
  missingCode: "TEST_MAILBOX_UNCONFIRMED" | "DELIVERED_LINK_UNCONFIRMED",
): OnboardingPreflightResult | undefined {
  const confirmed = value(environment, name);
  if (confirmed === undefined || confirmed === "0") return result("BLOCKED", missingCode);
  if (confirmed !== "1") return result("FAIL", "SMTP_CONFIGURATION_INVALID");
  return undefined;
}

/**
 * Checks only redacted operator readiness. PASS permits WP-W10 to start; it
 * does not claim a mail send, email completion, invitation claim, or passkey.
 */
export function onboardingPreflightFromEnvironment(
  environment: OnboardingEnvironment,
  inspectSecretFile: SecretFileInspector = defaultSecretFileInspector,
): OnboardingPreflightResult {
  const local = localDevelopmentResult(environment);
  if (local !== undefined) return local;
  if (
    value(environment, "PLE_AUTH_PROVIDER") !== undefined ||
    value(environment, "PLE_LOCAL_AUTH_FILE") !== undefined
  ) {
    return result("FAIL", "SMTP_CONFIGURATION_INVALID");
  }
  if (!smtpConfigured(environment)) return result("BLOCKED", "SMTP_PROVIDER_UNAVAILABLE");
  if (!validSmtpConfiguration(environment)) return result("FAIL", "SMTP_CONFIGURATION_INVALID");
  if (!safePasswordFile(environment, inspectSecretFile))
    return result("FAIL", "SMTP_PASSWORD_FILE_UNSAFE");
  const mailbox = confirmationResult(environment, MAILBOX_READY, "TEST_MAILBOX_UNCONFIRMED");
  if (mailbox !== undefined) return mailbox;
  const deliveredLink = confirmationResult(
    environment,
    DELIVERED_LINK_READY,
    "DELIVERED_LINK_UNCONFIRMED",
  );
  if (deliveredLink !== undefined) return deliveredLink;
  return result("PASS", "READY_FOR_CANONICAL_ONBOARDING");
}
