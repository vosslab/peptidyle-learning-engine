# Human Guidance evidence reconciliation

Status: planned. This plan owns unresolved current-scope work in
[human_guidance_implementation_checklist.md](../audits/human_guidance_implementation_checklist.md).
The [October 4 audit](../../archive/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md) and
[earlier compliance plan](../../archive/human_guidance_implementation_compliance_plan.md) are
historical inputs. Their counts and findings must be reconciled with later fixes and current
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), not treated as fresh acceptance or new product scope.

## Autonomous ownership

The manager coordinates fresh implementation, test, and independent review agents by coherent
product area. Agents provision local disposable services and seeded Accounts, run browser
workflows, assess captures, and resolve engineering choices from current guidance. Local SMTP
acceptance replaces a dependency on a human email account; real-provider delivery, deployment,
and credentials are outside completion. Existing deferred product choices remain deferred.

## Milestones and recovery

1. **Establish current scope.** Run the checklist `--diff` and `--consistency` checks. Reconcile
   the October 4 confirmed mismatches and pending rows against subsequent ledgers, changelog,
   source, and current authority. Classify each coherent area as evidence-ready, needing fresh
   verification, needing implementation repair, or explicitly deferred/not a product behavior.
   Correct stale mappings and copied evidence at their maintained source parts.
2. **Verify behavior.** Test agents reuse existing focused SQL/API/browser checks for the current
   requirements. Produce one-time checks where necessary; no requirement needs a human walkthrough
   or interview. Independent reviewers assess whether evidence demonstrates the actual consumer
   and permission boundary. Unsupported claims return to the test agent for concrete proof.
3. **Repair current gaps.** Assign every demonstrated current-scope defect to a bounded owner task
   or an existing active plan, including the UI and release follow-ups in
   [current_work.md](current_work.md). The manager carries those repairs through implementation,
   focused behavior verification, and independent review; creating a task or TODO does not close
   the parent requirement. Resolve ordinary design choices through an independent architect agent.
4. **Reconcile deferred work.** Record authority-backed deferrals in TODO and mark non-product
   guidance as such with reasons. Preserve incomplete evidence honestly. Deferrals do not require
   human reapproval, and they cannot be invented merely to make a failing current requirement pass.
5. **Close the evidence loop.** Update the maintained checklist parts and regenerate the checklist.
   Run `source source_me.sh && python3 devel/human_guidance_checklist.py --diff`, `--consistency`,
   applicable focused checks, and `source source_me.sh && ./launchers/all_test.sh`. Capture changed
   UI through the existing workflow. A fresh review agent verifies every current-scope area has
   accepted evidence and each deferral has authority. Repair failed checks and repeat affected
   verification; the manager then records completion and archives this plan with `git mv`.

## Evidence policy

Use coherent behavior coverage rather than one permanent test per guidance bullet. Apply
[PYTEST_STYLE.md](../../PYTEST_STYLE.md#permanent-test-checklist); keep one-time probes in ignored
`tests/_temp/` and remove them after recording results. Existing passing receipts may support
unchanged boundaries when their provenance is established; label their date and limits. Neither
an unchecked historical row nor an old passing aggregate alone establishes current behavior.
