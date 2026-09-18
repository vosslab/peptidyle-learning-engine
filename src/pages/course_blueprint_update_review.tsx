import { A } from "@solidjs/router";
import { createResource, createSignal, For, Show, type JSX } from "solid-js";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type {
  CourseAssessmentBlueprintUpdateSummary,
  CourseBlueprintUpdateReview,
} from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { assessmentTypePresentation } from "../assessment_type_presentation";

function updateLabel(assessment: CourseAssessmentBlueprintUpdateSummary): string {
  if (assessment.cannotApplyReason === "retainedSourceMissing") {
    return "Source Assessment removed";
  }
  if (assessment.cannotApplyReason === "assessmentTypeMismatch") {
    return "Assessment Type differs";
  }
  return assessment.matchesSource ? "Matches source" : "Changes available";
}

/** Read-only discovery; each Assessment's editor loads its own current detailed review. */
export function CourseBlueprintUpdateReviewList(props: {
  readonly courseReference: CourseInstanceId;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [opened, setOpened] = createSignal(false);
  const [review, { refetch }] = createResource(
    () => (opened() ? props.courseReference : false),
    async (reference): Promise<CourseBlueprintUpdateReview | null> => {
      try {
        return await applicationApi.client.getCourseBlueprintUpdateReview(reference);
      } catch {
        // Access loss and unavailable parents share one non-enumerating recovery state.
        return null;
      }
    },
  );

  return (
    <>
      <button
        class="quiet-action"
        type="button"
        aria-expanded={opened()}
        aria-controls={opened() ? "course-blueprint-update-review" : undefined}
        onClick={() => setOpened((value) => !value)}
        data-review-blueprint-updates
      >
        Review Blueprint updates
      </button>
      <Show when={opened()}>
        <section
          id="course-blueprint-update-review"
          class="course-blueprint-update-review"
          aria-labelledby="course-blueprint-update-review-heading"
          aria-busy={review.loading}
          data-course-blueprint-update-review
        >
          <div class="course-blueprint-update-review__heading">
            <h2 id="course-blueprint-update-review-heading">Blueprint updates</h2>
            <button
              class="quiet-action"
              type="button"
              disabled={review.loading}
              onClick={() => void refetch()}
              data-refresh-blueprint-updates
            >
              Refresh Blueprint updates
            </button>
          </div>
          <Show when={review.loading}>
            <p class="loading-state" role="status">
              Loading Blueprint updates...
            </p>
          </Show>
          <Show when={!review.loading && review() === null}>
            <p class="route-error" role="alert">
              Blueprint updates are unavailable through your current access. Try refreshing.
            </p>
          </Show>
          <Show when={!review.loading && review()}>
            {(summary) => (
              <>
                <p class="page-lede" data-blueprint-source-revision>
                  Source Blueprint {summary().blueprintReference}, Revision{" "}
                  {summary().sourceRevision} under review.
                </p>
                <p class="instructor-list__metadata">
                  Review and approve changes separately for each Assessment. The original Course
                  adoption remains Revision {summary().adoptedRevision}.
                </p>
                <Show
                  when={summary().assessments.length > 0}
                  fallback={
                    <p class="empty-state">
                      No Assessments copied from this Blueprint are available to review.
                    </p>
                  }
                >
                  <div class="instructor-list" aria-label="Blueprint Assessment updates">
                    <For each={summary().assessments}>
                      {(assessment) => (
                        <article
                          class="instructor-list__row course-blueprint-update-review__row"
                          data-blueprint-update-assessment={assessment.assessmentReference}
                        >
                          <div class="instructor-list__identity">
                            <p class="instructor-list__kind">{updateLabel(assessment)}</p>
                            {/* ASVS 1.2.1: source-controlled titles remain escaped text. */}
                            <h3>{assessment.title}</h3>
                            <p class="instructor-list__metadata">
                              {assessmentTypePresentation(assessment.assessmentType).label}
                              {"; "}Assessment {assessment.assessmentReference}
                            </p>
                          </div>
                          <div class="instructor-list__actions">
                            <A
                              class="quiet-link"
                              aria-label={`Review this Assessment: ${assessment.title}`}
                              href={`/instructor/courses/${props.courseReference}/assessments/${assessment.assessmentReference}/questions`}
                            >
                              Review this Assessment
                            </A>
                          </div>
                        </article>
                      )}
                    </For>
                  </div>
                </Show>
              </>
            )}
          </Show>
        </section>
      </Show>
    </>
  );
}
