// Shared Blueprint discovery result content. The list renders this record shape.

import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { RecordContent } from "../components/data_sheet/record_content";
function blueprintCoursePath(blueprintCourseId: string): string {
  return `/blueprint-courses/${encodeURIComponent(blueprintCourseId)}`;
}

/** Reusable result facts and inspection action for both Blueprint collections. */
export function blueprintCourseContent(
  course: BlueprintCourseSummaryView,
  formatTimestamp: (timestamp: number | Date) => string,
): RecordContent {
  return {
    title: course.long_name,
    description: course.short_name === course.long_name ? undefined : course.short_name,
    details: [
      ...(course.availability === "archived"
        ? [{ kind: "text" as const, label: "Availability", value: "Archived" }]
        : []),
      {
        kind: "text",
        label: "Current Blueprint Revision",
        value: course.current_revision_tuple.revisionNumber,
      },
      {
        kind: "instructorProfile",
        label: "Author",
        accountId: course.owner_account_id,
        displayName: course.owner_display_name,
      },
      { kind: "text", label: "Institution", value: course.owner_affiliation },
      { kind: "text", label: "Stars", value: course.star_count.toLocaleString() },
      { kind: "text", label: "Watches", value: course.watcher_count.toLocaleString() },
      {
        kind: "text",
        label: "Last edited",
        value: formatTimestamp(course.last_edited_at_millis),
      },
      { kind: "text", label: "Adoptions", value: course.total_adoptions.toLocaleString() },
      {
        kind: "text",
        label: "Students having taken the course",
        value: course.total_students_ever_enrolled.toLocaleString(),
      },
      { kind: "courseClassification", value: course.classification },
    ],
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open Blueprint",
        href: blueprintCoursePath(course.id),
        primary: true,
      },
    ],
  };
}
