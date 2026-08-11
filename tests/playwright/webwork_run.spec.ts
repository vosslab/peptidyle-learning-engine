// webwork_run.spec.ts - opt-in browser acceptance for the private WebWork MC path.
//
// Selector contract:
// - src/app.tsx: local-development credential form and accessible sign-in control.
// - src/pages/course_list_page.tsx and course_assignments_page.tsx: course and assignment cards.
// - src/pages/assignment_overview_page.tsx: visible practice-run entry.
// - src/components/response_widget.tsx: native radio controls and Enter submission.
// - src/components/feedback_panel.tsx: correctness-only feedback projection.

import { configuredLiveWebworkInputs, mockPreviewServerEnabled } from "../../playwright.config";
import { decodeAuthSession } from "../../src/api/decoders";
import { liveInputsFromEnvironment, type LiveWebworkInputs } from "./webwork_live_config";

import { expect, test, type Locator, type Page } from "@playwright/test";

// Validate activation before Playwright creates the live browser context.
test.describe.configure({ mode: "serial" });

const WEBWORK_COURSE_TITLE = "PLE WebWork pilot E2E course";
const WEBWORK_ASSIGNMENT_TITLE = "PLE WebWork pilot E2E assignment";
const WEBWORK_QUESTION_TITLE = "Biochemistry: Identify hydrophobic compounds from formulas";
const HYDROPHILIC_LABELS = [
  "acetate",
  "water",
  "erythrose",
  "glucose",
  "sucrose",
  "glycerol",
  "glycine",
  "ethanol",
  "methanol",
  "ammonia",
  "sodium chloride",
  "phosphoric acid",
  "urea",
] as const;
const HYDROCARBON_LABELS = [
  "benzene",
  "toluene",
  "ethylene",
  "propane",
  "butane",
  "cyclohexane",
  "hexane",
  "octane",
] as const;
const PROTECTED_UPSTREAM_JSON_KEYS = [
  "problemSource",
  "rawProblemSource",
  "uriEncodedProblemSource",
  "sourceFilePath",
  "pathToProblemFile",
  "courseID",
  "passwd",
  "AnSwEr",
  "hidden_input_field",
  "real_webwork_SITE_URL",
  "real_webwork_FORM_ACTION_URL",
  "problemUUID",
  "psvn",
] as const;
const PROTECTED_UPSTREAM_JSON_KEY_SET = new Set<string>(PROTECTED_UPSTREAM_JSON_KEYS);
const RAW_PG_SOURCE_FINGERPRINTS = ["BEGIN_PGML", "loadMacros(", "RadioButtons("] as const;
const FORBIDDEN_URL_PATHS = ["/webwork2/", "render_rpc", "/render-api"] as const;

interface BrowserTrace {
  readonly locations: ReadonlyArray<BrowserLocation>;
  readonly urlSafetyHits: ReadonlyArray<UrlSafetyHit>;
  readonly protectedJsonHits: ReadonlyArray<ProtectedJsonHit>;
  readonly rawSourceFingerprintHits: ReadonlyArray<RawSourceFingerprintHit>;
  readonly completedRunCorrectness: boolean[];
  readonly waitForBodies: () => Promise<void>;
}

interface BrowserLocation {
  readonly category: "request" | "response";
  readonly origin: string;
  readonly pathname: string;
}

interface UrlSafetyHit {
  readonly category: "request" | "response";
  readonly kind: "forbiddenPath" | "userinfo";
  readonly label: string;
  readonly location: BrowserLocation;
}

interface ProtectedJsonHit {
  readonly category: "requestJson" | "responseJson";
  readonly key: string;
  readonly location: BrowserLocation;
}

