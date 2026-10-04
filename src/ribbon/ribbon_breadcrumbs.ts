// ribbon_breadcrumbs.ts - selected Ribbon hierarchy followed by object and page ancestry.

import type { UserRole } from "../../generated/api/UserRole";
import type { RouteParamName } from "../navigation/route_params";
import { parseAssessmentAttemptId } from "../navigation/public_route";
import { userRoleHomePath, type RibbonTabId, type RouteId } from "../route_contract";
import { buildRoutePath, declaredParamNames } from "./ribbon_route_path";
import type {
  RibbonBreadcrumbModel,
  RibbonContextLabels,
  RibbonControlModel,
  RibbonRouteState,
  RibbonTaskAreaModel,
} from "./ribbon_contract";

function canonicalPathForRouteState(routeState: RibbonRouteState): string | undefined {
  const params: Partial<Record<RouteParamName, string>> = {};
  for (const name of declaredParamNames(routeState.route)) {
    const value = routeState.params[name];
    if (value === undefined) return undefined;
    params[name] = value;
  }
  return buildRoutePath(routeState.route.id, params);
}

function breadcrumb(label: string, href: string): RibbonBreadcrumbModel {
  return { label, href, current: false };
}

function collectionTrail(label: string, routeId: RouteId): RibbonBreadcrumbModel[] {
  const href = buildRoutePath(routeId, {});
  return href === undefined ? [] : [breadcrumb(label, href)];
}

function courseBreadcrumb(labels: RibbonContextLabels, href: string): RibbonBreadcrumbModel {
  const label = labels.courseLongName ?? labels.courseShortName ?? "Course";
  const compactLabel = labels.courseShortName;
  return {
    ...breadcrumb(label, href),
    ...(compactLabel === undefined || compactLabel === label ? {} : { compactLabel }),
  };
}

function tierBreadcrumb(
  control: RibbonControlModel,
  labels: RibbonContextLabels,
): RibbonBreadcrumbModel | undefined {
  if (control.href === undefined) return undefined;
  if (control.id.startsWith("studentCourse:")) return courseBreadcrumb(labels, control.href);
  return {
    ...breadcrumb(control.label, control.href),
    ...(control.state === undefined ? {} : { state: control.state }),
  };
}

