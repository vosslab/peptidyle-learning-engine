// assignment_access_live_page.tsx - course and assignment ownership gate for access modifiers.

import { useParams } from "@solidjs/router";
import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { StudentMembershipView } from "../../generated/api/StudentMembershipView";
import type { CourseId } from "../../generated/api/CourseId";
import type { TeachingOperationRevision } from "../../generated/api/TeachingOperationRevision";
import type { SelectedStudent } from "./assignment_access/model";
import { useApplicationApi } from "../api/application_api";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import { resolveAssignmentRoute } from "../navigation/resolved_route";
import {
  courseRouteReference,
  parseAssignmentReference,
  type AssignmentRouteReference,
  type CourseRouteReference,
} from "../navigation/public_route";
import { AssignmentAccessPage } from "./assignment_access_page";

type Gate =
  | { readonly kind: "loading" }
  | {
      readonly kind: "allowed";
      readonly courseId: CourseId;
      readonly assignmentId: AssignmentId;
      readonly revision: TeachingOperationRevision;
      readonly courseReference: CourseRouteReference;
      readonly assignmentReference: AssignmentRouteReference;
    }
  | { readonly kind: "denied" }
  | { readonly kind: "unavailable" };

async function loadAllPreviewSubjects(
  loadPage: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<{
    readonly students: ReadonlyArray<StudentMembershipView>;
    readonly nextCursor: string | null;
  }>,
): Promise<ReadonlyArray<SelectedStudent>> {
  const subjects: Array<SelectedStudent> = [];
  const seen = new Set<string>();
  let cursor: string | undefined;
  do {
    const page = await loadPage(cursor, 100);
    for (const student of page.students) {
      if (seen.has(student.reference)) continue;
      seen.add(student.reference);
      subjects.push({ reference: student.reference, display: student.display });
    }
    cursor = page.nextCursor ?? undefined;
  } while (cursor !== undefined);
  return subjects;
}

/** The route resolves public references once, proves course ownership, then passes typed IDs inward. */
export function AssignmentAccessLivePage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  const scopedRoute = useCourseThemeRouteData();
  const [gate, setGate] = createSignal<Gate>({ kind: "loading" });
  const allowedGate = (): Extract<Gate, { readonly kind: "allowed" }> | undefined => {
    const current = gate();
    return current.kind === "allowed" ? current : undefined;
  };

  async function checkAccess(): Promise<void> {
    const course =
      scopedRoute?.kind === "course" ? courseRouteData(scopedRoute).summary : undefined;
    if (course === undefined) {
      setGate({ kind: "unavailable" });
      return;
    }
    if (course.role !== "instructor") {
      setGate({ kind: "denied" });
      return;
    }
    try {
      const assignmentReference = parseAssignmentReference(params["assignmentRef"] ?? "");
      if (assignmentReference === null) {
        setGate({ kind: "unavailable" });
        return;
      }
      const assignment = await resolveAssignmentRoute(runtime.client, params["assignmentRef"]);
      if (assignment.courseId !== course.id) {
        setGate({ kind: "unavailable" });
        return;
      }
      const editor = await runtime.client.getAssignmentWorkspace(
        course.id,
        assignment.assignmentId,
      );
      if (editor.id !== assignment.assignmentId || editor.courseId !== course.id) {
        setGate({ kind: "unavailable" });
        return;
      }
      setGate({
        kind: "allowed",
        courseId: course.id,
        assignmentId: assignment.assignmentId,
        revision: editor.revision,
        courseReference: courseRouteReference(course.reference),
        assignmentReference,
      });
    } catch {
      setGate({ kind: "unavailable" });
    }
  }

  onMount(() => void checkAccess());

  return (
    <Show
      when={allowedGate()}
      keyed
      fallback={
        <section class="page" data-route-surface="assignmentAccessGate">
          <Show
            when={gate().kind === "loading"}
            fallback={<p role="alert">This assignment access page is unavailable.</p>}
          >
            <p role="status">Checking assignment access...</p>
          </Show>
        </section>
      }
    >
      {(allowed) => (
        <AssignmentAccessPage
          client={runtime.client}
          courseId={allowed.courseId}
          assignmentId={allowed.assignmentId}
          initialRevision={allowed.revision}
          courseReference={allowed.courseReference}
          assignmentReference={allowed.assignmentReference}
          reloadAssignmentRevision={async () => {
            const editor = await runtime.client.getAssignmentWorkspace(
              allowed.courseId,
              allowed.assignmentId,
            );
            if (editor.id !== allowed.assignmentId || editor.courseId !== allowed.courseId) {
              throw new Error("Assignment revision is no longer available");
            }
            return editor.revision;
          }}
          loadPreviewSubjects={() =>
            loadAllPreviewSubjects((cursor, pageSize) =>
              runtime.client.listCourseStudentTargets(allowed.courseId, cursor, pageSize),
            )
          }
        />
      )}
    </Show>
  );
}

export { loadAllPreviewSubjects };
