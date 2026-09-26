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
  "student_feedback_released",
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

export interface CoverageExceptions {
  readonly routes: ReadonlyArray<CoverageEntry>;
  readonly ribbonDestinations: ReadonlyArray<CoverageEntry>;
}

const IDENTIFIER_PATTERN = /^[a-z][a-z0-9_]*$/u;
const ROLE_PATH_PATTERN =
  /^(public|instructor|student|sysadmin)\/(?:(?:laptop|tablet|phone|square)\/)?[a-z0-9_-]+\.png$/u;
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
    throw new Error(`${label} path must be a semantic filename in its role folder`);
  }
  const pathParts = artifactPath.split("/");
  if (pathParts.length === 3 && pathParts[1] !== decoded["viewport"]) {
    throw new Error(`${label} viewport folder must match the capture viewport`);
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
    enumValue(capture.privacyProfile, PRIVACY_PROFILE_IDS, `capture ${capture.id} privacyProfile`);
    enumValue(capture.viewport, VIEWPORT_IDS, `capture ${capture.id} viewport`);
    enumValue(capture.role, ROLE_IDS, `capture ${capture.id} role`);
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

export function decodeCoverageExceptions(value: unknown): CoverageExceptions {
  const decoded = objectValue(value, "coverage exceptions");
  exactKeys(decoded, ["routes", "ribbonDestinations"], [], "coverage exceptions");
  if (!Array.isArray(decoded["routes"]) || !Array.isArray(decoded["ribbonDestinations"])) {
    throw new Error("coverage exceptions ledgers must be arrays");
  }
  return {
    routes: decoded["routes"].map((entry, index) =>
      decodeCoverageEntry(entry, `route exception ${String(index)}`, "id"),
    ),
    ribbonDestinations: decoded["ribbonDestinations"].map((entry, index) =>
      decodeCoverageEntry(entry, `Ribbon exception ${String(index)}`, "id"),
    ),
  };
}

export async function loadCoverageExceptions(exceptionsPath: string): Promise<CoverageExceptions> {
  const source = await readFile(exceptionsPath, "utf8");
  return decodeCoverageExceptions(JSON.parse(source) as unknown);
}

function exceptionById(
  entries: ReadonlyArray<CoverageEntry>,
  id: string,
): CoverageEntry | undefined {
  return entries.find((entry) => entry.id === id);
}

export function computeCoverage(
  captures: ReadonlyArray<CaptureRecord>,
  exceptions: CoverageExceptions,
): CaptureManifest["coverage"] {
  const routes = ROUTE_CONTRACT.map((route): CoverageEntry => {
    const captureIds = captures
      .filter((capture) => capture.routeId === route.id)
      .map((capture) => capture.id);
    if (captureIds.length > 0) {
      return { id: route.id, status: "captured", captureIds };
    }
    const listed = exceptionById(exceptions.routes, route.id);
    if (listed === undefined) {
      throw new Error(`uncovered route ${route.id} is not listed in coverage exceptions`);
    }
    return listed;
  });
  const ribbonIds = [
    ...TAB_CATALOG.map((control) => ({
      key: `tab:${control.id}`,
      destination: control.destination,
    })),
    ...RIBBON_TASK_CATALOG.map((control) => ({
      key: `task:${control.id}`,
      destination: control.destination,
    })),
  ];
  const ribbonDestinations = ribbonIds.map(({ key, destination }): CoverageEntry => {
    if (destination.kind === "route") {
      const captureIds = captures
        .filter((capture) => capture.routeId === destination.routeId)
        .map((capture) => capture.id);
      if (captureIds.length > 0) {
        return { id: key, status: "captured", captureIds };
      }
    }
    const listed = exceptionById(exceptions.ribbonDestinations, key);
    if (listed === undefined) {
      throw new Error(`uncovered Ribbon destination ${key} is not listed in coverage exceptions`);
    }
    return listed;
  });
  return { routes, ribbonDestinations };
}

export function assignGalleryOrder(
  captures: ReadonlyArray<CaptureRecord>,
): ReadonlyArray<CaptureRecord> {
  const grouped: CaptureRecord[] = [];
  for (const role of ROLE_IDS) {
    const roleCaptures = captures.filter((capture) => capture.role === role);
    const areas = [...new Set(roleCaptures.map((capture) => capture.area))];
    for (const area of areas) {
      const areaCaptures = roleCaptures.filter((capture) => capture.area === area);
      const workflows = [...new Set(areaCaptures.map((capture) => capture.workflow))];
      for (const workflow of workflows) {
        grouped.push(...areaCaptures.filter((capture) => capture.workflow === workflow));
      }
    }
  }
  return grouped.map((capture, index) => ({
    ...capture,
    gallery: { ...capture.gallery, order: index + 1 },
  }));
}

export function generateManifest(
  captures: ReadonlyArray<CaptureRecord>,
  exceptions: CoverageExceptions,
): CaptureManifest {
  const ordered = assignGalleryOrder(captures);
  validateCaptureRelationships(ordered);
  const manifest: CaptureManifest = {
    schemaVersion: 2,
    evidenceClass: "reproducible-rendered",
    rebuildCommand: "./devel/capture_screenshots.sh",
    viewports: CANONICAL_VIEWPORTS,
    captures: ordered,
    coverage: computeCoverage(ordered, exceptions),
  };
  validateCoverage(manifest);
  return manifest;
}

function encodeCoverageEntry(
  entry: CoverageEntry,
  identityField: "routeId" | "id",
): Record<string, unknown> {
  if (entry.status === "captured") {
    return { [identityField]: entry.id, status: entry.status, captureIds: entry.captureIds };
  }
  if (entry.status === "covered_by") {
    return {
      [identityField]: entry.id,
      status: entry.status,
      target: entry.target,
      reason: entry.reason,
    };
  }
  return { [identityField]: entry.id, status: entry.status, reason: entry.reason };
}

export function manifestJson(manifest: CaptureManifest): string {
  const encoded = {
    schemaVersion: manifest.schemaVersion,
    evidenceClass: manifest.evidenceClass,
    rebuildCommand: manifest.rebuildCommand,
    viewports: manifest.viewports,
    captures: manifest.captures,
    coverage: {
      routes: manifest.coverage.routes.map((entry) => encodeCoverageEntry(entry, "routeId")),
      ribbonDestinations: manifest.coverage.ribbonDestinations.map((entry) =>
        encodeCoverageEntry(entry, "id"),
      ),
    },
  };
  return `${JSON.stringify(encoded, null, 2)}\n`;
}

export function assertManifestMatches(
  committed: CaptureManifest,
  generated: CaptureManifest,
): void {
  if (manifestJson(committed) !== manifestJson(generated)) {
    throw new Error("committed screenshot manifest drifted from scenario generation");
  }
}
