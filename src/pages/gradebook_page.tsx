// gradebook_page.tsx - focused answer-free Instructor Gradebook.

import { useParams } from "@solidjs/router";
import { For, Show, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseGradebook, GradebookExportFormat } from "../api/live_gradebook";
import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceId } from "../navigation/public_route";
import { formatPointScore } from "../score_format";
import "./instructor_data_tables.css";

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

function GradebookEvidence(props: { readonly gradebook: CourseGradebook }): JSX.Element {
  return (
    <Show
      when={props.gradebook.studentWork.length > 0}
      fallback={
        <section class="gradebook-empty" aria-label="No student progress to review">
          <h2>No student progress to review yet</h2>
          <p>
            Progress and scores appear here when this Course has active Students and released
            Coursework.
          </p>
        </section>
      }
    >
      <div class="gradebook-table-wrap" role="region" aria-label="Student progress and scores">
        <table class="gradebook-table">
          <thead>
            <tr>
              <th scope="col">Student</th>
              <th scope="col">Coursework</th>
              <th scope="col">Progress status</th>
              <th scope="col">Current score</th>
            </tr>
          </thead>
          <tbody>
            <For each={props.gradebook.studentWork}>
              {(work) => (
                <tr>
                  <td>
                    {/* ASVS 1.2.1: roster names remain escaped text, never markup. */}
                    <div>{work.rosterName}</div>
                    <small>{work.rosterId}</small>
                  </td>
                  <td>
                    {/* ASVS 1.2.1: JSX renders the Course title as text, never markup. */}
                    <div>{work.assessmentTitle}</div>
                    <small>{work.assessmentId}</small>
                  </td>
                  <td>{progressLabel(work.assessmentAttemptCompletion)}</td>
                  <td>{scoreLabel(work)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </Show>
  );
}

function GradebookCoursePage(props: { readonly course: CourseInstanceId }): JSX.Element {
  const runtime = useApplicationApi();
  const [gradebook] = createResource(() => props.course, runtime.client.getCourseGradebook);
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
      const exportBlob = await runtime.client.downloadCourseGradebook(props.course, format);
      if (disposed) return;
      // ASVS 14.3.3: keep sensitive bytes only in this short-lived download URL.
      const downloadUrl = URL.createObjectURL(exportBlob);
      const link = document.createElement("a");
      try {
        link.href = downloadUrl;
        link.download = `ple_${props.course}_grades.${format}`;
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
    <section class="page gradebook-page" data-route-surface="gradebook">
      <p class="eyebrow">Course progress</p>
      <h1>Gradebook</h1>
      <p class="page-lede">Review student progress and scores for released Coursework.</p>
      <Show when={gradebook.loading}>
        <p class="loading-state" role="status">
          Loading student progress and scores...
        </p>
      </Show>
      <Show when={gradebook.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Gradebook unavailable</h2>
          <p>This Course Instance is not available through your current Instructor access.</p>
        </section>
      </Show>
      <Show when={gradebook()}>
        {(loaded) => (
          <>
            <section class="gradebook-export" aria-label="Download point grades">
              <p>
                Export Assessment points. Handle Course weighting and percentages in your home LMS.
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
            <GradebookEvidence gradebook={loaded()} />
          </>
        )}
      </Show>
    </section>
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
        <section class="page gradebook-page" data-route-surface="gradebook">
          <section class="route-error" role="alert">
            <h1>Gradebook unavailable</h1>
            <p>Return to your course list, then open the Gradebook again.</p>
          </section>
        </section>
      }
    >
      {(loadedCourse) => <GradebookCoursePage course={loadedCourse} />}
    </Show>
  );
}
