import { MemoryRouter, Route } from "@solidjs/router";
import { render } from "solid-js/web";
import type { JSX } from "solid-js";

import { LiveAssessmentWorkspaceConflictError } from "../../src/api/http_client/assessment_release";
import type { CourseAssessmentSummary } from "../../src/api/assessment_release";
import { createAssessmentRowModel } from "../../src/pages/course_instance_page";

type AssessmentRowClient = Parameters<typeof createAssessmentRowModel>[0]["client"];

export function mountAssessmentRow(
  assessment: CourseAssessmentSummary,
  refreshedAssessment: CourseAssessmentSummary,
): {
  readonly refresh: () => void;
  readonly saveBaselines: ReadonlyArray<string>;
  readonly dispose: () => void;
} {
  const saveBaselines: Array<string> = [];
  const client: AssessmentRowClient = {
    saveLiveAssessmentInline: (
      _courseInstanceId,
      _assessmentId,
      _input,
      expectedAssessmentEditNumber,
    ) => {
      saveBaselines.push(expectedAssessmentEditNumber);
      if (saveBaselines.length === 1) {
        return Promise.reject(new LiveAssessmentWorkspaceConflictError("/assessments/A1"));
      }
      return Promise.resolve({ ...refreshedAssessment, assessmentEditNumber: "6" });
    },
    listCourseAssessments: () => Promise.resolve([refreshedAssessment]),
  };
  const model = createAssessmentRowModel({
    courseInstanceId: "C1",
    assessment,
    client,
  });
  const dispose = render(
    () => (
      <MemoryRouter>
        <Route
          path="/*"
          component={(): JSX.Element => (
            <div>
              {model.identity(1)}
              {model.status()}
              {model.metadata()}
              {model.actions()}
              {model.details()}
            </div>
          )}
        />
      </MemoryRouter>
    ),
    document.body,
  );
  (
    window as Window & { courseInstanceAssessmentFixture?: unknown }
  ).courseInstanceAssessmentFixture = {
    refresh: (): void => model.refresh(refreshedAssessment),
    saveBaselines,
    dispose,
  };
  return { refresh: (): void => model.refresh(refreshedAssessment), saveBaselines, dispose };
}
