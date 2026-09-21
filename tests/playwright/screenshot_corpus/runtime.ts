// runtime.ts - shared browser and capture assertions for screenshot scenarios.

import path from "node:path";
import { mkdir } from "node:fs/promises";

import type { Browser, BrowserContext, Page } from "playwright";

import { routeContractForPathname, type RouteId } from "../../../src/route_contract";
import { TAB_CATALOG } from "../../../src/ribbon/ribbon_catalog";
import { CANONICAL_VIEWPORTS, type CaptureRecord, type ViewportId } from "./manifest";
import { monitorCapturePrivacy, type PrivacyMonitor } from "./privacy_profiles";
import type { CaptureDeclaration, ScenarioDefinition } from "./scenario_types";

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
  readonly producedPaths: Set<string>;
  readonly producedCaptures: CaptureRecord[];
  readonly open: (checkpoint: string) => Promise<CaptureSession>;
  readonly captureCheckpoint: (session: CaptureSession, checkpoint: string) => Promise<void>;
  readonly close: (session: CaptureSession) => Promise<void>;
}

export function captureIdentity(
  role: ScenarioDefinition["role"],
  checkpoint: string,
  viewport: ViewportId,
): { readonly id: string; readonly path: string } {
  const semanticName = checkpoint.replace(new RegExp(`_${viewport}$`, "u"), "");
  const pathPrefix = semanticName === checkpoint ? role : `${role}/${viewport}`;
  return { id: `${role}_${checkpoint}`, path: `${pathPrefix}/${semanticName}.png` };
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

function requireNoPageErrors(session: CaptureSession): void {
  if (session.pageErrors.length > 0) {
    throw new Error(
      `capture page raised browser errors: ${session.pageErrors.map((error) => error.message).join("; ")}`,
    );
  }
}

function observedRoute(page: Page, captureId: string): RouteId {
  const url = new URL(page.url());
  const route = routeContractForPathname(url.pathname);
  if (route === undefined) {
    throw new Error(`${captureId} reached no declared route at ${url.pathname}`);
  }
  return route.id;
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
  if (!target.startsWith(`${resolvedRoot}${path.sep}`))
    throw new Error("capture target escaped root");
  return target;
}

function declarationFor(scenario: ScenarioDefinition, checkpoint: string): CaptureDeclaration {
  const matches = scenario.captures.filter((capture) => capture.checkpoint === checkpoint);
  if (matches.length !== 1 || matches[0] === undefined) {
    throw new Error(`expected one capture declaration for ${scenario.id}:${checkpoint}`);
  }
  return matches[0];
}

export function createScenarioRuntime(options: {
  readonly browser: Browser;
  readonly entryUrl: URL;
  readonly outputRoot: string;
  readonly scenario: ScenarioDefinition;
}): ScenarioRuntime {
  const producedPaths = new Set<string>();
  const producedCaptures: CaptureRecord[] = [];

  async function openSession(checkpoint: string): Promise<CaptureSession> {
    const declaration = declarationFor(options.scenario, checkpoint);
    const viewport = CANONICAL_VIEWPORTS[declaration.viewport];
    const context = await options.browser.newContext({
      colorScheme: "light",
      deviceScaleFactor: 1,
      hasTouch: viewport.mobile,
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
    const privacy = monitorCapturePrivacy(page, options.entryUrl.origin);
    return { context, page, pageErrors, privacy, viewport: declaration.viewport };
  }

  async function captureCheckpoint(session: CaptureSession, checkpoint: string): Promise<void> {
    const declaration = declarationFor(options.scenario, checkpoint);
    if (session.viewport !== declaration.viewport) {
      throw new Error(`${checkpoint} uses a session with the wrong viewport`);
    }
    const identity = captureIdentity(options.scenario.role, checkpoint, declaration.viewport);
    const routeId = observedRoute(session.page, identity.id);
    const captureRecord: CaptureRecord = {
      id: identity.id,
      path: identity.path,
      role: options.scenario.role,
      routeId,
      area: declaration.area,
      workflow: declaration.workflow,
      state: declaration.state,
      scenario: options.scenario.id,
      checkpoint,
      viewport: declaration.viewport,
      privacyProfile: declaration.privacyProfile,
      gallery: {
        order: producedCaptures.length + 1,
        caption: declaration.caption,
        featured: declaration.featured === true,
      },
    };
    console.log(`Checking screenshot checkpoint ${captureRecord.id}`);
    await session.page.evaluate(async () => document.fonts.ready);
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
    producedCaptures.push(captureRecord);
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
    producedPaths,
    producedCaptures,
    open: openSession,
    captureCheckpoint,
    close,
  };
}

export function requireProducedClosure(
  runtime: ScenarioRuntime,
  scenario: ScenarioDefinition,
): void {
  const expected = new Set(
    scenario.captures.map(
      (capture) => captureIdentity(scenario.role, capture.checkpoint, capture.viewport).path,
    ),
  );
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
