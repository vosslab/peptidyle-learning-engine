# Ribbon task model

## Purpose and authority

This model applies the navigation decisions in
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). It separates stable product
navigation from page-local actions and current implementation gaps.

The Ribbon keeps its declared rows and geometry stable while content loads,
changes, becomes empty, or reports an error. Route admission is presentation,
not authorization; the server rechecks every protected request.

## Product frame

The top bar owns application identity, current Product Role, and Profile access.
Sign Out belongs inside the Profile menu rather than beside the primary tabs.
The Context Row identifies the current Course, Assessment, or other scope. The
Task Row contains sibling destinations for the selected primary area. Deep
pages use breadcrumbs and retain one visible page heading.

Loading, denial, an empty collection, or a route error changes the content
area, not the surrounding Ribbon geometry. Focus moves to the main content
heading after successful navigation and to a useful recovery target after an
error.

## Instructor product navigation

The primary tabs are:

1. Courses
2. Questions
3. Assessments

Their Task Rows are:

| Primary tab | Tasks |
| --- | --- |
| Courses | My Blueprint Courses; My Active Courses; My Inactive Courses; Search Public Blueprint Courses |
| Questions | My Questions; My Draft Questions; Starred; Watched; Search Question Library; Browse Question Library |
| Assessments | Assessments Due Soon; My Assessment Templates |

Search and Browse are different interactions. Search Question Library exposes
an explicit query/filter workflow. Browse Question Library supports
discovery without pretending it is the same operation.

Required backed destinations remain visible when their collection is empty and
the content area explains how to create the first item. A genuinely
unimplemented future capability is not displayed as a usable control. An
implementation that lacks a Human-Guidance-required destination is a product
gap to record, not authority to delete that destination from this model.

## Course Instance navigation

Course pages expose only implemented Course-local teaching tasks and preserve
the Course identity in context. Every current co-Instructor sees the same
teaching destinations; the creator or first Instructor has no special set.

Assessment composition is the **Assessment Question Editor**. Settings are the
**Assessment Properties Editor**. Create Assessment is a page action in the
appropriate content, not a competing primary Ribbon tab.

Student View is an Instructor preview that retains Instructor identity and
creates no Student Work. It is not a Product Role switch.

## Student navigation

Student work is collectively **Coursework**. A specific item uses its exact
Assessment Type name. Within an Assessment Attempt, Question navigation and
saved status stay in the content area while the Ribbon and context remain
stable.

The return action is **Back to Coursework**. The completion action is **Submit
Assessment** and submits the whole Attempt, finalizing all saved responses
together.

## Sysadmin navigation

The Sysadmin Ribbon exposes implemented platform administration and scoped
support entry points only. It does not imply Course membership or ambient FERPA
access. Human Guidance does not specify a complete fixed Sysadmin tab/task list,
so this document does not invent one.

## Responsive and keyboard behavior

- Preserve source order and meaningful labels at every viewport.
- Collapse presentation without reordering tasks or hiding the selected
  destination.
- Keep the primary keyboard path available through native links and buttons.
- Announce pending navigation and content errors without remounting the whole
  shell.
- Do not use icon-only controls except where the visible surrounding context and
  accessible name make the action unambiguous.

## Current implementation evidence

[RIBBON_DESTINATION_LEDGER.md](RIBBON_DESTINATION_LEDGER.md) is generated from
the current route catalog and capability registry. Its old labels and omitted
required destinations are implementation gaps. The ledger must not be read as
permission to replace the product model above.

[FRONTEND_CAPABILITY_INTEGRATION.md](FRONTEND_CAPABILITY_INTEGRATION.md) defines
how a real destination becomes usable without confusing visibility with server
authorization.
