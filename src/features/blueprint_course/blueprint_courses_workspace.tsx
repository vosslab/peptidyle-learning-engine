// My Blueprint Courses uses the same complete discovery query as Public Search.
import { A } from "@solidjs/router";
import { Show, createSignal, type JSX } from "solid-js";

import { PageFrame } from "../../components/page_frame";
import { BlueprintCourseCollection } from "../../pages/blueprint_course_search_page";
import { BlueprintCourseCreateDialog } from "./blueprint_course_create_dialog";
import { BlueprintCourseImport } from "./blueprint_exchange";
import type { BlueprintCoursesWorkspaceProps } from "./blueprint_course_workspace_types";
import "./blueprint_course.css";

export function BlueprintCoursesWorkspace(props: BlueprintCoursesWorkspaceProps): JSX.Element {
  const [creating, setCreating] = createSignal(false);
  const [failure, setFailure] = createSignal("");
  let createTrigger: HTMLButtonElement | undefined;

  const actions = (
    <div class="blueprint-course-inline-actions">
      <Show when={props.proposalClient}>
        <A href="/blueprint-change-proposals">My Change Proposals</A>
      </Show>
      <button
        type="button"
        onClick={(event) => {
          createTrigger = event.currentTarget;
          setCreating(true);
        }}
      >
        Create Blueprint Course
      </button>
      <BlueprintCourseImport client={props.client} />
    </div>
  );

  return (
    <PageFrame routeSurface="blueprintCourses">
      <Show when={failure() !== ""}>
        <p class="blueprint-course-notice" role="alert">
          {failure()}
        </p>
      </Show>
      <BlueprintCourseCollection client={props.client} scope="all" actions={actions} />
      <Show when={creating()}>
        <BlueprintCourseCreateDialog
          client={props.client}
          pickerRepository={props.pickerRepository}
          pickerSources={props.pickerSources}
          loadQuestionInspection={props.loadQuestionInspection}
          questionRevisionPreviewDocumentUrl={props.questionRevisionPreviewDocumentUrl}
          questionImageUrl={props.questionImageUrl}
          onClose={() => {
            setCreating(false);
            queueMicrotask(() => createTrigger?.focus());
          }}
          onFailure={setFailure}
        />
      </Show>
    </PageFrame>
  );
}
