// gradebook_page.tsx - focused answer-free Instructor Gradebook.

import { useParams } from "@solidjs/router";
import { Show, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseGradebook, GradebookExportFormat } from "../api/live_gradebook";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import {
  RecordTable,
  type RecordTableColumn,
  type RecordTableRowHeader,
} from "../components/record_list/record_table";
import type { RecordCollectionState } from "../components/record_list/record_collection_state";
import { parseCourseInstanceId } from "../navigation/public_route";
import { formatPointScore } from "../score_format";
import "./gradebook_record_list.css";

function progressLabel(completion: "inProgress" | "completed" | null): string {
  if (completion === "completed") return "Completed and scored";
  if (completion === "inProgress") return "In progress";
  return "Not started";
}

function scoreLabel(work: CourseGradebook["studentWork"][number]): string {
  if (work.score !== null)
    return formatPointScore(work.score.pointsEarned, work.score.pointsPossible);
  return work.expiredSubmitting ? "Expired, submitting" : "-";
}

type GradebookStudentWork = CourseGradebook["studentWork"][number];

const gradebookRowHeader: RecordTableRowHeader<GradebookStudentWork> = {
  id: "student",
  header: "Student",
  width: "var(--gradebook-student-column)",
  content: (work) => (
    <>
      <span>{work.rosterName}</span>
      <small>{work.rosterId}</small>
    </>
  ),
};

const gradebookColumns: ReadonlyArray<RecordTableColumn<GradebookStudentWork>> = [
  {
    id: "coursework",
    header: "Coursework",
    width: "var(--gradebook-coursework-column)",
    cell: (work) => <span>{work.assessmentTitle}</span>,
  },
  {
    id: "progress",
    header: "Progress status",
    width: "var(--gradebook-progress-column)",
    cell: (work) => <span>{progressLabel(work.assessmentAttemptCompletion)}</span>,
  },
  {
    id: "score",
    header: "Current score",
    width: "var(--gradebook-score-column)",
    align: "end",
    cell: (work) => <span>{scoreLabel(work)}</span>,
  },
];

function GradebookEvidence(props: {
  readonly rows: ReadonlyArray<GradebookStudentWork>;
  readonly state: RecordCollectionState;
}): JSX.Element {
  return (
    <section class="gradebook-record-table" aria-label="Student progress and scores">
      <RecordTable
        rows={props.rows}
        rowId={(work) => `${work.rosterId}-${work.assessmentId}`}
        rowHeader={gradebookRowHeader}
        columns={gradebookColumns}
        state={props.state}
        ariaLabel="Student progress and scores"
        emptyState={{
          title: "No student progress to review yet",
          message:
            "Progress and scores appear here when this Course has active Students and released Coursework.",
        }}
      />
    </section>
  );
}

function GradebookCoursePage(props: { readonly courseInstanceId: CourseInstanceId }): JSX.Element {
  const runtime = useApplicationApi();
  const [gradebook] = createResource(
    () => props.courseInstanceId,
    runtime.client.getCourseGradebook,
  );
  const [downloading, setDownloading] = createSignal(false);
  const [downloadMessage, setDownloadMessage] = createSignal("");
  const [downloadError, setDownloadError] = createSignal("");
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });

  const gradebookState = (): RecordCollectionState => {
    if (gradebook.loading) {
      return { kind: "loading", label: "Loading student progress and scores..." };
    }
    if (gradebook.error !== undefined) {
      return {
        kind: "error",
        title: "Gradebook unavailable",
        message: "This Course Instance is not available through your current Instructor access.",
      };
    }
    return { kind: "ready" };
  };

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
    <PageFrame
      routeSurface="gradebook"
      eyebrow="Course progress"
      title="Gradebook"
      lede="Review student progress and scores for released Coursework."
    >
      <Show when={gradebook()}>
        <section class="gradebook-export" aria-label="Download point grades">
          <p>Export Assessment points. Handle Course weighting and percentages in your home LMS.</p>
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
      <GradebookEvidence rows={gradebook()?.studentWork ?? []} state={gradebookState()} />
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
        <PageFrame routeSurface="gradebook" title="Gradebook unavailable">
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
