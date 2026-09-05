// capability_registry.ts - truthfulness boundary for declared Ribbon destinations.

import type { ProductRole } from "../../generated/api/ProductRole";
import { productRoleMayAccessRoute, type RouteId } from "../route_contract";
import {
  RIBBON_TASK_CATALOG,
  TAB_CATALOG,
  type RibbonCatalogControl,
  type RibbonDestination,
  type RibbonDestinationId,
} from "./ribbon_catalog";
import type { RibbonRelationshipRequirement } from "./ribbon_schema";

/** A Ribbon control may be drawn only when this result is Available. */
export type RibbonAvailability = "Available" | "Checking" | "Unavailable";

/** Relationship information is the one availability input that may arrive after first paint. */
export type RibbonRelationshipState =
  { readonly kind: "outstanding" } | { readonly kind: "resolved"; readonly allowed: boolean };

/** Stable, reviewable evidence identifiers rather than source-line snapshots. */
export type RibbonEvidence = ReadonlyArray<string>;

/** A usable path has a client operation and server evidence, or a no-server rationale. */
export type BackedCapabilityEvidence =
  | { readonly kind: "registeredHandler"; readonly handler: string }
  | { readonly kind: "noServerCall"; readonly justification: string };

/** Backed evidence is deliberately stricter than a mounted browser route. */
export interface BackedRibbonCapability {
  readonly kind: "backed";
  readonly clientMethod: string;
  readonly serverEvidence: BackedCapabilityEvidence;
  readonly evidence: RibbonEvidence;
}

/** An unbacked entry states why it cannot be truthfully admitted. */
export interface UnbackedRibbonCapability {
  readonly kind: "unbacked";
  readonly reason: string;
  readonly evidence: RibbonEvidence;
}

export type RibbonCapability = BackedRibbonCapability | UnbackedRibbonCapability;

interface RibbonCapabilityEntryBase<Id extends RibbonDestinationId = RibbonDestinationId> {
  readonly id: Id;
  readonly label: string;
  readonly destination: RibbonDestination;
  readonly relationshipRequirement: RibbonRelationshipRequirement;
}

/** The route is mandatory for a backed capability, which makes role-ceiling checks possible. */
export interface BackedRibbonCapabilityEntry extends RibbonCapabilityEntryBase {
  readonly routeId: RouteId;
  readonly capability: BackedRibbonCapability;
}

export interface UnbackedRibbonCapabilityEntry extends RibbonCapabilityEntryBase {
  /** Present when the catalog has a declared route; absent for a truthful future destination. */
  readonly routeId?: RouteId;
  readonly capability: UnbackedRibbonCapability;
}

export type RibbonCapabilityEntry = BackedRibbonCapabilityEntry | UnbackedRibbonCapabilityEntry;

function isBackedRibbonCapabilityEntry(
  entry: RibbonCapabilityEntry,
): entry is BackedRibbonCapabilityEntry {
  return entry.capability.kind === "backed";
}

type RibbonCapabilityDeclarations = Readonly<
  Record<RibbonDestinationId, UnbackedRibbonCapability | BackedRibbonCapability>
>;

const CATALOG_CONTROLS: ReadonlyArray<RibbonCatalogControl<RibbonDestinationId>> = [
  ...TAB_CATALOG,
  ...RIBBON_TASK_CATALOG,
];

function catalogControlFor(id: RibbonDestinationId): RibbonCatalogControl<RibbonDestinationId> {
  const control = CATALOG_CONTROLS.find((candidate) => candidate.id === id);
  if (control === undefined) {
    throw new Error(`Ribbon capability registry has no catalog control for ${id}.`);
  }
  return control;
}

function routeIdFor(destination: RibbonDestination): RouteId | undefined {
  return destination.kind === "route" ? destination.routeId : undefined;
}

function requireNonBlankCapabilityText(value: string, description: string): void {
  if (value.trim().length === 0) {
    throw new Error(`Ribbon capability ${description} must not be blank.`);
  }
}

