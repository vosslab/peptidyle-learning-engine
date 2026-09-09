// manifest.ts - strict schema and coverage model for the durable screenshot corpus.

import { readFile } from "node:fs/promises";

import { ROUTE_CONTRACT, type RouteId } from "../../../src/route_contract";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../../../src/ribbon/ribbon_catalog";

export const ROLE_IDS = ["public", "instructor", "student", "sysadmin"] as const;
export type ScreenshotRole = (typeof ROLE_IDS)[number];

export const VIEWPORT_IDS = ["laptop", "tablet", "phone", "square"] as const;
export type ViewportId = (typeof VIEWPORT_IDS)[number];

export const PRIVACY_PROFILE_IDS = [
  "public",
  "instructor_answer_free",
  "student_unanswered",
  "student_selected_response",
  "student_self",
  "authorization_denial",
  "sysadmin_account",
  "sysadmin_scoped_roster",
] as const;
export type PrivacyProfileId = (typeof PRIVACY_PROFILE_IDS)[number];

export interface ViewportProfile {
  readonly width: number;
  readonly height: number;
  readonly mobile: boolean;
}

export const CANONICAL_VIEWPORTS: Readonly<Record<ViewportId, ViewportProfile>> = {
  laptop: { width: 1280, height: 800, mobile: false },
  tablet: { width: 800, height: 1280, mobile: true },
  phone: { width: 393, height: 852, mobile: true },
  square: { width: 800, height: 800, mobile: false },
};

export interface GalleryMetadata {
  readonly order: number;
  readonly caption: string;
  readonly featured: boolean;
}

export interface CaptureRecord {
  readonly id: string;
  readonly path: string;
  readonly role: ScreenshotRole;
  readonly routeId: RouteId;
  readonly area: string;
  readonly workflow: string;
  readonly state: string;
  readonly scenario: string;
  readonly checkpoint: string;
  readonly viewport: ViewportId;
  readonly privacyProfile: PrivacyProfileId;
  readonly gallery: GalleryMetadata;
}

export type CoverageEntry =
  | {
      readonly id: string;
      readonly status: "captured";
      readonly captureIds: ReadonlyArray<string>;
    }
  | {
      readonly id: string;
      readonly status: "covered_by";
      readonly target: string;
      readonly reason: string;
    }
  | { readonly id: string; readonly status: "deferred"; readonly reason: string };

export interface CaptureManifest {
  readonly schemaVersion: 2;
  readonly evidenceClass: "reproducible-rendered";
  readonly rebuildCommand: "./devel/capture_screenshots.sh";
  readonly viewports: Readonly<Record<ViewportId, ViewportProfile>>;
  readonly captures: ReadonlyArray<CaptureRecord>;
  readonly coverage: {
    readonly routes: ReadonlyArray<CoverageEntry>;
    readonly ribbonDestinations: ReadonlyArray<CoverageEntry>;
  };
}

export interface ScenarioRegistration {
  readonly id: string;
  readonly checkpoints: ReadonlyArray<string>;
}

const IDENTIFIER_PATTERN = /^[a-z][a-z0-9_]*$/u;
const ROLE_PATH_PATTERN = /^(public|instructor|student|sysadmin)\/[a-z0-9_]+\.png$/u;
const TEXT_PATTERN = /^[\x20-\x7e]+$/u;

