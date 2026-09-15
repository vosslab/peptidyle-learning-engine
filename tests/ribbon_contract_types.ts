// Compile-only public-boundary assertions for the pure Ribbon contract.

import { ROUTE_CONTRACT } from "../src/route_contract";
import {
  buildRoutePath,
  deriveRibbonModel,
  type RibbonContextLabels,
  type RibbonRouteState,
  type RibbonViewerIdentity,
} from "../src/ribbon/ribbon_contract";

const route = ROUTE_CONTRACT.find((candidate) => candidate.id === "courseAssessments");
if (route === undefined)
  throw new Error("Course Assessments route is required by the Ribbon contract.");
const params = { courseRef: "CI7K3M2Q" } as const;
const routeState = { route, params } satisfies RibbonRouteState;
const viewerIdentity = { productRole: "instructor" } satisfies RibbonViewerIdentity;
const contextLabels = {} satisfies RibbonContextLabels;

// Positive calls ensure the negative cases below cannot pass due to a broken API.
deriveRibbonModel(routeState, viewerIdentity, contextLabels);
buildRoutePath("courseAssessments", params);

const withResource = { productRole: "instructor" as const, scopeResource: { courseId: "1" } };
// @ts-expect-error A resource cannot cross the viewer identity boundary through a variable.
deriveRibbonModel(routeState, withResource, contextLabels);

const withPromise = {
  courseShortName: "Molecular Biology",
  pendingScope: Promise.resolve("CI7K3M2Q"),
};
// @ts-expect-error A Promise cannot cross the context label boundary through a variable.
deriveRibbonModel(routeState, viewerIdentity, withPromise);

const withAccessor = {
  courseShortName: (): string => "Molecular Biology",
};
// @ts-expect-error A Solid-style accessor cannot cross the context label boundary.
deriveRibbonModel(routeState, viewerIdentity, withAccessor);

const withCallback = {
  courseShortName: "Molecular Biology",
  onSignOut: (): undefined => undefined,
};
// @ts-expect-error A callback cannot cross the pure context label boundary.
deriveRibbonModel(routeState, viewerIdentity, withCallback);

const withSessionData = { productRole: "instructor" as const, sessionData: { token: "secret" } };
// @ts-expect-error Session data cannot cross the viewer identity boundary through a variable.
deriveRibbonModel(routeState, withSessionData, contextLabels);

const routeStateWithResource = { route, params, scopeResource: { courseId: "1" } };
// @ts-expect-error Route state is exact and cannot carry a resource.
deriveRibbonModel(routeStateWithResource, viewerIdentity, contextLabels);

const paramsWithProjection = { courseRef: "CI7K3M2Q", courseProjection: { id: "1" } };
const routeStateWithProjection = { route, params: paramsWithProjection };
// @ts-expect-error Declared parameters are exact strings, not a projection carrier.
deriveRibbonModel(routeStateWithProjection, viewerIdentity, contextLabels);

const routeIdFromUntrustedText: string = "courseAssessments";
// @ts-expect-error The normal route builder accepts a declared RouteId, not arbitrary text.
buildRoutePath(routeIdFromUntrustedText, params);

const paramsWithSessionData = { courseRef: "CI7K3M2Q", sessionData: { token: "secret" } };
// @ts-expect-error The normal route builder rejects variable extra route data.
buildRoutePath("courseAssessments", paramsWithSessionData);
