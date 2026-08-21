/* eslint-disable @typescript-eslint/explicit-function-return-type -- ApiClient supplies the public signatures. */
// Deterministic course-grade mock state kept separate from the general client facade.

import { publishedProblemFixture } from "../../../generated/fixtures/published_problem";
import type { CourseGradeSchemeUpdateView } from "../../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseGradeSchemeView } from "../../../generated/api/CourseGradeSchemeView";
import type { CourseGradebookTotalsView } from "../../../generated/api/CourseGradebookTotalsView";
import type { ApiClient } from "../client";
import { ApiProtocolError } from "../http_client";

export function createMockCourseGradeClient(): Pick<
  ApiClient,
  | "getCourseGradeScheme"
  | "saveCourseGradeScheme"
  | "getCourseGradebookTotals"
  | "createCourseGradeExport"
> {
  let revision = 1;
  let scheme: CourseGradeSchemeView = {
    scheme: {
      mode: "totalPoints",
      rounding: "fourDecimalPlacesHalfAwayFromZero",
      categories: [],
      letterBands: [],
    },
    assignments: [
      {
        assignment: publishedProblemFixture.assignment.id,
        title: publishedProblemFixture.assignment.title,
        included: true,
        category: null,
        position: null,
      },
    ],
  };

  return {
    getCourseGradeScheme: () =>
      Promise.resolve({ ...structuredClone(scheme), revision: `"${revision}"` }),
    saveCourseGradeScheme: (_courseId, update: CourseGradeSchemeUpdateView, observedRevision) => {
      if (observedRevision !== `"${revision}"`)
        return Promise.reject(new ApiProtocolError("Mock course grade revision conflict"));
      scheme = {
        scheme: structuredClone(update.scheme),
        assignments: update.assignments.map((item) => ({ ...item, title: "Fixture assignment" })),
      };
      revision += 1;
      return Promise.resolve({ ...structuredClone(scheme), revision: `"${revision}"` });
    },
    getCourseGradebookTotals: (): Promise<CourseGradebookTotalsView> =>
      Promise.resolve({
        mode: scheme.scheme.mode,
        rounding: scheme.scheme.rounding,
        rows: [
          {
            rosterId: ".student-01",
            displayName: "Student One",
            outcome: { status: "unavailable", reason: "recalculating" },
          },
        ],
      }),
    createCourseGradeExport: () =>
      Promise.resolve({
        exportId: "0198e000-0000-7000-8000-000000000099",
        filename: "ple-course-grade-export.csv",
        csv: new Blob(
          [
            "record_type,aggregation_mode,rounding_rule,roster_id,email,display_name,course_total,letter,unavailable_status\r\nmetadata,totalPoints,fourDecimalPlacesHalfAwayFromZero,,,,,,\r\nstudent,,,student-01,,Student One,,,recalculating\r\n",
          ],
          { type: "text/csv" },
        ),
      }),
  };
}
