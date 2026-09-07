// student_work_inspection_page.tsx - retained route with no M15 response-inspection capability.

import { A, useParams } from "@solidjs/router";
import type { JSX } from "solid-js";

/**
 * M15 publishes aggregate Gradebook evidence only. A future Student Work inspection
 * capability must arrive with its own server projection; this route never falls back
 * to the obsolete client-side response inspection surface.
 */
export function StudentWorkInspectionPage(): JSX.Element {
  const params = useParams<{ courseRef: string }>();
  const course = params.courseRef;
  return (
    <section class="page student-work-page" data-route-surface="studentWorkInspection">
      <p class="eyebrow">Student Work</p>
      <h1>Student Work inspection is not available</h1>
      <p class="page-lede">
        This Live Demo Gradebook provides answer-free immutable grading evidence. Individual Student
        responses and grading details are not part of this browser capability.
      </p>
      <A class="primary-link" href={`/instructor/courses/${encodeURIComponent(course)}/gradebook`}>
        Return to Gradebook
      </A>
    </section>
  );
}