interface RawSourceFingerprintHit {
  readonly category: "requestJson" | "responseJson";
  readonly fingerprint: string;
  readonly location: BrowserLocation;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isUnknownArray(value: unknown): value is unknown[] {
  return Array.isArray(value);
}

type JsonParseResult =
  { readonly kind: "parsed"; readonly value: unknown } | { readonly kind: "invalid" };

function parsedJson(value: string): JsonParseResult {
  try {
    return { kind: "parsed", value: JSON.parse(value) as unknown };
  } catch {
    return { kind: "invalid" };
  }
}

function inspectJsonForPrivateMaterial(
  value: unknown,
  category: ProtectedJsonHit["category"],
  location: BrowserLocation,
  protectedJsonHits: ProtectedJsonHit[],
  rawSourceFingerprintHits: RawSourceFingerprintHit[],
): void {
  if (isUnknownArray(value)) {
    for (const item of value) {
      inspectJsonForPrivateMaterial(
        item,
        category,
        location,
        protectedJsonHits,
        rawSourceFingerprintHits,
      );
    }
    return;
  }
  if (isRecord(value)) {
    for (const [key, nested] of Object.entries(value)) {
      if (PROTECTED_UPSTREAM_JSON_KEY_SET.has(key)) {
        protectedJsonHits.push({ category, key, location });
      }
      inspectJsonForPrivateMaterial(
        nested,
        category,
        location,
        protectedJsonHits,
        rawSourceFingerprintHits,
      );
    }
    return;
  }
  if (typeof value === "string") {
    for (const fingerprint of RAW_PG_SOURCE_FINGERPRINTS) {
      if (value.includes(fingerprint)) {
        rawSourceFingerprintHits.push({ category, fingerprint, location });
      }
    }
  }
}

function attachTrace(page: Page): BrowserTrace {
  const locations: BrowserLocation[] = [];
  const urlSafetyHits: UrlSafetyHit[] = [];
  const protectedJsonHits: ProtectedJsonHit[] = [];
  const rawSourceFingerprintHits: RawSourceFingerprintHit[] = [];
  const completedRunCorrectness: boolean[] = [];
  const bodyReaders: Promise<void>[] = [];
  function inspectUrl(url: string, category: BrowserLocation["category"]): BrowserLocation {
    const parsed = new URL(url);
    const location = { category, origin: parsed.origin, pathname: parsed.pathname };
    locations.push(location);
    if (parsed.username !== "" || parsed.password !== "") {
      urlSafetyHits.push({ category, kind: "userinfo", label: "userinfo", location });
    }
    const pathname = parsed.pathname.toLowerCase();
    for (const forbiddenPath of FORBIDDEN_URL_PATHS) {
      if (pathname.includes(forbiddenPath)) {
        urlSafetyHits.push({ category, kind: "forbiddenPath", label: forbiddenPath, location });
      }
    }
    for (const [key] of parsed.searchParams) {
      if (PROTECTED_UPSTREAM_JSON_KEY_SET.has(key)) {
        protectedJsonHits.push({
          category: `${category}Json`,
          key,
          location,
        });
      }
    }
    return location;
  }
  page.on("request", (request) => {
    if (!request.url().startsWith("http")) return;
    const location = inspectUrl(request.url(), "request");
    const body = request.postData();
    if (body !== null) {
      const parsed = parsedJson(body);
      if (parsed.kind === "parsed") {
        inspectJsonForPrivateMaterial(
          parsed.value,
          "requestJson",
          location,
          protectedJsonHits,
          rawSourceFingerprintHits,
        );
      }
    }
  });
  page.on("response", (response) => {
    if (!response.url().startsWith("http")) return;
    const location = inspectUrl(response.url(), "response");
    const bodyReader = response.text().then(
      (body) => {
        const parsed = parsedJson(body);
        if (parsed.kind === "parsed") {
          inspectJsonForPrivateMaterial(
            parsed.value,
            "responseJson",
            location,
            protectedJsonHits,
            rawSourceFingerprintHits,
          );
        }
        if (!/\/api\/runs\/[0-9a-f-]+\/summary/u.test(response.url())) return;
        if (parsed.kind !== "parsed") return;
        const summary = parsed.value;
        if (!isRecord(summary) || !("outcomes" in summary)) return;
        const outcomes = summary["outcomes"];
        if (!isRecord(outcomes) || !("items" in outcomes)) return;
        const items = outcomes["items"];
        if (!isUnknownArray(items) || items.length !== 1) return;
        const item = items[0];
        if (!isRecord(item) || !("feedback" in item)) return;
        const feedback = item["feedback"];
        if (isRecord(feedback) && "correctness" in feedback) {
          if (typeof feedback["correctness"] === "boolean") {
            completedRunCorrectness.push(feedback["correctness"]);
          }
        }
      },
      () => undefined,
    );
    bodyReaders.push(bodyReader);
  });
  async function waitForBodies(): Promise<void> {
    await Promise.all(bodyReaders);
  }
  return {
    locations,
    urlSafetyHits,
    protectedJsonHits,
    rawSourceFingerprintHits,
    completedRunCorrectness,
    waitForBodies,
  };
}

async function signInAsDeterministicStudent(page: Page, inputs: LiveWebworkInputs): Promise<void> {
  const login = await page.context().request.post(new URL("/api/auth/login", inputs.baseUrl).href, {
    data: { credential: inputs.studentCredential },
    failOnStatusCode: false,
  });
  if (login.status() !== 200) {
    throw new Error(`local browser-context login returned HTTP ${login.status()}`);
  }
  const sessionResponse = page.waitForResponse(
    (response) => new URL(response.url()).pathname === "/api/auth/session",
  );
  await page.goto(inputs.baseUrl);
  const session = await sessionResponse;
  if (session.status() !== 200) {
    throw new Error(`authenticated browser session returned HTTP ${session.status()}`);
  }
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
}

async function openWebworkAssignment(page: Page, inputs: LiveWebworkInputs): Promise<void> {
  const course = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: WEBWORK_COURSE_TITLE }),
  });
  await expect(course).toHaveCount(1);
  await course.getByRole("link", { name: "Open course" }).click();

  const assignment = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: WEBWORK_ASSIGNMENT_TITLE }),
  });
  await expect(assignment).toHaveCount(1);
  const review = assignment.getByRole("link", { name: "Review assignment" });
  await expect(review).toHaveAttribute("href", new RegExp(inputs.assignmentId, "u"));
  await review.click();
  await expect(page.getByRole("heading", { name: WEBWORK_ASSIGNMENT_TITLE })).toBeVisible();
  await page.getByRole("button", { name: "Start or resume practice" }).click();
  await expect(page.getByRole("heading", { name: WEBWORK_QUESTION_TITLE })).toBeVisible();
}