function validateCapabilityProof(id: RibbonDestinationId, capability: RibbonCapability): void {
  if (capability.evidence.length === 0) {
    throw new Error(`Ribbon capability ${id} must include reviewable evidence.`);
  }
  for (const evidence of capability.evidence) {
    requireNonBlankCapabilityText(evidence, `${id} evidence`);
  }

  if (capability.kind === "unbacked") {
    requireNonBlankCapabilityText(capability.reason, `${id} unbacked reason`);
    return;
  }

  requireNonBlankCapabilityText(capability.clientMethod, `${id} client method`);
  if (capability.serverEvidence.kind === "registeredHandler") {
    requireNonBlankCapabilityText(capability.serverEvidence.handler, `${id} registered handler`);
    return;
  }
  requireNonBlankCapabilityText(
    capability.serverEvidence.justification,
    `${id} no-server-call justification`,
  );
}

/**
 * Joins a capability claim to its catalog control and rejects incomplete proof at the sole
 * declaration-to-registry boundary. This validates UI truthfulness only; it is not authorization.
 */
export function createRibbonCapabilityEntry(
  id: RibbonDestinationId,
  capability: RibbonCapability,
  relationshipRequirement: RibbonRelationshipRequirement = "none",
): RibbonCapabilityEntry {
  const control = catalogControlFor(id);
  const routeId = routeIdFor(control.destination);
  validateCapabilityProof(id, capability);
  if (capability.kind === "backed") {
    if (routeId === undefined) {
      throw new Error(`Backed Ribbon capability ${id} must have a declared route.`);
    }
    return {
      id,
      label: control.label,
      destination: control.destination,
      routeId,
      relationshipRequirement,
      capability,
    };
  }
  return {
    id,
    label: control.label,
    destination: control.destination,
    ...(routeId === undefined ? {} : { routeId }),
    relationshipRequirement,
    capability,
  };
}

const NO_TEACHING_HANDLER = [
  "crates/server/src/composition.rs::production_router_from_env",
] as const;

/**
 * Capability declarations are total over the catalog, but they do not invent paths or authority.
 * Every current destination is unbacked: production registers entry/auth/health only, not a
 * complete teaching or data handler for any Ribbon destination.
 */
