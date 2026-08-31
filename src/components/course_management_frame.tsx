// course_management_frame.tsx - stable course identity and navigation for Instructor routes.

import { Show, type JSX } from "solid-js";

import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { RouteId } from "../route_contract";
import { CourseManagementNav, type CourseManagementSection } from "./course_management_nav";
import "./course_management_frame.css";

export interface CourseManagementFrameProps {
  readonly course: CourseSummary;
  readonly routeId: RouteId;
  readonly children: JSX.Element;
}

/** Maps each course-owned Instructor task to its persistent ribbon selection. */
export function courseManagementSectionForRoute(
  routeId: RouteId,
): CourseManagementSection | undefined {
  switch (routeId) {
    case "courseAssignments":
    case "assignmentWorkspaceOverview":
    case "assignmentWorkspaceQuestions":
    case "assignmentWorkspacePolicies":
    case "assignmentWorkspaceStudentView":
    case "assignmentWorkspaceGradingOperations":
    case "assignmentAccess":
    case "assignmentPreview":
      return "assignments";
    case "assignmentCreate":
      return "newAssignment";
    case "courseRoster":
      return "students";
    case "teachingOperations":
      return "teachingOperations";
    case "gradebook":
    case "studentWorkInspection":
      return "gradebook";
    case "courseGradeSettings":
      return "gradeSettings";
    case "courseAppearance":
      return "appearance";
    default:
      return undefined;
  }
}

/** Keeps course context and the Instructor ribbon stationary while route content changes below it. */
export function CourseManagementFrame(props: CourseManagementFrameProps): JSX.Element {
  const section = (): CourseManagementSection =>
    courseManagementSectionForRoute(props.routeId) ?? "assignments";
  const courseHome = (): boolean => props.routeId === "courseAssignments";

  return (
    <div class="page course-management-frame" data-course-management-frame>
      <header class="course-management-header">
        <p class="eyebrow">Instructor course</p>
        <Show
          when={courseHome()}
          fallback={
            <p class="course-management-title" data-course-title>
              {props.course.title}
            </p>
          }
        >
          <h1 class="course-management-title" data-course-title>
            {props.course.title}
          </h1>
        </Show>
        <CourseManagementNav courseReference={props.course.reference} active={section()} />
      </header>
      <div class="course-management-content">{props.children}</div>
    </div>
  );
}
