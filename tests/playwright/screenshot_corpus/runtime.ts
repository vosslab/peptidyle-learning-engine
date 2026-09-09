// runtime.ts - shared browser and capture assertions for screenshot scenarios.

import path from "node:path";
import { mkdir, readFile } from "node:fs/promises";

import type { Browser, BrowserContext, Page } from "playwright";

import { routeContractForPathname } from "../../../src/route_contract";
import { TAB_CATALOG } from "../../../src/ribbon/ribbon_catalog";
import {
  CANONICAL_VIEWPORTS,
  type CaptureManifest,
  type CaptureRecord,
  type ViewportId,
} from "./manifest";
import { monitorCapturePrivacy, type PrivacyMonitor } from "./privacy_profiles";

export interface SeededReferences {
  readonly course: string;
  readonly assignment: string;
}

export interface CaptureSession {
  readonly context: BrowserContext;
  readonly page: Page;
  readonly pageErrors: Error[];
  readonly privacy: PrivacyMonitor;
  readonly viewport: ViewportId;
}

export interface ScenarioRuntime {
  readonly browser: Browser;
  readonly entryUrl: URL;
  readonly outputRoot: string;
  readonly manifest: CaptureManifest;
  readonly references: SeededReferences;
  readonly producedPaths: Set<string>;
  readonly record: (scenario: string, checkpoint: string) => CaptureRecord;
  readonly open: (record: CaptureRecord) => Promise<CaptureSession>;
  readonly capture: (session: CaptureSession, record: CaptureRecord) => Promise<void>;
  readonly close: (session: CaptureSession) => Promise<void>;
}

export function requireEntryUrl(argument: string | undefined): URL {
  if (argument === undefined) throw new Error("capture environment did not report its entry URL");
  const url = new URL(argument);
  if (
    url.protocol !== "https:" ||
    url.hostname !== "localhost" ||
    url.pathname !== "/sign-in" ||
    url.search !== "" ||
    url.hash !== ""
  ) {
    throw new Error("capture entry must be the local HTTPS sign-in URL");
  }
  return url;
}

export async function loadSeededReferences(reportPath: string): Promise<SeededReferences> {
  const parsed = JSON.parse(await readFile(reportPath, "utf8")) as unknown;
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Live Demo Course report must be an object");
  }
  const report = parsed as Record<string, unknown>;
  const course = report["course_reference"];
  const assignment = report["assignment_reference"];
  if (
    typeof course !== "string" ||
    !/^C-[1-9][0-9]{0,9}$/u.test(course) ||
    typeof assignment !== "string" ||
    !/^A-[1-9][0-9]{0,9}$/u.test(assignment)
  ) {
    throw new Error("Live Demo Course report lacks canonical Course and Assignment references");
  }
  return { course, assignment };
}

function requireNoPageErrors(session: CaptureSession): void {
  if (session.pageErrors.length > 0) {
    throw new Error(
      `capture page raised browser errors: ${session.pageErrors.map((error) => error.message).join("; ")}`,
    );
  }
}

function assertRoute(page: Page, capture: CaptureRecord): void {
  const url = new URL(page.url());
  const route = routeContractForPathname(url.pathname);
  if (route?.id !== capture.routeId) {
    throw new Error(
      `${capture.id} reached ${route?.id ?? "no declared route"} at ${url.pathname}; ` +
        `expected ${capture.routeId}`,
    );
  }
}

async function assertRibbon(page: Page, capture: CaptureRecord): Promise<void> {
  if (capture.role === "public" || capture.privacyProfile === "authorization_denial") return;
  const route = routeContractForPathname(new URL(page.url()).pathname);
  if (route === undefined) throw new Error(`${capture.id} has no route for Ribbon verification`);
  const ribbon = page.getByRole("region", { name: "PLE application Ribbon", exact: true });
  await ribbon.waitFor();
  const tabId = route.ribbon.tab;
  if (tabId === undefined) return;
  const control = TAB_CATALOG.find((candidate) => candidate.id === tabId);
  if (control === undefined) throw new Error(`${capture.id} route names an unknown Ribbon tab`);
  const selected = ribbon
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: control.label, exact: true });
  await selected.waitFor();
  if ((await selected.getAttribute("aria-current")) !== "page") {
    throw new Error(`${capture.id} does not select its ${control.label} Ribbon tab`);
  }
}

