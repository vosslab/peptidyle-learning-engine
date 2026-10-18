import { createResource, createSignal, Show, type JSX } from "solid-js";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type {
  CourseAssessmentBlueprintUpdateSummary,
  CourseBlueprintUpdateReview,
} from "../api/assessment_release";
import { useApplicationApi } from "../api/application_api";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import type { RecordField } from "../components/data_sheet/data_sheet_fields";
import type { RecordContent } from "../components/data_sheet/record_content";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import { assessmentRouteId } from "../navigation/public_route";

function updateLabel(assessment: CourseAssessmentBlueprintUpdateSummary): string {
  if (assessment.cannotApplyReason === "retainedSourceMissing") {
    return "Source Assessment removed";
  }
  if (assessment.cannotApplyReason === "assessmentTypeMismatch") {
    return "Assessment Type differs";
  }
  return assessment.matchesSource ? "Matches source" : "Changes available";
}

type UpdateSortKey = "title" | "type" | "update";
type UpdateFilterKey = "type" | "update";
const updateFields: ReadonlyArray<
  RecordField<CourseAssessmentBlueprintUpdateSummary, UpdateSortKey, UpdateFilterKey>
> = [
  {
    id: "title",
    label: "Assessment",
    role: "identity",
    sortKey: "title",
    value: (assessment) => assessment.title,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "type",
    label: "Type",
    sortKey: "type",
    filterKey: "type",
    value: (assessment) => assessmentTypePresentation(assessment.assessmentType).label,
    defaultColumn: { priority: 1, width: "11rem" },
  },
  {
    id: "update",
    label: "Update",
    sortKey: "update",
    filterKey: "update",
    value: updateLabel,
    defaultColumn: { priority: 2, width: "14rem" },
  },
];

function assessmentUpdateContent(
  courseInstanceId: CourseInstanceId,
): (assessment: CourseAssessmentBlueprintUpdateSummary) => RecordContent {
  return (assessment): RecordContent => {
    const assessmentId = assessmentRouteId(assessment.assessmentId);
    return {
      // ASVS 1.2.1: source-controlled titles remain escaped text in RecordList.
      title: assessment.title,
      details: [
        { kind: "text", label: "Update:", value: updateLabel(assessment) },
        { kind: "assessmentType", value: assessment.assessmentType },
        { kind: "text", label: "Assessment ID:", value: assessment.assessmentId },
      ],
      actions: [
        {
          id: "review-assessment",
          kind: "link",
          label: "Review Assessment",
          title: `Review this Assessment: ${assessment.title}`,
          href: `/instructor/courses/${courseInstanceId}/assessments/${assessmentId}/questions`,
          primary: true,
        },
      ],
    };
  };
}

/** Read-only discovery; each Assessment's editor loads its own current detailed review. */
export function CourseBlueprintUpdateReviewList(props: {
  readonly courseInstanceId: CourseInstanceId;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [opened, setOpened] = createSignal(false);
  const [review, { refetch }] = createResource(
    () => (opened() ? props.courseInstanceId : false),
    async (courseInstanceId): Promise<CourseBlueprintUpdateReview> => {
      try {
        return await applicationApi.client.getCourseBlueprintUpdateReview(courseInstanceId);
      } catch {
        // Access loss and unavailable parents share one non-enumerating recovery state.
        throw new Error("Blueprint updates are unavailable through your current access.");
      }
    },
  );
  const updateSearch = createLocalCompleteSearch(
    {
      rows: () => {
        const summary = review();
        return summary === undefined ? undefined : summary.assessments;
      },
      loading: () => review.loading,
      error: (): unknown => review.error,
      refetchSource: refetch,
    },
    {
      fields: updateFields,
      rowId: (assessment) => assessment.assessmentId,
      keyword: (assessment) =>
        `${assessment.title} ${assessment.assessmentId} ${assessmentTypePresentation(assessment.assessmentType).label} ${updateLabel(assessment)}`,
      content: assessmentUpdateContent(props.courseInstanceId),
    },
  );
  const sourceRevisionContext = (): CourseBlueprintUpdateReview | undefined =>
    review.state === "ready" || review.state === "refreshing" ? review.latest : undefined;

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
          <Show when={sourceRevisionContext()}>
            {(summary) => (
              <p class="page-lede" data-blueprint-source-revision>
                Source Blueprint {summary().currentBlueprintCourseRevisionTuple.blueprintCourseId}{" "}
                at current Revision {summary().currentBlueprintCourseRevisionTuple.revisionNumber};
                adopted Revision {summary().adoptedBlueprintCourseRevisionTuple.revisionNumber}.
              </p>
            )}
          </Show>
          <SearchBlock
            layout="above"
            controls={
              <SearchControls
                state={updateSearch.state}
                textLabel="Search Blueprint Assessment updates"
              />
            }
            state={updateSearch.state}
            collection={{
              ariaLabel: "Blueprint Assessment updates",
              ...updateSearch.collection,
              get emptyState() {
                return {
                  title:
                    updateSearch.state.query().text || updateSearch.state.query().filters.size
                      ? "No Blueprint Assessment updates match your search."
                      : "No Assessments copied from this Blueprint are available to review.",
                };
              },
            }}
          />
        </section>
      </Show>
    </>
  );
}