const CAPABILITY_DECLARATIONS = {
  courses: {
    kind: "unbacked",
    reason: "Course listing has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/api/application_api.tsx::ApiClient.listCourses"],
  },
  questionLibrary: {
    kind: "unbacked",
    reason: "Question Library search has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/api/application_api.tsx::ApiClient.searchQuestionLibrary",
    ],
  },
  blueprintCourses: {
    kind: "unbacked",
    reason: "Blueprint Course listing has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/api/application_api.tsx::ApiClient.listBlueprintCourses",
    ],
  },
  assignments: {
    kind: "unbacked",
    reason: "Assignment listing has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/api/application_api.tsx::ApiClient.listAssignments"],
  },
  students: {
    kind: "unbacked",
    reason: "Course roster has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/api/application_api.tsx::ApiClient.listCourseRoster"],
  },
  gradebook: {
    kind: "unbacked",
    reason: "Calculated Gradebook has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/api/application_api.tsx::ApiClient.getCalculatedGradebook",
    ],
  },
  teachingOperations: {
    kind: "unbacked",
    reason: "Teaching Operations has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/pages/teaching_operations_page.tsx::TeachingOperationsPage",
    ],
  },
  blueprintUpdates: {
    kind: "unbacked",
    reason: "Blueprint Updates has no declared route, page, client method, or registered handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::blueprintUpdates"],
  },
  courseSetup: {
    kind: "unbacked",
    reason: "Course Setup is a future destination identity, not a declared usable path.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::courseSetup"],
  },
  attempt: {
    kind: "unbacked",
    reason: "Assignment Attempt screen has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/api/application_api.tsx::ApiClient.getAssignmentAttemptScreen",
    ],
  },
  instructorAccounts: {
    kind: "unbacked",
    reason:
      "Instructor Accounts has no declared route, page, client method, or registered handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::instructorAccounts"],
  },
  allQuestions: {
    kind: "unbacked",
    reason: "All Questions has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/api/application_api.tsx::ApiClient.searchQuestionLibrary",
    ],
  },
  myQuestions: {
    kind: "unbacked",
    reason: "My Questions has no declared route state or registered production handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::myQuestions"],
  },
  myQuestionDrafts: {
    kind: "unbacked",
    reason:
      "My Question Drafts is a retained future destination without a declared usable path " +
      "or registered production handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::myQuestionDrafts"],
  },
  starred: {
    kind: "unbacked",
    reason: "Starred has no declared route, page, client method, or registered handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::starred"],
  },
  watched: {
    kind: "unbacked",
    reason: "Watched has no declared route, page, client method, or registered handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::watched"],
  },
  assignmentOverview: {
    kind: "unbacked",
    reason: "Assignment workspace Overview has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/routes.ts::routeComponents"],
  },
  assignmentQuestions: {
    kind: "unbacked",
    reason: "Assignment workspace Questions has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/routes.ts::routeComponents"],
  },
  assignmentPolicies: {
    kind: "unbacked",
    reason: "Assignment workspace Policies has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/routes.ts::routeComponents"],
  },
  assignmentGradingOperations: {
    kind: "unbacked",
    reason:
      "Assignment workspace Grading Operations has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/routes.ts::routeComponents"],
  },
  assignmentStudentView: {
    kind: "unbacked",
    reason: "Assignment workspace Student View has no registered production teaching/data handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/routes.ts::routeComponents"],
  },
  gradeSettings: {
    kind: "unbacked",
    reason: "Grade Settings has no registered production teaching/data handler.",
    evidence: [
      ...NO_TEACHING_HANDLER,
      "src/api/application_api.tsx::ApiClient.getCourseGradeSettings",
    ],
  },
  appearance: {
    kind: "unbacked",
    reason: "Appearance has no declared usable route and no registered production handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::appearance"],
  },
  backToAssignments: {
    kind: "unbacked",
    reason: "Back to Assignments leads to a surface without a registered production handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/api/application_api.tsx::ApiClient.listAssignments"],
  },
} as const satisfies RibbonCapabilityDeclarations;

/** Total, catalog-joined registry. Visibility is a UI ceiling, never authorization. */
export const CAPABILITY_REGISTRY: Readonly<Record<RibbonDestinationId, RibbonCapabilityEntry>> =
  Object.freeze(
    Object.fromEntries(
      (Object.keys(CAPABILITY_DECLARATIONS) as ReadonlyArray<RibbonDestinationId>).map((id) => [
        id,
        createRibbonCapabilityEntry(id, CAPABILITY_DECLARATIONS[id]),
      ]),
    ) as Record<RibbonDestinationId, RibbonCapabilityEntry>,
  );

/** A withheld Checking entry never becomes a pending visual control. */
export function isRibbonAvailabilityVisible(
  availability: RibbonAvailability,
): availability is "Available" {
  return availability === "Available";
}

/**
 * Returns the admission ceiling for one declared destination. This does not authorize a request:
 * route access boundaries and the server retain that authority (ASVS 8.3.1).
 */
export function ribbonAvailability(
  entry: RibbonCapabilityEntry,
  productRole: ProductRole,
  relationshipState: RibbonRelationshipState,
): RibbonAvailability {
  if (!isBackedRibbonCapabilityEntry(entry)) {
    return "Unavailable";
  }
  if (!productRoleMayAccessRoute(entry.routeId, productRole)) {
    return "Unavailable";
  }
  if (entry.relationshipRequirement !== "none") {
    if (relationshipState.kind === "outstanding") {
      return "Checking";
    }
    if (!relationshipState.allowed) {
      return "Unavailable";
    }
  }
  return "Available";
}

/** Convenience predicate for rendering code; Checking is withheld by construction. */
export function isRibbonEntryVisible(
  entry: RibbonCapabilityEntry,
  productRole: ProductRole,
  relationshipState: RibbonRelationshipState,
): boolean {
  return isRibbonAvailabilityVisible(ribbonAvailability(entry, productRole, relationshipState));
}
