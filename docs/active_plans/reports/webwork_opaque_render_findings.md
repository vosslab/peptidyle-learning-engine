# WeBWorK opaque render findings

> **Dated implementation evidence.** This report records repository state and
> conclusions at its stated time. It is not current product authority;
> [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) supersedes conflicting product
> vocabulary, lifecycle, grading, role, authorization, and interface claims below.

## Scope

This is one-time connected evidence for the opaque WeBWorK boundary. It is not
a permanent corpus, compatibility specification, or fixture inventory. The
machine-readable observations remain temporary at
`/private/tmp/ple-webwork-opaque-20260912/m2_probe_final/findings.json`.

The final renderer image was
`296f4f4bba83563aec7092c7e89de128bac810a62e0f3215df784874490e4df3`.

## Fixed requirements

All five supplied representative inputs passed the embed-document requirements
recorded at `items.<item>.document.checks`: no renderer JWT hidden input, form
action, base element, submit container, footer, or onload handler; one PLE
bridge and baseline stylesheet; query-free renderer assets under the two proxy
namespaces; and no renderer-origin or protected markers.

The repeated-checkbox observation at
`items.repeated_checkbox.grades` preserved ordered duplicate form pairs and
received its expected partial score. The one-submit multi-part partial-credit
observations at `items.multipart_partial.grades` also received the expected
partial score. The generated graph fetch at
`items.generated_graph.generated_asset_fetches[0]` was HTTP 200,
`image/png`, and 4,641 bytes.

All fifteen shared JavaScript assets returned HTTP 200 with identity encoding
under Chrome request headers and parsed successfully. These observations are
evidence that the temporary proxy did not cause the common browser diagnostic.

## Selected runtime decisions

### E1: stateless one-grade-request submission

Select E1. For every comparison at
`items.<item>.grades[*]`, both `problem_result` and `problem_state` were
identical with and without the render-issued state in the one grade request.
This includes correct, incorrect, partial-credit, repeated-checkbox, Formula,
and generated-graph observations. WeBWorK therefore uses no state row and its
shared opaque state slot is `None` for this lifecycle. This evidence makes no
claim about continuation or progression workflows.

### C2: same-origin iframe with document CSP

Select C2: `sandbox="allow-scripts allow-forms allow-same-origin"` with the
planned document CSP. C1 failed the decision rule for real opaque-origin
behavior: all representative inputs reported browser errors and the Formula
input could not load MathQuill fonts from the opaque origin. C2 loaded each
frame, exposed controls, and established keyboard focus under the planned CSP.

The temporary C2 harness's `Invalid or unexpected token` message was later
identified as a generic bridge integration defect: object-valued capture traffic
collided with the renderer's `CSSMessage` listener. The bridge now sends the
renderer-expected strict string capture request. The post-repair connected
browser lane recorded zero iframe or page JavaScript errors and no recurrence of
the diagnostic. This closes that diagnostic for the exercised opaque lifecycle;
it does not claim broader PG compatibility.
