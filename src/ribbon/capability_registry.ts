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
 * The Question Library is backed by its registered Instructor routes; every
 * other teaching destination remains unavailable until its complete path lands.
 */
const CAPABILITY_DECLARATIONS = {
  courses: {
    kind: "backed",
    clientMethod: "ApiClient.listCourseInstances",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/course_instance.rs::course_instance_router",
    },
    evidence: [
      "crates/server/src/course_instance.rs::course_instance_router",
      "src/api/http_client/course_instance.ts::createCourseInstanceClient",
    ],
  },
  questions: {
    kind: "backed",
    clientMethod: "ApiClient.searchQuestionLibrary",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/question_library.rs::question_library_router",
    },
    evidence: [
      "crates/server/src/question_library.rs::question_library_router",
      "src/api/application_api.tsx::ApiClient.searchQuestionLibrary",
    ],
  },
  productAssignments: {
    kind: "backed",
    clientMethod: "ApiClient.listAssignmentsDueSoon",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/assignment_release.rs::assignment_release_router",
    },
    evidence: [
      "crates/server/src/assignment_release.rs::assignment_release_router",
      "src/api/http_client/assignment_release.ts::createLiveAssignmentReleaseClient",
      "src/pages/assignments_due_soon_page.tsx::AssignmentsDueSoonPage",
    ],
  },
  assignments: {
    kind: "backed",
    clientMethod: "ApiClient.getLiveAssignmentWorkspace",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/assignment_release.rs::assignment_release_router",
    },
    evidence: [
      "crates/server/src/assignment_release.rs::assignment_release_router",
      "src/api/http_client/assignment_release.ts::createLiveAssignmentReleaseClient",
    ],
  },
  studentAssignments: {
    kind: "backed",
    clientMethod: "ApiClient.listLiveStudentAssignments",
    serverEvidence: {
      kind: "registeredHandler",
      handler:
        "crates/server/src/live_student_course_landing.rs::live_student_course_landing_router",
    },
    evidence: [
      "crates/server/src/live_student_course_landing.rs::live_student_course_landing_router",
      "src/api/http_client/live_student_course_landing.ts::createLiveStudentCourseLandingClient",
    ],
  },
  students: {
    kind: "backed",
    clientMethod: "ApiClient.getLiveCourseRoster",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/course_roster.rs::course_roster_router",
    },
    evidence: [
      "crates/server/src/course_roster.rs::course_roster_router",
      "src/api/http_client/course_roster.ts::createLiveCourseRosterClient",
    ],
  },
  gradebook: {
    kind: "backed",
    clientMethod: "ApiClient.getLiveDemoGradebook",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/live_gradebook.rs::live_gradebook_router",
    },
    evidence: [
      "crates/server/src/live_gradebook.rs::live_gradebook_router",
      "src/api/http_client/live_gradebook.ts::createLiveDemoGradebookClient",
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
    kind: "backed",
    clientMethod: "ApiClient.startLiveAssignment",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/assignment_delivery.rs::assignment_delivery_router",
    },
    evidence: [
      "crates/server/src/assignment_delivery.rs::assignment_delivery_router",
      "src/api/http_client/assignment_attempt_issuance.ts::createLiveAssignmentAttemptIssuanceClient",
      "src/pages/assignment_attempt_page.tsx::AssignmentAttemptPage",
    ],
  },
  instructorAccounts: {
    kind: "backed",
    clientMethod: "ApiClient.listInstructorAccounts",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/instructor_account.rs::instructor_account_router",
    },
    evidence: [
      "crates/server/src/instructor_account.rs::instructor_account_router",
      "src/api/http_client/instructor_account.ts::createInstructorAccountClient",
    ],
  },
  supportRoster: {
    kind: "backed",
    clientMethod: "ApiClient.readSupportCourseRoster",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/support_capability.rs::support_capability_router",
    },
    evidence: [
      "crates/server/src/support_capability.rs::support_capability_router",
      "src/api/http_client/support_roster.ts::createSupportCapabilityClient",
    ],
  },
  myBlueprintCourses: {
    kind: "backed",
    clientMethod: "ApiClient.listBlueprintCourses",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/blueprint_course.rs::blueprint_course_router",
    },
    evidence: [
      "crates/server/src/blueprint_course.rs::blueprint_course_router",
      "src/api/application_api.tsx::ApiClient.listBlueprintCourses",
    ],
  },
  myActiveCourses: {
    kind: "unbacked",
    reason: "My Active Courses remains unavailable until the Course activity capability lands.",
    evidence: ["src/ribbon/ribbon_catalog.ts::myActiveCourses"],
  },
  myInactiveCourses: {
    kind: "unbacked",
    reason: "My Inactive Courses remains unavailable until retention derives inactive Courses.",
    evidence: ["src/ribbon/ribbon_catalog.ts::myInactiveCourses"],
  },
  searchPublicBlueprintCourses: {
    kind: "unbacked",
    reason: "Public Blueprint Course search has no declared route or authorized handler.",
    evidence: ["src/ribbon/ribbon_catalog.ts::searchPublicBlueprintCourses"],
  },
  myQuestions: {
    kind: "unbacked",
    reason: "My Questions has no declared route state or registered production handler.",
    evidence: [...NO_TEACHING_HANDLER, "src/ribbon/ribbon_catalog.ts::myQuestions"],
  },
  myDraftQuestions: {
    kind: "backed",
    clientMethod: "QuestionDraftsPage::listDrafts",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/authoring.rs::authoring_router",
    },
    evidence: [
      "crates/server/src/authoring.rs::authoring_router",
      "src/pages/question_drafts_page.tsx::QuestionDraftsPage",
    ],
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
  searchQuestionLibrary: {
    kind: "backed",
    clientMethod: "ApiClient.searchQuestionLibrary",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/question_library.rs::question_library_router",
    },
    evidence: [
      "crates/server/src/question_library.rs::question_library_router",
      "src/api/application_api.tsx::ApiClient.searchQuestionLibrary",
    ],
  },
  browseQuestionLibrary: {
    kind: "backed",
    clientMethod: "ApiClient.searchQuestionLibrary",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/question_library.rs::question_library_router",
    },
    evidence: [
      "crates/server/src/question_library.rs::question_library_router",
      "src/api/application_api.tsx::ApiClient.searchQuestionLibrary",
    ],
  },
  assignmentsDueSoon: {
    kind: "backed",
    clientMethod: "ApiClient.listAssignmentsDueSoon",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/assignment_release.rs::assignment_release_router",
    },
    evidence: [
      "crates/server/src/assignment_release.rs::assignment_release_router",
      "src/api/http_client/assignment_release.ts::createLiveAssignmentReleaseClient",
      "src/pages/assignments_due_soon_page.tsx::AssignmentsDueSoonPage",
    ],
  },
  assignmentTemplates: {
    kind: "unbacked",
    reason: "My Assignment Templates has no declared route or registered handler.",
    evidence: ["src/ribbon/ribbon_catalog.ts::assignmentTemplates"],
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
    kind: "backed",
    clientMethod: "ApiClient.getLiveAssignmentWorkspace",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "GET /api/course-instances/{course}/assignments/{assignment}",
    },
    evidence: [
      "src/routes.ts::routeComponents",
      "src/pages/assignment_workspace/assignment_workspace_policies_page.tsx::AssignmentWorkspacePoliciesPage",
      "src/api/http_client/assignment_release.ts::getLiveAssignmentWorkspace",
    ],
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
    kind: "backed",
    clientMethod: "ApiClient.getCourseAppearanceView",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/course_appearance.rs::course_appearance_router",
    },
    evidence: [
      "src/api/application_api.tsx::createApplicationApi",
      "src/api/http_client/response.ts::courseAppearanceView",
      "src/api/http_client/response.ts::updateCourseTheme",
      "src/api/http_client/response.ts::uploadCourseBanner",
      "src/api/http_client/response.ts::setCourseBanner",
      "src/api/http_client/response.ts::removeCourseBanner",
      "src/pages/course_appearance_page.tsx::CourseAppearancePage",
      "crates/server/src/course_appearance.rs::course_appearance_router",
      "crates/learning-data-access/src/course_theme.rs::CourseThemeStore",
      "crates/learning-data-access/src/course_banner.rs::CourseBannerStore",
    ],
  },
  backToAssignments: {
    kind: "backed",
    clientMethod: "ApiClient.getLiveAssignmentAccess",
    serverEvidence: {
      kind: "registeredHandler",
      handler: "crates/server/src/assignment_delivery.rs::assignment_delivery_router",
    },
    evidence: [
      "crates/server/src/assignment_delivery.rs::assignment_delivery_router",
      "src/api/http_client/assignment_attempt_issuance.ts::createLiveAssignmentAttemptIssuanceClient",
      "src/pages/assignment_overview_page.tsx::AssignmentOverviewPage",
      "src/app.tsx::ribbonParamsFor",
    ],
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
