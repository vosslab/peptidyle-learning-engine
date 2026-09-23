# Plan: Role tier-two navigation

## Context

The shell work reserves tier-two space for all signed-in roles. Instructor tier-two controls already
have Human Guidance-backed tasks; Student and sysadmin rows may correctly remain empty. This record
decides contents only. It does not revisit row height, Ribbon geometry, or route access.

The current fast gate is red. This evidence decision may be completed now, but no resulting
implementation begins until stabilization is green.

## Objectives

- Decide whether each Student and sysadmin tier-two row has a demonstrated task control.
- Keep a reserved empty row valid when no demonstrated control exists.
- Close the current contents decision from repository evidence without inventing a human approval
  event.

## Design philosophy

A gap is not a product requirement. Reserve layout space for stability, but add a control only for
a named, reachable workflow. Filling the row with generic shortcuts is rejected because it invents
navigation rather than serving a task.

## Scope

- Review Student and sysadmin reachable routes against current Human Guidance and route contracts.
- Record `empty` for a role unless the evidence admits a finite list of task controls.
- Require each admitted control to name its existing route, task, access boundary, and label.

## Non-goals

- Do not change tier-one navigation, row reservation, or shell CSS.
- Do not invent destinations, workflows, or role capabilities.
- Do not implement controls in this decision record.

## Current state summary

The parent plan separates "does the row take space?" from "does the row hold controls?" The first is
already yes for every signed-in role. Human Guidance says Student tier-two tasks remain unsettled
and that the complete Sysadmin Ribbon task layout is not locked. Current evidence does not admit a
Student or sysadmin tier-two workflow. Their reserved rows therefore remain empty: that is the
current decision, not an omission to mask.

## Approach

1. Inventory each existing Student and sysadmin route that could be proposed as a tier-two control.
2. Admit a control only when repository evidence demonstrates all four facts: an existing,
   role-authorized destination; a named recurring task; a within-context use distinct from tier-one
   navigation; and no better page-local location near the affected content.
3. Record `empty` for each role when no candidate satisfies all four facts. Current rows stay empty
   until such evidence exists.
4. Close this decision after recording the evidence outcome. A non-empty outcome is a cross-cutting
   design change: escalate it to the architect and create one separate implementation plan after
   stabilization. Do not implement controls from this record.

## Acceptance criteria and gates

- Every admitted control has an existing destination and role authorization evidence.
- No control exists merely to occupy the reserved row.
- Adding an admitted control cannot change the reserved row's height or move page content.

## Decision record

- Current decision: Student and sysadmin tier-two rows are `empty`. The cited Human Guidance leaves
  their task contents unsettled or unlocked, and this record has no demonstrated workflow satisfying
  the Approach evidence rule.
- Evidence rule: replace `empty` only when a control maps to an existing, role-authorized, recurring
  within-context task that cannot be better placed near its affected content.
- Follow-up: only a non-empty evidence outcome requires architect review and a separate
  implementation plan after stabilization; this record remains decision-only.
