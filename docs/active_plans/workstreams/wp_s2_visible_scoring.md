# WP-S2 visible scoring and instructor outcome

## Status

**IMPLEMENTED OFFLINE; AWAITING RETAINED-STACK EVIDENCE.**

## Scope

- J5 signs the local instructor in through the visible form, opens the exact public course, and
  uses visible assignment pagination to bind the exact public assignment link to its rendered card
  heading. The heading is browser-local selector data only; it is never placed in protected state
  or a report.
- From the rendered course management navigation, J5 reaches Gradebook with Tab and Enter. It
  reaches the one exact visible assignment row through visible gradebook pagination and asserts the
  row's displayed Best `100%`, Latest `100%`, and Completed `2` cells.
- J5 uses Tab and Enter to expand only that row's View run history control, then requires exactly
  two visibly completed list entries in run-number order. It neither reads nor records learner ID,
  timestamps, or per-run scores.
- `j5_v2_handoff.ts` is deliberately a public-ID-only temporary seam. The isolated
  `v2_j5_j8_state.ts` boundary consumes the exact J11--J4 public prefix and appends J5 only after
  the browser assertions pass. It then lets the no-output J8 child descriptor-safely re-read the
  exact J11--J5 prefix, cross-bind only course and assignment IDs, and append J8's closed
  cross-actor vocabulary. Scores, title, learner identity, and run details never enter state or a
  report. Historical schema-v1 state/report modules remain unchanged.

## Acceptance boundary

No score polling is permitted. Completion writes the run and gradebook summary transactionally;
J5 waits only for normal visible Gradebook and run-history loads. Live evidence remains pending the
serial WP-S1 student repeat integration and the final WP-E1 retained-stack report gate.
