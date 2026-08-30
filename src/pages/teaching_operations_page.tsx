// teaching_operations_page.tsx - authorized course shell for groups, team, and retention.

import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { CourseGroupMemberView } from "../../generated/api/CourseGroupMemberView";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import {
  CourseGroupsPanel,
  type CourseMemberOption,
} from "./teaching_operations/course_groups_panel";
import { RetentionPanel } from "./teaching_operations/retention_panel";
import { SysadminInstructorApprovalPanel } from "./teaching_operations/sysadmin_instructor_approval_panel";
import { TeachingTeamPanel } from "./teaching_team_panel";

type PageState = "loading" | "ready" | "denied" | "unavailable";
type MembersState = "loading" | "ready" | "error";

async function loadAllStudentTargets(
  loadPage: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<{
    readonly students: ReadonlyArray<CourseGroupMemberView>;
    readonly nextCursor: string | null;
  }>,
): Promise<ReadonlyArray<CourseMemberOption>> {
  const members: Array<CourseMemberOption> = [];
  const seen = new Set<string>();
  let cursor: string | undefined;
  do {
    const page = await loadPage(cursor, 100);
    for (const student of page.students) {
      if (seen.has(student.reference)) continue;
      seen.add(student.reference);
      members.push({ reference: student.reference, display: student.display });
    }
    cursor = page.nextCursor ?? undefined;
  } while (cursor !== undefined);
  return members;
}

/** Course-local shell; child panels receive only safe membership labels and opaque request values. */
export function TeachingOperationsPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const scopedRoute = useCourseThemeRouteData();
  const course = scopedRoute?.kind === "course" ? courseRouteData(scopedRoute).summary : undefined;
  const [state, setState] = createSignal<PageState>("loading");
  const [membersState, setMembersState] = createSignal<MembersState>("loading");
  const [members, setMembers] = createSignal<ReadonlyArray<CourseMemberOption>>([]);

  async function loadMembers(): Promise<void> {
    if (course === undefined) return;
    setMembersState("loading");
    try {
      setMembers(
        await loadAllStudentTargets((cursor, pageSize) =>
          runtime.client.listCourseStudentTargets(course.id, cursor, pageSize),
        ),
      );
      setMembersState("ready");
    } catch {
      setMembersState("error");
    }
  }

  function load(): void {
    if (course === undefined) {
      setState("unavailable");
      return;
    }
    if (course.role !== "instructor") {
      setState("denied");
      return;
    }
    setState("loading");
    setState("ready");
    void loadMembers();
  }

  const mayExtendRetention = (): boolean => {
    const current = session.state();
    return current.kind === "authenticated" && current.session.account.role === "sysadmin";
  };

  onMount(() => void load());

  return (
    <section class="page teaching-operations-page" data-route-surface="teachingOperations">
      <p class="eyebrow">Instructor course settings</p>
      <h1>Teaching operations</h1>
      <p class="page-lede">Manage groups, the teaching team, and student-record retention.</p>
      <Show when={state() === "loading"}>
        <p role="status">Loading authorized course members...</p>
      </Show>
      <Show when={state() === "denied"}>
        <p role="alert">You do not manage this course.</p>
      </Show>
      <Show when={state() === "unavailable"}>
        <section class="route-error" role="alert">
          <p>Teaching operations are unavailable for this course.</p>
          <button type="button" onClick={() => void load()}>
            Retry
          </button>
        </section>
      </Show>
      <Show when={course !== undefined && state() === "ready"}>
        <div class="teaching-operations-hub">
          <Show when={membersState() === "error"}>
            <section class="inline-error" role="alert">
              <p>
                Student choices could not load. Teaching-team and retention controls remain
                available.
              </p>
              <button type="button" onClick={() => void loadMembers()}>
                Retry student choices
              </button>
            </section>
          </Show>
          <CourseGroupsPanel
            courseId={course!.id}
            courseReference={course!.reference}
            runtime={runtime}
            memberOptions={members()}
          />
          <TeachingTeamPanel courseId={course!.id} />
          <Show when={mayExtendRetention()}>
            <SysadminInstructorApprovalPanel runtime={runtime} />
          </Show>
          <RetentionPanel
            courseId={course!.id}
            courseReference={course!.reference}
            runtime={runtime}
            mayExtendRetention={mayExtendRetention()}
          />
        </div>
      </Show>
    </section>
  );
}

export { loadAllStudentTargets };
