// privacy_profiles.ts - closed screenshot privacy profiles and browser response inspection.

import type { Page } from "playwright";

import type { CaptureRecord, PrivacyProfileId } from "./manifest";

interface PrivacyProfile {
  readonly allowFilledEmail: boolean;
  readonly selectedControl: "forbidden" | "required" | "allowed";
  readonly statusHeading: "forbidden" | "allowed";
  readonly allowedResponseKeys: ReadonlySet<string>;
}

const NO_KEYS = new Set<string>();
const SELF_AGGREGATE_KEYS = new Set([
  "correct",
  "correctness",
  "score",
  "scoretotal",
  "pointsearned",
  "pointspossible",
]);

export const PRIVACY_PROFILES: Readonly<Record<PrivacyProfileId, PrivacyProfile>> = {
  public: {
    allowFilledEmail: false,
    selectedControl: "allowed",
    statusHeading: "allowed",
    allowedResponseKeys: NO_KEYS,
  },
  instructor_answer_free: {
    allowFilledEmail: false,
    selectedControl: "allowed",
    statusHeading: "allowed",
    allowedResponseKeys: SELF_AGGREGATE_KEYS,
  },
  student_unanswered: {
    allowFilledEmail: false,
    selectedControl: "forbidden",
    statusHeading: "forbidden",
    allowedResponseKeys: SELF_AGGREGATE_KEYS,
  },
  student_selected_response: {
    allowFilledEmail: false,
    selectedControl: "required",
    statusHeading: "forbidden",
    allowedResponseKeys: NO_KEYS,
  },
  student_self: {
    allowFilledEmail: false,
    selectedControl: "allowed",
    statusHeading: "allowed",
    allowedResponseKeys: SELF_AGGREGATE_KEYS,
  },
  authorization_denial: {
    allowFilledEmail: false,
    selectedControl: "forbidden",
    statusHeading: "allowed",
    allowedResponseKeys: SELF_AGGREGATE_KEYS,
  },
  sysadmin_account: {
    allowFilledEmail: false,
    selectedControl: "allowed",
    statusHeading: "allowed",
    allowedResponseKeys: NO_KEYS,
  },
  sysadmin_scoped_roster: {
    allowFilledEmail: false,
    selectedControl: "allowed",
    statusHeading: "allowed",
    allowedResponseKeys: NO_KEYS,
  },
};

const PROTECTED_SCREEN_TERMS = [
  /presentation nonce/iu,
  /source object/iu,
  /source checksum/iu,
  /answer key\s*:/iu,
  /correct answer\s*:/iu,
  /instructor answer check/iu,
  /correct feedback\s*:/iu,
  /private source/iu,
  /access token/iu,
  /internal binding/iu,
];

const PROTECTED_RESPONSE_KEYS = new Set([
  "answerkey",
  "correctanswer",
  "correctfeedback",
  "questionanswer",
  "questionanswerexplanation",
  "choicefeedback",
  "incorrectfeedback",
  "solution",
  "sourceobject",
  "sourcechecksum",
  "privatesource",
  "credential",
  "credentials",
  "accesstoken",
  "refreshtoken",
  "sessiontoken",
  "capabilityid",
  "internalbinding",
  "questionattemptid",
  "studentrecord",
  "studentrecordid",
  "answer",
  "correct",
  "correctness",
  "score",
  "scoretotal",
  "pointsearned",
  "pointspossible",
]);
const FEEDBACK_RELEASE_TIMINGS = new Set(["never", "after_submit", "after_close"]);

function normalizedKey(key: string): string {
  return key.replace(/[^a-z0-9]/giu, "").toLowerCase();
}

function protectedKey(value: unknown, allowedKeys: ReadonlySet<string>): string | undefined {
  if (Array.isArray(value)) {
    for (const item of value) {
      const match = protectedKey(item, allowedKeys);
      if (match !== undefined) return match;
    }
    return undefined;
  }
  if (value === null || typeof value !== "object") return undefined;
  for (const [key, nested] of Object.entries(value)) {
    const normalized = normalizedKey(key);
    const isReleaseTiming =
      (normalized === "questionanswer" || normalized === "questionanswerexplanation") &&
      typeof nested === "string" &&
      FEEDBACK_RELEASE_TIMINGS.has(nested);
    if (
      PROTECTED_RESPONSE_KEYS.has(normalized) &&
      !allowedKeys.has(normalized) &&
      !isReleaseTiming
    ) {
      return key;
    }
    const match = protectedKey(nested, allowedKeys);
    if (match !== undefined) return match;
  }
  return undefined;
}

