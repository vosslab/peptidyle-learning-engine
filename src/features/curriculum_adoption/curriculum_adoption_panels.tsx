// curriculum_adoption_panels.tsx - receipt evidence surface for completed live curriculum changes.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import type { CourseReference } from "../../../generated/api/CourseReference";
import type { CurriculumAdoptionReconciliationResult } from "../../../generated/api/CurriculumAdoptionReconciliationResult";
import type { CurriculumAdoptionReceiptBinding } from "../../../generated/api/CurriculumAdoptionReceiptBinding";
import { courseRouteReference } from "../../navigation/public_route";

export function ReceiptPanel(props: {
  readonly courseReference: CourseReference;
  readonly receipt: CurriculumAdoptionReceiptBinding | undefined;
  readonly reconciliation: CurriculumAdoptionReconciliationResult | undefined;
  readonly onInspect: () => void;
}): JSX.Element {
  const reference = courseRouteReference(props.courseReference);
  return (
    <section class="curriculum-adoption-receipt" aria-label="Completed curriculum adoption">
      <h2>Live change complete</h2>
      <p>
        The server recorded an immutable receipt for this adopted curriculum. Its idempotency
        binding is retained privately by the browser contract.
      </p>
      <Show when={props.reconciliation}>
        {(result) => (
          <p role="status">
            {result().kind === "alreadyConsistent"
              ? "Import projections are already consistent."
              : "Import projections were repaired from immutable evidence."}
          </p>
        )}
      </Show>
      <div class="curriculum-adoption-receipt-actions">
        <A class="primary-link" href={`/courses/${reference}`}>
          Open course
        </A>
        <A href={`/instructor/courses/${reference}/curriculum`}>Inspect imports</A>
        <button type="button" onClick={props.onInspect} disabled={props.receipt === undefined}>
          Check receipt evidence
        </button>
      </div>
    </section>
  );
}