function objectValue(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(
  value: Record<string, unknown>,
  required: ReadonlyArray<string>,
  optional: ReadonlyArray<string>,
  label: string,
): void {
  const allowed = new Set([...required, ...optional]);
  const actual = Object.keys(value);
  const missing = required.filter((key) => !Object.prototype.hasOwnProperty.call(value, key));
  const unexpected = actual.filter((key) => !allowed.has(key));
  if (missing.length > 0 || unexpected.length > 0) {
    throw new Error(
      `${label} has invalid fields; missing=${missing.join(",") || "none"}; ` +
        `unexpected=${unexpected.join(",") || "none"}`,
    );
  }
}

function stringValue(value: unknown, label: string): string {
  if (typeof value !== "string" || value.length === 0 || !TEXT_PATTERN.test(value)) {
    throw new Error(`${label} must be nonempty ASCII text`);
  }
  return value;
}

function identifier(value: unknown, label: string): string {
  const decoded = stringValue(value, label);
  if (!IDENTIFIER_PATTERN.test(decoded))
    throw new Error(`${label} must be a snake-case identifier`);
  return decoded;
}

function enumValue<const Item extends string>(
  value: unknown,
  choices: ReadonlyArray<Item>,
  label: string,
): Item {
  if (typeof value !== "string" || !choices.includes(value as Item)) {
    throw new Error(`${label} is not in the closed vocabulary`);
  }
  return value as Item;
}

function positiveInteger(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${label} must be a positive integer`);
  }
  return value;
}

function decodeViewports(value: unknown): Readonly<Record<ViewportId, ViewportProfile>> {
  const decoded = objectValue(value, "manifest viewports");
  exactKeys(decoded, VIEWPORT_IDS, [], "manifest viewports");
  const entries = VIEWPORT_IDS.map((id): readonly [ViewportId, ViewportProfile] => {
    const profile = objectValue(decoded[id], `viewport ${id}`);
    exactKeys(profile, ["width", "height", "mobile"], [], `viewport ${id}`);
    const mobile = profile["mobile"];
    if (typeof mobile !== "boolean") throw new Error(`viewport ${id} mobile must be boolean`);
    const actual: ViewportProfile = {
      width: positiveInteger(profile["width"], `viewport ${id} width`),
      height: positiveInteger(profile["height"], `viewport ${id} height`),
      mobile,
    };
    const canonical = CANONICAL_VIEWPORTS[id];
    if (
      actual.width !== canonical.width ||
      actual.height !== canonical.height ||
      actual.mobile !== canonical.mobile
    ) {
      throw new Error(`viewport ${id} does not match the canonical dimensions`);
    }
    return [id, actual];
  });
  return Object.fromEntries(entries) as Readonly<Record<ViewportId, ViewportProfile>>;
}

function decodeGallery(value: unknown, label: string): GalleryMetadata {
  const decoded = objectValue(value, label);
  exactKeys(decoded, ["order", "caption"], ["featured"], label);
  if (decoded["featured"] !== undefined && typeof decoded["featured"] !== "boolean") {
    throw new Error(`${label} featured must be boolean`);
  }
  return {
    order: positiveInteger(decoded["order"], `${label} order`),
    caption: stringValue(decoded["caption"], `${label} caption`),
    featured: decoded["featured"] === true,
  };
}

function decodeCapture(value: unknown, index: number): CaptureRecord {
  const label = `capture ${String(index)}`;
  const decoded = objectValue(value, label);
  exactKeys(
    decoded,
    [
      "id",
      "path",
      "role",
      "routeId",
      "area",
      "workflow",
      "state",
      "scenario",
      "checkpoint",
      "viewport",
      "privacyProfile",
      "gallery",
    ],
    [],
    label,
  );
  const role = enumValue(decoded["role"], ROLE_IDS, `${label} role`);
  const artifactPath = stringValue(decoded["path"], `${label} path`);
  if (!ROLE_PATH_PATTERN.test(artifactPath) || artifactPath.split("/")[0] !== role) {
    throw new Error(`${label} path must be one flat semantic filename in its role folder`);
  }
  const routeIds = ROUTE_CONTRACT.map((route) => route.id);
  const routeId = enumValue(decoded["routeId"], routeIds, `${label} routeId`);
  return {
    id: identifier(decoded["id"], `${label} id`),
    path: artifactPath,
    role,
    routeId,
    area: stringValue(decoded["area"], `${label} area`),
    workflow: stringValue(decoded["workflow"], `${label} workflow`),
    state: stringValue(decoded["state"], `${label} state`),
    scenario: identifier(decoded["scenario"], `${label} scenario`),
    checkpoint: identifier(decoded["checkpoint"], `${label} checkpoint`),
    viewport: enumValue(decoded["viewport"], VIEWPORT_IDS, `${label} viewport`),
    privacyProfile: enumValue(
      decoded["privacyProfile"],
      PRIVACY_PROFILE_IDS,
      `${label} privacyProfile`,
    ),
    gallery: decodeGallery(decoded["gallery"], `${label} gallery`),
  };
}

function decodeCoverageEntry(
  value: unknown,
  label: string,
  identityField: "routeId" | "id",
): CoverageEntry {
  const decoded = objectValue(value, label);
  const entryId = stringValue(decoded[identityField], `${label} ${identityField}`);
  const status = enumValue(
    decoded["status"],
    ["captured", "covered_by", "deferred"] as const,
    `${label} status`,
  );
  if (status === "captured") {
    exactKeys(decoded, [identityField, "status", "captureIds"], [], label);
    if (!Array.isArray(decoded["captureIds"]) || decoded["captureIds"].length === 0) {
      throw new Error(`${label} captured coverage needs captureIds`);
    }
    return {
      id: entryId,
      status,
      captureIds: decoded["captureIds"].map((item, index) =>
        identifier(item, `${label} captureIds ${String(index)}`),
      ),
    };
  }
  if (status === "covered_by") {
    exactKeys(decoded, [identityField, "status", "target", "reason"], [], label);
    return {
      id: entryId,
      status,
      target: stringValue(decoded["target"], `${label} target`),
      reason: concreteReason(decoded["reason"], label),
    };
  }
  exactKeys(decoded, [identityField, "status", "reason"], [], label);
  return { id: entryId, status, reason: concreteReason(decoded["reason"], label) };
}

function concreteReason(value: unknown, label: string): string {
  const reason = stringValue(value, `${label} reason`);
  if (reason.length < 20) throw new Error(`${label} needs a concrete capability reason`);
  return reason;
}

function decodeCoverage(value: unknown): CaptureManifest["coverage"] {
  const decoded = objectValue(value, "manifest coverage");
  exactKeys(decoded, ["routes", "ribbonDestinations"], [], "manifest coverage");
  if (!Array.isArray(decoded["routes"]) || !Array.isArray(decoded["ribbonDestinations"])) {
    throw new Error("manifest coverage ledgers must be arrays");
  }
  return {
    routes: decoded["routes"].map((entry, index) =>
      decodeCoverageEntry(entry, `route coverage ${String(index)}`, "routeId"),
    ),
    ribbonDestinations: decoded["ribbonDestinations"].map((entry, index) =>
      decodeCoverageEntry(entry, `Ribbon coverage ${String(index)}`, "id"),
    ),
  };
}

function requireUnique(values: ReadonlyArray<string>, label: string): void {
  const seen = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) throw new Error(`${label} contains duplicate ${value}`);
    seen.add(value);
  }
}

function validateCaptureRelationships(captures: ReadonlyArray<CaptureRecord>): void {
  requireUnique(
    captures.map((capture) => capture.id),
    "capture IDs",
  );
  requireUnique(
    captures.map((capture) => capture.path),
    "capture paths",
  );
  requireUnique(
    captures.map((capture) => `${capture.scenario}:${capture.checkpoint}`),
    "scenario checkpoints",
  );
  requireUnique(
    captures.map((capture) => String(capture.gallery.order)),
    "gallery orders",
  );
  for (const capture of captures) {
    const route = ROUTE_CONTRACT.find((candidate) => candidate.id === capture.routeId);
    if (route === undefined) throw new Error(`capture ${capture.id} names an unknown route`);
    if (
      capture.role !== "public" &&
      capture.privacyProfile !== "authorization_denial" &&
      route.requiredProductRoles.length > 0 &&
      !new Set<string>(route.requiredProductRoles).has(capture.role)
    ) {
      throw new Error(`capture ${capture.id} has a Product Role outside its route contract`);
    }
    if (capture.role === "public" && capture.routeId !== "signIn" && capture.state !== "expired") {
      throw new Error(`public capture ${capture.id} must be sign-in or expired-session recovery`);
    }
  }
}

function validateCoverageSet(
  entries: ReadonlyArray<CoverageEntry>,
  expectedIds: ReadonlyArray<string>,
  captures: ReadonlyArray<CaptureRecord>,
  kind: "route" | "Ribbon",
): void {
  requireUnique(
    entries.map((entry) => entry.id),
    `${kind} coverage IDs`,
  );
  const actual = new Set(entries.map((entry) => entry.id));
  const expected = new Set(expectedIds);
  const missing = expectedIds.filter((id) => !actual.has(id));
  const unexpected = [...actual].filter((id) => !expected.has(id));
  if (missing.length > 0 || unexpected.length > 0) {
    throw new Error(
      `${kind} coverage is not closed; missing=${missing.join(",") || "none"}; ` +
        `unexpected=${unexpected.join(",") || "none"}`,
    );
  }
  const capturesById = new Map(captures.map((capture) => [capture.id, capture]));
  for (const entry of entries) {
    if (entry.status === "captured") {
      requireUnique(entry.captureIds, `${kind} coverage ${entry.id} capture IDs`);
      for (const captureId of entry.captureIds) {
        const capture = capturesById.get(captureId);
        if (capture === undefined) throw new Error(`${kind} coverage names unknown ${captureId}`);
        if (kind === "route" && capture.routeId !== entry.id) {
          throw new Error(`route coverage ${entry.id} names capture for ${capture.routeId}`);
        }
      }
    } else if (entry.status === "covered_by") {
      if (!expected.has(entry.target) || entry.target === entry.id) {
        throw new Error(`${kind} coverage ${entry.id} has an invalid covered_by target`);
      }
    }
  }
}

function validateCoverage(manifest: CaptureManifest): void {
  validateCoverageSet(
    manifest.coverage.routes,
    ROUTE_CONTRACT.map((route) => route.id),
    manifest.captures,
    "route",
  );
  const ribbonIds = [
    ...TAB_CATALOG.map((control) => `tab:${control.id}`),
    ...RIBBON_TASK_CATALOG.map((control) => `task:${control.id}`),
  ];
  validateCoverageSet(manifest.coverage.ribbonDestinations, ribbonIds, manifest.captures, "Ribbon");
}

/**
 * Decode only the closed schema. ASVS 1.5.1, 2.2.1, and 15.3.5: untrusted JSON
 * cannot add executable behavior, fields, paths, or values through coercion.
 */
export function decodeManifest(value: unknown): CaptureManifest {
  const decoded = objectValue(value, "screenshot manifest");
  exactKeys(
    decoded,
    ["schemaVersion", "evidenceClass", "rebuildCommand", "viewports", "captures", "coverage"],
    [],
    "screenshot manifest",
  );
  if (
    decoded["schemaVersion"] !== 2 ||
    decoded["evidenceClass"] !== "reproducible-rendered" ||
    decoded["rebuildCommand"] !== "./devel/capture_screenshots.sh" ||
    !Array.isArray(decoded["captures"])
  ) {
    throw new Error("screenshot manifest has an unsupported top-level contract");
  }
  const manifest: CaptureManifest = {
    schemaVersion: 2,
    evidenceClass: "reproducible-rendered",
    rebuildCommand: "./devel/capture_screenshots.sh",
    viewports: decodeViewports(decoded["viewports"]),
    captures: decoded["captures"].map(decodeCapture),
    coverage: decodeCoverage(decoded["coverage"]),
  };
  validateCaptureRelationships(manifest.captures);
  validateCoverage(manifest);
  return manifest;
}

export async function loadManifest(manifestPath: string): Promise<CaptureManifest> {
  const source = await readFile(manifestPath, "utf8");
  // JSON.parse is data-only; the exact decoder above rejects all undeclared fields.
  return decodeManifest(JSON.parse(source) as unknown);
}

export function validateScenarioClosure(
  manifest: CaptureManifest,
  registrations: ReadonlyArray<ScenarioRegistration>,
): void {
  requireUnique(
    registrations.map((registration) => registration.id),
    "scenario registry IDs",
  );
  const registered = new Map(
    registrations.map((registration) => {
      requireUnique(registration.checkpoints, `scenario ${registration.id} checkpoints`);
      return [registration.id, new Set(registration.checkpoints)] as const;
    }),
  );
  for (const capture of manifest.captures) {
    const checkpoints = registered.get(capture.scenario);
    if (checkpoints === undefined || !checkpoints.has(capture.checkpoint)) {
      throw new Error(
        `capture ${capture.id} has no registered ${capture.scenario}:${capture.checkpoint}`,
      );
    }
  }
  for (const [scenarioId, checkpoints] of registered) {
    for (const checkpoint of checkpoints) {
      if (
        !manifest.captures.some(
          (capture) => capture.scenario === scenarioId && capture.checkpoint === checkpoint,
        )
      ) {
        throw new Error(`registry checkpoint ${scenarioId}:${checkpoint} has no manifest capture`);
      }
    }
  }
}
