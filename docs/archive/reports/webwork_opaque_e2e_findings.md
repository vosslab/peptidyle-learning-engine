# WeBWorK opaque E2E findings

> **Dated implementation evidence.** This report records repository state and
> conclusions at its stated time. It is not current product authority;
> [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) supersedes conflicting product
> vocabulary, lifecycle, grading, role, authorization, and interface claims below.

## Scope

This record closes the one-time connected acceptance work for M12 of the
[webwork_opaque_backend_plan.md](../active/webwork_opaque_backend_plan.md).
It records product behavior observed through the local Developer Browser Suite.
It is not a permanent PG compatibility corpus, fixture catalog, or renderer
test harness. The temporary inputs and detailed receipts remain outside the
repository under `/private/tmp/ple-webwork-opaque-20260912/`.

The connected run used renderer OCI image
`296f4f4bba83563aec7092c7e89de128bac810a62e0f3215df784874490e4df3`.
The image includes the reviewed immutable PG checkout
`3ca5687eaa28bebde231043a1ea5609c04edd670`.

## Published and temporary work

The ordinary Pilot publisher publishes eight Questions to the Question Library:
four PLE Question JSON Questions and four WeBWorK Questions. The fixed Live
Demo sample Assignment contains only the four PLE-native Questions. Its
automated activity derives only those PLE-native Student Responses and refuses
`backendOwned` input as configuration drift. The published WeBWorK Questions
remain available for ordinary Instructor selection.

The connected lane used the ordinary product APIs to create and release a
temporary Course and Assignment containing a published WeBWorK Question
Revision, then started a Student Attempt. It saved one backend-owned response,
finalized that Attempt, and observed the worker result through Student history.
This exercises a real WeBWorK path without teaching Live Demo provisioning how
to construct a WeBWorK response.

The canonical response was the ordered duplicate sequence
`[["AnSwEr0001", "B2"], ["AnSwEr0001", "B2"]]`. Save and finalization both
returned HTTP 200. The worker recorded `completed|graded|false|0|1`, and the
Student history view reported `0.0/1.0`. This shows worker-to-history
propagation for the connected result; it is not an independent score-replay
claim.

## Document and asset boundary

The authorized backend-document route returned HTTP 200 with
`text/html; charset=utf-8`, `Cache-Control: no-store`, the selected C2 document
CSP, and `Cross-Origin-Resource-Policy: same-origin`. The iframe used exactly
`sandbox="allow-scripts allow-forms allow-same-origin"`.

Renderer and PG assets loaded through the two public WeBWorK asset-proxy
prefixes. A connected renderer stylesheet fetch succeeded through that route.
The document contains no renderer credential input. These observations support
the same generic document and asset boundary described by
[webwork_opaque_render_findings.md](webwork_opaque_render_findings.md).

## Built-browser interaction

The built browser signed in, navigated visibly to the temporary Student
Attempt, and selected an iframe form control generically. It did not inspect a
Question Type, PG macro, control name, or response value. PLE displayed
`Response saved.` after Save and the submitted-Attempt outcome after Finish.

The first browser run exposed a real integration defect: object-valued capture
traffic conflicted with the renderer's `CSSMessage` listener. The repair uses a
strict same-origin string capture request,
`ple.backendOwned.capture:<16 lowercase hex characters>`, and keeps the
closed, correlated backend-owned response message for the reply. It did not
change the renderer or introduce control-specific handling.

The post-repair run repeated the visible capture, Save, and Finish path on a
fresh temporary Attempt. It recorded zero iframe or page JavaScript errors and
no `Invalid or unexpected token` diagnostic. The only console/resource result
was the expected pre-sign-in authentication-session 401.

## Presentation evidence

The connected document contains exactly one `/styles/ple_embed.css` link. It
follows the three renderer third-party stylesheets and precedes later PG
stylesheets. The uncustomized document had the generic white background and
16px body padding. A PG-authored inline color and font weight remained computed
as authored. This verifies a minimal generic PLE baseline while WeBWorK and
the Question retain presentation ownership.

The selected published PG did not use `extra_css_files`, so this run does not
claim a connected external authored-stylesheet case. That omission is
proportionate: the maintained template directly places the PLE baseline before
the `extra_css_files` loop, the connected document verified the template's
later-PG-stylesheet ordering, and the connected inline override verified that
Question-authored presentation wins. The M12 review accepted this evidence
instead of adding an otherwise unneeded Question merely to exercise the loop.

## Form-field evidence limit

The selected connected PG source has no legitimate HTML hidden input. The
absence is a source limitation, not a restriction of the bridge. The M0
representative evidence excludes renderer JWT hidden inputs, while focused M9
and M11 boundary evidence establishes that generic capture and forwarding
preserve legitimate hidden fields, order, and duplicate names without
recognizing controls. No renderer credential was reintroduced.

## Result and limits

M12 is **ACCEPTED**. The durable product evidence is the focused opaque-payload
and bridge contract tests together with this one-time connected lifecycle and
built-browser observation. Projection-era WeBWorK end-to-end tests that assumed
PLE-native response shapes or radio controls were retired; no replacement
fixture corpus was created.

This evidence covers one published WeBWorK Question and one generic browser
interaction. It does not establish a compatibility promise for every PG
interaction, external authored stylesheet, or progression workflow. Future
work should use the same opaque boundary and add temporary connected evidence
when a real product requirement needs it.
