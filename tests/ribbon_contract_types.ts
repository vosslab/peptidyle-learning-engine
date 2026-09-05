// Compile-only public-boundary assertions for the pure Ribbon contract.

import { ROUTE_CONTRACT, type RouteContract } from "../src/route_contract";
import {
  buildRoutePath,
  deriveRibbonModel,
  type RibbonContextLabels,
  type RibbonRouteState,
  type RibbonViewerIdentity,
} from "../src/ribbon/ribbon_contract";

const route = ROUTE_CONTRACT.find(
  (candidate) => candidate.id === "courseAssignments",
) as RouteContract;
const params = { courseRef: "C-1" } as const;
const routeState = { route, params } satisfies RibbonRouteState;
const viewerIdentity = { productRole: "instructor" } satisfies RibbonViewerIdentity;
const contextLabels = { accountLabel: "Neil Voss" } satisfies RibbonContextLabels;

// Positive calls ensure the negative cases below cannot pass due to a broken API.
deriveRibbonModel(routeState, viewerIdentity, contextLabels);
buildRoutePath("courseAssignments", params);

const withResource = { productRole: "instructor" as const, scopeResource: { courseId: "1" } };
// @ts-expect-error A resource cannot cross the viewer identity boundary through a variable.
deriveRibbonModel(routeState, withResource, contextLabels);

const withPromise = { accountLabel: "Neil Voss", pendingScope: Promise.resolve("C-1") };
// @ts-expect-error A Promise cannot cross the context label boundary through a variable.
deriveRibbonModel(routeState, viewerIdentity, withPromise);

const withAccessor = {
  accountLabel: "Neil Voss",
  courseTitle: (): string => "Molecular Biology",
};
// @ts-expect-error A Solid-style accessor cannot cross the context label boundary.
deriveRibbonModel(routeState, viewerIdentity, withAccessor);

const withCallback = { accountLabel: "Neil Voss", onSignOut: (): undefined => undefined };
// @ts-expect-error A callback cannot cross the pure context label boundary.
deriveRibbonModel(routeState, viewerIdentity, withCallback);

const withSessionData = { productRole: "instructor" as const, sessionData: { token: "secret" } };
// @ts-expect-error Session data cannot cross the viewer identity boundary through a variable.
deriveRibbonModel(routeState, withSessionData, contextLabels);

const routeStateWithResource = { route, params, scopeResource: { courseId: "1" } };
// @ts-expect-error Route state is exact and cannot carry a resource.
deriveRibbonModel(routeStateWithResource, viewerIdentity, contextLabels);

const paramsWithProjection = { courseRef: "C-1", courseProjection: { id: "1" } };
const routeStateWithProjection = { route, params: paramsWithProjection };
// @ts-expect-error Declared parameters are exact strings, not a projection carrier.
deriveRibbonModel(routeStateWithProjection, viewerIdentity, contextLabels);

const routeIdFromUntrustedText: string = "courseAssignments";
// @ts-expect-error The normal route builder accepts a declared RouteId, not arbitrary text.
buildRoutePath(routeIdFromUntrustedText, params);

const paramsWithSessionData = { courseRef: "C-1", sessionData: { token: "secret" } };
// @ts-expect-error The normal route builder rejects variable extra route data.
buildRoutePath("courseAssignments", paramsWithSessionData);