export interface PrivacyMonitor {
  readonly settleResponses: () => Promise<void>;
  readonly assertSafe: (capture: CaptureRecord) => Promise<void>;
}

/**
 * ASVS 8.2.1, 14.2.6, and 15.3.1: inspect same-origin JSON responses against
 * the capture's closed minimum-data profile and reject unrelated origins.
 */
export function monitorCapturePrivacy(page: Page, applicationOrigin: string): PrivacyMonitor {
  const responseInspections: Array<Promise<void>> = [];
  const responseBodies: unknown[] = [];
  const violations: string[] = [];

  page.on("request", (request) => {
    const url = new URL(request.url());
    if (
      (url.protocol === "http:" || url.protocol === "https:") &&
      url.origin !== applicationOrigin
    ) {
      violations.push(`request escaped the application origin: ${url.origin}`);
    }
  });
  page.on("requestfinished", (request) => {
    const inspection = request
      .response()
      .then(async (response) => {
        if (response === null) return;
        const url = new URL(response.url());
        const contentType = response.headers()["content-type"] ?? "";
        if (url.origin !== applicationOrigin || !contentType.includes("application/json")) return;
        try {
          responseBodies.push((await response.json()) as unknown);
        } catch {
          violations.push(`${url.pathname} could not be inspected as JSON`);
        }
      })
      .catch(() => {
        violations.push(`${new URL(request.url()).pathname} response inspection failed`);
      });
    responseInspections.push(inspection);
  });

  async function settleResponses(): Promise<void> {
    await Promise.all(responseInspections.splice(0));
  }

  return {
    settleResponses,
    async assertSafe(capture: CaptureRecord): Promise<void> {
      await settleResponses();
      const profile = PRIVACY_PROFILES[capture.privacyProfile];
      const captureViolations = violations.splice(0);
      for (const body of responseBodies.splice(0)) {
        const key = protectedKey(body, profile.allowedResponseKeys);
        if (key !== undefined) captureViolations.push(`response contains protected key ${key}`);
      }
      if (new URL(page.url()).origin !== applicationOrigin) {
        captureViolations.push("capture page escaped the application origin");
      }
      const visibleText = await page.locator("body").innerText();
      const screenTerm = PROTECTED_SCREEN_TERMS.find((term) => term.test(visibleText));
      if (screenTerm !== undefined) {
        captureViolations.push(`screen text matches protected term ${screenTerm.source}`);
      }
      if (!profile.allowFilledEmail) {
        const hasFilledEmail = await page
          .locator('input[type="email"]')
          .evaluateAll((inputs) =>
            inputs.some((input) => input instanceof HTMLInputElement && input.value.length > 0),
          );
        if (hasFilledEmail) {
          captureViolations.push("capture contains a filled Authentication Email field");
        }
      }
      const hasFilledPassword = await page
        .locator('input[type="password"]')
        .evaluateAll((inputs) =>
          inputs.some((input) => input instanceof HTMLInputElement && input.value.length > 0),
        );
      if (hasFilledPassword) captureViolations.push("capture contains a filled credential field");
      const selectedCount = await page.locator("input:checked").count();
      if (profile.selectedControl === "forbidden" && selectedCount > 0) {
        captureViolations.push("capture must not contain a selected response control");
      }
      if (profile.selectedControl === "required" && selectedCount === 0) {
        captureViolations.push("capture must contain the fictional Student's selected response");
      }
      const statusCount = await page
        .getByRole("heading", { name: /^(Response received|Graded)$/u })
        .count();
      if (profile.statusHeading === "forbidden" && statusCount > 0) {
        captureViolations.push("capture must not contain a submitted-response status");
      }
      if (captureViolations.length > 0) throw new Error(captureViolations.join("; "));
    },
  };
}
