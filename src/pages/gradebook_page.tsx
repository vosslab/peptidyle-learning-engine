// gradebook_page.tsx - focused answer-free Instructor Gradebook.

import { useParams } from "@solidjs/router";
import { Show, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseGradebook, GradebookExportFormat } from "../api/live_gradebook";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
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

const gradebookRegions: ReadonlyArray<RecordRegion<GradebookStudentWork>> = [
  {
    id: "student",
    role: "identity",
    priority: "required",
    width: "var(--gradebook-student-column)",
    align: "start",
    header: "Student",
    content: (work) => (
      <>
        <span class="visually-hidden">Student: </span>
        <span>{work.rosterName}</span>
        <small>{work.rosterId}</small>
      </>
    ),
  },
  {
    id: "coursework",
    role: "metadata",
    priority: "high",
    width: "var(--gradebook-coursework-column)",
    align: "start",
    header: "Coursework",
    content: (work) => (
      <>
        <span class="visually-hidden">Coursework: </span>
        <span>{work.assessmentTitle}</span>
      </>
    ),
  },
  {
    id: "progress",
    role: "status",
    priority: "medium",
    width: "var(--gradebook-progress-column)",
    align: "start",
    header: "Progress status",
    content: (work) => (
      <>
        <span class="visually-hidden">Progress status: </span>
        {progressLabel(work.assessmentAttemptCompletion)}
      </>
    ),
  },
  {
    id: "score",
    role: "status",
    priority: "required",
    width: "var(--gradebook-score-column)",
    align: "end",
    header: "Current score",
    content: (work) => (
      <>
        <span class="visually-hidden">Current score: </span>
        {scoreLabel(work)}
      </>
    ),
  },
];

function GradebookEvidence(props: {
  readonly rows: ReadonlyArray<GradebookStudentWork>;
  readonly state: RecordListState;
}): JSX.Element {
  return (
    <section class="gradebook-record-list" aria-label="Student progress and scores">
      <RecordList
        rows={props.rows}
        regions={gradebookRegions}
        recordId={(work) => `${work.rosterId}-${work.assessmentId}`}
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

  const gradebookState = (): RecordListState => {
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
      contentClass="gradebook-page"
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
        <PageFrame
          contentClass="gradebook-page"
          routeSurface="gradebook"
          title="Gradebook unavailable"
        >
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