function captureTarget(outputRoot: string, capture: CaptureRecord): string {
  const resolvedRoot = path.resolve(outputRoot);
  const target = path.resolve(resolvedRoot, capture.path);
  // ASVS 5.3.1: manifest decoding and this containment check close file publication.
  if (!target.startsWith(`${resolvedRoot}${path.sep}`))
    throw new Error("capture target escaped root");
  return target;
}

export function createScenarioRuntime(options: {
  readonly browser: Browser;
  readonly entryUrl: URL;
  readonly outputRoot: string;
  readonly manifest: CaptureManifest;
  readonly references: SeededReferences;
}): ScenarioRuntime {
  const producedPaths = new Set<string>();

  function record(scenario: string, checkpoint: string): CaptureRecord {
    const matches = options.manifest.captures.filter(
      (capture) => capture.scenario === scenario && capture.checkpoint === checkpoint,
    );
    if (matches.length !== 1 || matches[0] === undefined) {
      throw new Error(`expected one manifest record for ${scenario}:${checkpoint}`);
    }
    return matches[0];
  }

  async function open(capture: CaptureRecord): Promise<CaptureSession> {
    const viewport = CANONICAL_VIEWPORTS[capture.viewport];
    const context = await options.browser.newContext({
      colorScheme: "light",
      deviceScaleFactor: 1,
      hasTouch: viewport.mobile,
      ignoreHTTPSErrors: true,
      isMobile: viewport.mobile,
      reducedMotion: "reduce",
      viewport: { width: viewport.width, height: viewport.height },
    });
    const page = await context.newPage();
    const pageErrors: Error[] = [];
    page.on("pageerror", (error) => pageErrors.push(error));
    page.setDefaultTimeout(30_000);
    page.setDefaultNavigationTimeout(60_000);
    await page.goto(options.entryUrl.href, { waitUntil: "commit" });
    await page
      .getByRole("heading", {
        level: 1,
        name: "Explore Peptidyle Learning Engine",
        exact: true,
      })
      .waitFor();
    // The sign-in page has completed its bootstrap. Inspect only workflow traffic, matching the
    // durable browser scenarios instead of treating application startup as screenshot evidence.
    const privacy = monitorCapturePrivacy(page, options.entryUrl.origin);
    return { context, page, pageErrors, privacy, viewport: capture.viewport };
  }

  async function capture(session: CaptureSession, captureRecord: CaptureRecord): Promise<void> {
    if (session.viewport !== captureRecord.viewport) {
      throw new Error(`${captureRecord.id} uses a session with the wrong viewport`);
    }
    console.log(`Checking screenshot checkpoint ${captureRecord.id}`);
    await session.page.evaluate(async () => document.fonts.ready);
    assertRoute(session.page, captureRecord);
    await assertRibbon(session.page, captureRecord);
    console.log(`Checking screenshot privacy ${captureRecord.id}`);
    await session.privacy.assertSafe(captureRecord);
    requireNoPageErrors(session);
    const target = captureTarget(options.outputRoot, captureRecord);
    await mkdir(path.dirname(target), { recursive: true });
    await session.page.screenshot({
      animations: "disabled",
      caret: "hide",
      path: target,
    });
    producedPaths.add(captureRecord.path);
    console.log(`Captured ${captureRecord.path}`);
  }

  async function close(session: CaptureSession): Promise<void> {
    requireNoPageErrors(session);
    await session.context.close();
  }

  return {
    browser: options.browser,
    entryUrl: options.entryUrl,
    outputRoot: options.outputRoot,
    manifest: options.manifest,
    references: options.references,
    producedPaths,
    record,
    open,
    capture,
    close,
  };
}

export function requireProducedClosure(runtime: ScenarioRuntime): void {
  const expected = new Set(runtime.manifest.captures.map((capture) => capture.path));
  const missing = [...expected].filter((artifactPath) => !runtime.producedPaths.has(artifactPath));
  const unexpected = [...runtime.producedPaths].filter(
    (artifactPath) => !expected.has(artifactPath),
  );
  if (missing.length > 0 || unexpected.length > 0) {
    throw new Error(
      `scenario output is not closed; missing=${missing.join(",") || "none"}; ` +
        `unexpected=${unexpected.join(",") || "none"}`,
    );
  }
}