async function tabTo(page: Page, target: Locator, limit = 40): Promise<void> {
  for (let index = 0; index < limit; index += 1) {
    if (await target.evaluate((element) => document.activeElement === element)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error("Tab did not reach the first visible WebWork response radio");
}

function visibleChoiceIndex(labels: readonly string[], candidates: readonly string[]): number {
  const index = labels.findIndex((label) =>
    candidates.some((candidate) => label.toLowerCase().includes(candidate)),
  );
  if (index < 0) {
    throw new Error("the rendered question did not contain a required visible pedagogical choice");
  }
  return index;
}

async function selectChoiceWithTabAndArrows(page: Page, targetIndex: number): Promise<void> {
  const radios = page.getByRole("radio");
  const count = await radios.count();
  expect(count).toBe(5);
  for (const radio of await radios.all()) {
    await expect(radio).not.toBeChecked();
  }
  const first = radios.first();
  await tabTo(page, first);
  await expect(first).toBeFocused();

  // With no initial selection, a first arrow establishes the native radio
  // selection. Five presses return to visible index zero when it is the target.
  const presses = targetIndex === 0 ? count : targetIndex;
  for (let index = 0; index < presses; index += 1) {
    await page.keyboard.press("ArrowDown");
  }
  const selected = radios.nth(targetIndex);
  await expect(selected).toBeFocused();
  await expect(selected).toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
}

async function submitWithEnter(
  page: Page,
  expectedCorrectness: "Correct" | "Not quite",
): Promise<void> {
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Feedback" })).toBeVisible();
  await expect(page.getByRole("heading", { name: expectedCorrectness })).toBeVisible();
  await expect(page.getByRole("button", { name: "Continue" })).toBeFocused();
}

async function completeRun(
  page: Page,
  expectedCorrectness: "Correct" | "Not quite",
): Promise<void> {
  await page.getByRole("button", { name: "Continue" }).click();
  const summary = page.locator(".attempt-summary");
  await expect(
    summary.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();
  await expect(summary.getByRole("heading", { name: expectedCorrectness })).toBeVisible();
}

async function startFreshRun(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Start another practice run" }).click();
  await expect(page.getByRole("heading", { name: WEBWORK_QUESTION_TITLE })).toBeVisible();
  await expect(page.getByRole("radio")).toHaveCount(5);
}

async function inspectBrowserStorage(page: Page): Promise<{
  readonly protectedKeys: ReadonlyArray<string>;
  readonly rawSourceFingerprints: ReadonlyArray<string>;
}> {
  return await page.evaluate(
    ({ protectedKeys, fingerprints }) => {
      const protectedKeySet = new Set<string>(protectedKeys);
      const foundKeys = new Set<string>();
      const foundFingerprints = new Set<string>();
      function inspect(value: unknown): void {
        if (Array.isArray(value)) {
          value.forEach(inspect);
        } else if (typeof value === "object" && value !== null) {
          for (const [key, nested] of Object.entries(value)) {
            if (protectedKeySet.has(key)) foundKeys.add(key);
            inspect(nested);
          }
        } else if (typeof value === "string") {
          fingerprints.forEach((fingerprint) => {
            if (value.includes(fingerprint)) foundFingerprints.add(fingerprint);
          });
        }
      }
      for (const storage of [localStorage, sessionStorage]) {
        for (let index = 0; index < storage.length; index += 1) {
          const key = storage.key(index);
          if (key === null) continue;
          if (protectedKeySet.has(key)) foundKeys.add(key);
          const parsed = storage.getItem(key);
          if (parsed === null) continue;
          try {
            inspect(JSON.parse(parsed));
          } catch {
            fingerprints.forEach((fingerprint) => {
              if (parsed.includes(fingerprint)) foundFingerprints.add(fingerprint);
            });
          }
        }
      }
      return {
        protectedKeys: [...foundKeys],
        rawSourceFingerprints: [...foundFingerprints],
      };
    },
    {
      protectedKeys: [...PROTECTED_UPSTREAM_JSON_KEYS],
      fingerprints: [...RAW_PG_SOURCE_FINGERPRINTS],
    },
  );
}

function assertPrivateMaterialNeverCrossedBrowser(
  inputs: LiveWebworkInputs,
  trace: BrowserTrace,
  storage: {
    readonly protectedKeys: ReadonlyArray<string>;
    readonly rawSourceFingerprints: ReadonlyArray<string>;
  },
): void {
  const expectedOrigin = new URL(inputs.baseUrl).origin;
  for (const location of trace.locations) {
    expect(location.origin).toBe(expectedOrigin);
  }
  expect(trace.urlSafetyHits).toEqual([]);
  expect(trace.protectedJsonHits).toEqual([]);
  expect(trace.rawSourceFingerprintHits).toEqual([]);
  expect(storage.protectedKeys).toEqual([]);
  expect(storage.rawSourceFingerprints).toEqual([]);
}

test("live-required configuration is explicit and rejects incomplete activation", () => {
  expect(liveInputsFromEnvironment({ PLE_WEBWORK_LIVE_REQUIRED: "0" }, () => "")).toBeUndefined();
  expect(() => liveInputsFromEnvironment({ PLE_WEBWORK_LIVE_REQUIRED: "1" }, () => "")).toThrow(
    "PLE_WEBWORK_LIVE_BASE_URL",
  );
  expect(() =>
    liveInputsFromEnvironment(
      {
        PLE_WEBWORK_LIVE_REQUIRED: "1",
        PLE_WEBWORK_LIVE_BASE_URL: "http://127.0.0.1:3000",
        PLE_WEBWORK_LIVE_ASSIGNMENT_ID: "0198e000-0000-7000-8000-000000000001",
      },
      () => "",
      () => undefined,
    ),
  ).toThrow("PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE");
  const completeLiveEnvironment = {
    PLE_WEBWORK_LIVE_REQUIRED: "1",
    PLE_WEBWORK_LIVE_BASE_URL: "http://127.0.0.1:3000",
    PLE_WEBWORK_LIVE_ASSIGNMENT_ID: "0198e000-0000-7000-8000-000000000001",
    PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE: "fixture-local-login.txt",
  };
  expect(() =>
    liveInputsFromEnvironment(
      completeLiveEnvironment,
      () => "student=fixture_credential_that_is_long_enough_for_the_local_provider",
      () => {
        throw new Error("unsafe metadata");
      },
    ),
  ).toThrow("unsafe metadata");
  expect(
    liveInputsFromEnvironment(
      completeLiveEnvironment,
      () => "student=fixture_credential_that_is_long_enough_for_the_local_provider",
      () => undefined,
    ),
  ).toBeDefined();
  expect(mockPreviewServerEnabled({ PLE_WEBWORK_LIVE_REQUIRED: "1" })).toBe(false);
  expect(mockPreviewServerEnabled({ PLE_WEBWORK_LIVE_REQUIRED: "0" })).toBe(true);
  if (process.env["PLE_WEBWORK_LIVE_REQUIRED"] === "1") {
    expect(configuredLiveWebworkInputs).toBeDefined();
  }
});

test("browser session decoding accepts canonical deterministic local UUIDs", () => {
  const session = decodeAuthSession({
    authenticated: true,
    tenant: "00000000-0000-0000-0000-000000000100",
    user: {
      id: "00000000-0000-0000-0000-000000000102",
      displayName: "Local Student",
      roles: ["student"],
    },
  });

  expect(session.tenant).toBe("00000000-0000-0000-0000-000000000100");
  expect(session.user.roles).toEqual(["student"]);
});

test.describe("private live WebWork browser acceptance", () => {
  test.skip(
    configuredLiveWebworkInputs === undefined,
    "requires the explicit private live-stack invocation",
  );

  test("live WebWork run is answer-free, keyboard-operable, and PLE-only", async ({ page }) => {
    const inputs = configuredLiveWebworkInputs;
    if (inputs === undefined) {
      throw new Error("the declaration-time live WebWork skip did not apply");
    }

    await signInAsDeterministicStudent(page, inputs);
    const trace = attachTrace(page);
    await openWebworkAssignment(page, inputs);

    const radios = page.getByRole("radio");
    await expect(radios).toHaveCount(5);
    const labels = await page.locator(".choice-card").allTextContents();
    expect(labels).toHaveLength(5);
    for (const radio of await radios.all()) {
      await expect(radio).toHaveAccessibleName(/\S/u);
    }

    const wrongIndex = visibleChoiceIndex(labels, HYDROPHILIC_LABELS);
    await selectChoiceWithTabAndArrows(page, wrongIndex);
    await submitWithEnter(page, "Not quite");
    await completeRun(page, "Not quite");

    await startFreshRun(page);
    const correctLabels = await page.locator(".choice-card").allTextContents();
    const correctIndex = visibleChoiceIndex(correctLabels, HYDROCARBON_LABELS);
    await selectChoiceWithTabAndArrows(page, correctIndex);
    await submitWithEnter(page, "Correct");
    await completeRun(page, "Correct");

    await trace.waitForBodies();
    // Deferred feedback remains hidden until this one-question run completes;
    // the completed receipt and summary then expose only policy-approved results.
    expect(trace.completedRunCorrectness).toEqual([false, true]);
    const storage = await inspectBrowserStorage(page);
    assertPrivateMaterialNeverCrossedBrowser(inputs, trace, storage);
  });
});
