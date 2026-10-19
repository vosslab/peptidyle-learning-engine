// gradebook_page.tsx - focused answer-free Instructor Gradebook.

import { useParams } from "@solidjs/router";
import { Show, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { CourseGradebookStudentWork, GradebookExportFormat } from "../../api/live_gradebook";
import { useApplicationApi } from "../../api/application_api";
import { PageFrame } from "../../components/page_frame/page_frame";
import type { RecordContent } from "../../components/data_sheet/record_content";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { parseCourseInstanceId } from "../../navigation/public_route";
import {
  compareGradebookWork,
  gradebookFields,
  gradebookProgressLabel,
  gradebookScoreLabel,
  gradebookWorkId,
} from "./course_roster_gradebook_fields";
import "./gradebook_record_list.css";

function gradebookContent(work: CourseGradebookStudentWork): RecordContent {
  return {
    title: work.rosterName,
    description: `${work.rosterId} · ${work.assessmentTitle}`,
    details: [
      { kind: "text", label: "Coursework", value: work.assessmentTitle },
      { kind: "text", label: "Progress status", value: gradebookProgressLabel(work) },
      { kind: "text", label: "Current score", value: gradebookScoreLabel(work) },
    ],
    actions: [],
  };
}

function GradebookCoursePage(props: { readonly courseInstanceId: CourseInstanceId }): JSX.Element {
  const runtime = useApplicationApi();
  const [studentWork, { refetch }] = createResource(
    () => props.courseInstanceId,
    async (course) => (await runtime.client.getCourseGradebook(course)).studentWork,
  );
  const gradebookSearch = createLocalCompleteSearch(
    {
      rows: studentWork,
      loading: () => studentWork.loading,
      error: (): unknown => studentWork.error,
      refetchSource: refetch,
    },
    {
      fields: gradebookFields,
      rowId: gradebookWorkId,
      compare: compareGradebookWork,
      keyword: (work) =>
        `${work.rosterName} ${work.rosterId} ${work.assessmentTitle} ${work.assessmentId} ${gradebookProgressLabel(work)} ${gradebookScoreLabel(work)}`,
      content: gradebookContent,
    },
  );
  const [downloading, setDownloading] = createSignal(false);
  const [downloadMessage, setDownloadMessage] = createSignal("");
  const [downloadError, setDownloadError] = createSignal("");
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  async function download(format: GradebookExportFormat): Promise<void> {
    if (downloading()) return;
    setDownloading(true);
    setDownloadMessage(`Preparing ${format.toUpperCase()} download...`);
    setDownloadError("");
    try {
      const exportBlob = await runtime.client.downloadCourseGradebook(
        props.courseInstanceId,
        format,
      );
      if (disposed) return;
      // ASVS 14.3.3: keep sensitive bytes only in this short-lived download URL.
      const downloadUrl = URL.createObjectURL(exportBlob);
      const link = document.createElement("a");
      try {
        link.href = downloadUrl;
        link.download = `ple_${props.courseInstanceId}_grades.${format}`;
        document.body.append(link);
        link.click();
      } finally {
        link.remove();
        setTimeout(() => URL.revokeObjectURL(downloadUrl), 0);
      }
      setDownloadMessage(`${format.toUpperCase()} download started.`);
    } catch {
      if (disposed) return;
      setDownloadMessage("");
      setDownloadError(
        "Gradebook could not be downloaded. Try again or reload to check your access.",
      );
    } finally {
      if (!disposed) setDownloading(false);
    }
  }

  return (
    <PageFrame routeSurface="gradebook">
      <Show when={studentWork.state === "ready"}>
        <section class="gradebook-export" aria-label="Download point grades">
          <p>
            Export all Assessment points for this Course, including rows outside the current search
            or page. Handle Course weighting and percentages in your home LMS.
          </p>
          <div class="gradebook-export-actions" aria-busy={downloading()}>
            <button type="button" disabled={downloading()} onClick={() => void download("csv")}>
              Download CSV
            </button>
            <button type="button" disabled={downloading()} onClick={() => void download("tsv")}>
              Download TSV
            </button>
          </div>
          <p role="status">{downloadMessage()}</p>
          <Show when={downloadError()}>
            <p role="alert">{downloadError()}</p>
          </Show>
        </section>
      </Show>
      <section class="gradebook-record-table" aria-label="Student progress and scores">
        <SearchBlock
          layout="above"
          controls={
            <SearchControls state={gradebookSearch.state} textLabel="Search student progress" />
          }
          state={gradebookSearch.state}
          collection={{
            ariaLabel: "Student progress and scores",
            ...gradebookSearch.collection,
            get emptyState() {
              return gradebookSearch.state.query().text ||
                gradebookSearch.state.query().filters.size
                ? { title: "No student progress matches your search." }
                : {
                    title: "No student progress to review yet",
                    message:
                      "Progress and scores appear here when this Course has active Students and released Coursework.",
                  };
            },
          }}
        />
      </section>
    </PageFrame>
  );
}

/** Loads the server-authorized Gradebook for the exact Course Instance route ID. */
export function GradebookPage(): JSX.Element {
  const params = useParams();
  // ASVS 2.2.1/8.3.1: validate the locator here; the server retains authorization.
  const course = (): ReturnType<typeof parseCourseInstanceId> =>
    parseCourseInstanceId(params["courseInstanceId"] ?? "");
  return (
    <Show
      when={course()}
      keyed
      fallback={
        <PageFrame routeSurface="gradebook">
          <section class="route-error" role="alert">
            <p>Return to your course list, then open the Gradebook again.</p>
          </section>
        </PageFrame>
      }
    >
      {(loadedCourse) => <GradebookCoursePage courseInstanceId={loadedCourse} />}
    </Show>
  );
}