/** Only object/section ancestry belongs here. Collection ancestry comes from the Ribbon. */
function pageTrail(
  routeState: RibbonRouteState,
  labels: RibbonContextLabels,
  currentHref: string,
  userRole: UserRole,
): ReadonlyArray<RibbonBreadcrumbModel> {
  const page = (label: string): RibbonBreadcrumbModel => breadcrumb(label, currentHref);
  const courseParams = { courseInstanceId: routeState.params.courseInstanceId };
  const courseHref = buildRoutePath(
    userRole === "student" ? "studentCourseLanding" : "courseAssessments",
    courseParams,
  );
  const course = courseHref === undefined ? [] : [courseBreadcrumb(labels, courseHref)];
  const assessmentParams = { ...courseParams, assessmentId: routeState.params.assessmentId };
  const assessmentHref = buildRoutePath(
    userRole === "student" ? "assessmentOverview" : "assessmentWorkspaceOverview",
    assessmentParams,
  );
  const assessmentLabel = labels.assessmentTitle ?? "Assessment";
  const questionLabel = labels.questionTitle ?? "Question";

  switch (routeState.route.id) {
    case "signIn":
    case "courses":
    case "instructorHome":
    case "studentHome":
    case "sysadminHome":
      return [];
    case "profile":
      return [page("Profile settings")];
    case "pendingCourseInvitations":
    case "studentCourseInvitations":
      return [page("Course Invitations")];
    case "studentCourseInvitation":
      return [
        ...collectionTrail(
          "Course Invitations",
          userRole === "student" ? "studentCourseInvitations" : "pendingCourseInvitations",
        ),
        page("Course Invitation"),
      ];
    case "studentCourses":
    case "sysadminCourseInspection":
      return [page("Courses")];
    case "sysadminCourseInspectionDetail":
      return [...collectionTrail("Courses", "sysadminCourseInspection"), page("Course")];
    case "instructorAccounts":
      return [page("Instructor Accounts")];
    case "contentDisciplines":
      return [page("Disciplines")];
    case "instructorInactiveCourses":
      return [page("My Inactive Courses")];
    case "library":
      return [page("Question Library")];
    case "libraryBrowse":
      return [page("Browse Question Library")];
    case "libraryWatchNotifications":
      return [page("Watch notifications")];
    case "myQuestions":
      return [page("My Questions")];
    case "starredQuestions":
      return [page("Starred")];
    case "questionDrafts":
      return [page("My Draft Questions")];
    case "questionDetail":
    case "questionDraftEditor":
      return [page(questionLabel)];
    case "blueprintCourses":
      return [page("My Blueprint Courses")];
    case "publicBlueprintSearch":
      return [page("Search Public Blueprint Courses")];
    case "blueprintCourseDetail":
      return [page(labels.blueprintCourseTitle ?? "Blueprint Course")];
    case "myChangeProposals":
      return [page("My Change Proposals")];
    case "changeProposalDetail":
      return [
        ...collectionTrail("My Change Proposals", "myChangeProposals"),
        page("Change Proposal"),
      ];
    case "assessmentsDueSoon":
      return [page("Assessments Due Soon")];
    case "assessmentTemplates":
      return [page("My Assessment Templates")];
    case "courseAssessments":
    case "studentCourseLanding":
      return [courseBreadcrumb(labels, currentHref)];
    case "studentCourseProgress":
      return [...course, page("Progress")];
    case "courseRoster":
      return [...course, page("Students")];
    case "courseAppearance":
      return [...course, page("Appearance")];
    case "gradebook":
      return [...course, page("Gradebook")];
    case "assessmentCreate":
      return [...course, page("New Assessment")];
    case "assessmentWorkspaceOverview":
      return [...course, page(assessmentLabel)];
    case "assessmentWorkspaceQuestions":
    case "assessmentWorkspacePolicies":
    case "assessmentWorkspaceStudentView": {
      const section = {
        assessmentWorkspaceQuestions: "Questions",
        assessmentWorkspacePolicies: "Properties",
        assessmentWorkspaceStudentView: "Student View",
      }[routeState.route.id];
      return [
        ...course,
        ...(assessmentHref === undefined ? [] : [breadcrumb(assessmentLabel, assessmentHref)]),
        page(section),
      ];
    }
    case "assessmentOverview":
      return [...course, page(labels.assessmentTitle ?? "Before you start")];
    case "studentDueSoon":
      return [page("Due Soon")];
    case "studentCompleted":
      return [page("Completed")];
    case "studentScores":
      return [page("Scores")];
    case "studentResponseStats":
      return [page("Response Stats")];
    case "studentAttemptHistory":
      return [page("Attempt History")];
    case "assessmentAttempt":
    case "assessmentAttemptSummary": {
      const attemptId = parseAssessmentAttemptId(routeState.params.assessmentAttemptId ?? "");
      const current = {
        ...page(routeState.route.id === "assessmentAttempt" ? "Attempt" : "Attempt history"),
        ...(attemptId === null ? {} : { state: { assessmentAttemptId: attemptId } }),
      };
      if (assessmentHref === undefined) return [current];
      return [
        ...course,
        breadcrumb(labels.assessmentAttemptTitle ?? "Coursework", assessmentHref),
        current,
      ];
    }
  }
}

/** Preserve distinct names even when they share a URL; the deeper identical name wins. */
function collapseAdjacentLabels(
  trail: ReadonlyArray<RibbonBreadcrumbModel>,
): ReadonlyArray<RibbonBreadcrumbModel> {
  const collapsed: RibbonBreadcrumbModel[] = [];
  for (const item of trail) {
    if (collapsed[collapsed.length - 1]?.label === item.label) collapsed.pop();
    collapsed.push(item);
  }
  return Object.freeze(
    collapsed.map((item, index) =>
      Object.freeze({ ...item, current: index === collapsed.length - 1 }),
    ),
  );
}

export function presentRibbonBreadcrumbs(
  routeState: RibbonRouteState,
  userRole: UserRole,
  labels: RibbonContextLabels,
  tabs: ReadonlyArray<RibbonControlModel<RibbonTabId>>,
  taskAreas: ReadonlyArray<RibbonTaskAreaModel>,
): ReadonlyArray<RibbonBreadcrumbModel> {
  if (routeState.route.id === "signIn") return [];
  const home = breadcrumb("Home", userRoleHomePath(userRole));
  const currentHref = canonicalPathForRouteState(routeState);
  // Invalid scope keeps a known way home without guessing an ancestor or exposing an ID.
  if (currentHref === undefined) return Object.freeze([Object.freeze(home)]);
  if (routeState.route.id === "courses") return collapseAdjacentLabels([home]);
  const selectedTab = tabs.find((tab) => tab.selected);
  const selectedTask = taskAreas
    .flatMap((area) => area.controls)
    .find((control) => control.selected);
  const tierOne = selectedTab === undefined ? undefined : tierBreadcrumb(selectedTab, labels);
  const tierTwo = selectedTask === undefined ? undefined : tierBreadcrumb(selectedTask, labels);
  return collapseAdjacentLabels([
    home,
    ...(tierOne === undefined ? [] : [tierOne]),
    ...(tierTwo === undefined ? [] : [tierTwo]),
    ...pageTrail(routeState, labels, currentHref, userRole),
  ]);
}
