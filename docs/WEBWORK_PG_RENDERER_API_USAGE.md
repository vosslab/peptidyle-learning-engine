# WeBWorK PG renderer API usage

PLE uses the standalone `webwork-pg-renderer` service as an opaque Question
Backend. WeBWorK owns PG/PGML rendering, document structure, controls,
interaction semantics, response interpretation, grading, partial credit, and
backend state. PLE owns authorization, immutable Question Revision selection,
Assessment Attempt lifecycle, persistence, and recorded outcomes.

Question Type is immutable author-declared educational metadata on the
Published Question Revision. PLE uses it for labels, filtering, and search;
it does not infer Question Type from a WeBWorK document or controls.

## Runtime boundary

The renderer is a private Podman service. Student browsers call PLE only:

```text
browser -> PLE API -> private webwork-pg-renderer -> WeBWorK PG
```

The local stack builds `localhost/pg-renderer:reviewed` from the maintained
sibling checkout `../webwork-pg-renderer`. PLE records the selected image
reference and its OCI configuration ID in private Local Stack State. That OCI
ID is the `QuestionRendererVersion` retained with each rendered Question; PLE
does not maintain a second renderer-version value.

The renderer has no public host port, PLE database access, student session,
or persistent educational record. PLE calls its `POST /render-api` endpoint
with form data over the private service network. The adapter accepts only the
expected JSON envelope, checks its private renderer state, then discards that
state before the browser-facing document is stored or served.

## Render request

PLE derives the PG source, source path, Question Revision, seed, renderer
identity, PLE origin, and asset origin from trusted attempt and deployment
state. The browser supplies none of them.

Each render request includes the server-owned fields below. The PG source is
base64 encoded in `problemSource`.

| Field | Value or purpose |
| --- | --- |
| `_format` | `json` |
| `problemSource`, `sourceFilePath`, `problemSeed` | Immutable source and attempt seed |
| `outputFormat` | `ple_embed` |
| `pleOrigin` | Trusted PLE origin |
| `pleAssetBase` | `{pleOrigin}/api/webwork-assets` |
| `displayMode` | `MathJax` |
| Student display flags | Server-selected embed settings |

`ple_embed` is a renderer format for PLE documents. It is not a PLE parser or
a second Question presentation format.

## Backend-owned document

The adapter stores `renderedHTML` verbatim as one immutable backend document
per selected WeBWorK Question in an Assessment Attempt, along with its SHA-256
and renderer OCI identity. The Student document route re-authorizes the current Student and
attempt position before returning that exact document with `no-store`.

The embed document omits renderer JWT inputs, form action, submit controls,
footer, and base element. PLE can therefore capture ordinary PG form data
without exposing renderer credentials. Legitimate PG hidden answer or control
fields remain normal form fields and are not removed.

PLE presents the document in a same-origin iframe with:

```text
sandbox="allow-scripts allow-forms allow-same-origin"
```

The document route adds a CSP that permits same-origin scripts and styles,
same-origin or data images, and blocks base URLs, objects, form navigation,
and external framing. This is the C2 decision recorded in
[webwork_opaque_render_findings.md](archive/reports/webwork_opaque_render_findings.md).

## Presentation and assets

WeBWorK owns Question-authored presentation. PLE supplies the generic
`/styles/ple_embed.css` baseline so a backend document fits the Student UI.
It sets document colors, a readable default typeface, available width,
inherited form fonts, and a visible focus outline. It does not recognize or
style PG control types, Question Types, macros, or individual elements.

The renderer template orders styles as follows:

1. Renderer third-party styles.
2. PLE's generic `ple_embed.css` baseline.
3. PG `extra_css_files`.

PG-authored styles intentionally follow and can override the PLE baseline.

Every renderer asset URL in an embed document is query-free and uses one of
these PLE proxy namespaces:

| PLE path | Renderer path | Cache policy |
| --- | --- | --- |
| `/api/webwork-assets/webwork2_files/...` | `/webwork2_files/...` | Public for one day |
| `/api/webwork-assets/pg_files/...` | `/pg_files/...` | `no-store` |

The proxy accepts only `GET` for those two paths. It rejects other prefixes,
queries, encoded path escapes, redirects, unsafe media types, and oversized
responses. It forwards from the configured private renderer origin; the
browser never selects an upstream URL.

## Submission and grading

The PLE bridge is loaded only inside the backend-owned document. It serializes
the first form with `new FormData(form)` into a canonical JSON array of
`[name, value]` pairs. This preserves form order and duplicate names, including
repeated checkboxes and legitimate PG hidden fields. The bridge sends the pair
array to the parent for ordinary submit events and for a parent-requested
capture; the capture reply carries a per-request identifier.

PLE stores the opaque bounded pair array as the shared `BackendOwned` Student
Response. It does not inspect a field name to determine control behavior or
Question Type. Before grading, the adapter rejects invalid or server-owned
names, including source, seed, output, display, PLE-origin, JWT, credential,
answer-key, submission-control, and `showCorrectAnswers*` fields. It forwards
all remaining pairs in their original order, then appends its own
`submitAnswers=1` to the trusted render fields.

The selected E1 lifecycle is stateless: PLE sends one grade request using the
immutable source, seed, and submitted pair array. It stores no WeBWorK state
row and passes `None` through the shared backend-state slot. The renderer's
normalized score is validated and recorded as the PLE grading outcome; answer
keys and renderer-private state never enter the Student response or document.

## Completed-Attempt correct answers

`WebworkRenderer::render_answer_review` and
`WebworkAdapter::answer_review_document` are server-only operations. They reuse
the verified immutable source, registered PG path, exact Revision, and stored
Attempt seed. Their private form adds `showCorrectAnswers=1` and
`showScoreSummary=0` while retaining `outputFormat=ple_embed`, `isInstructor=0`,
disabled hints and solutions, and hidden summaries, messages, and controls.
They supply no response pairs or submission flags. Ordinary issue, preview,
resume, and grading omit `showCorrectAnswers`.

The renderer preserves its embed format and lets PG present correct answers
without enabling hints or solutions. Protected generated answer assets stay
inline inside the authorized HTML; they are not published under `pg_files`.
Unsupported asset handling and upstream PG errors fail review. The adapter
checks the existing closed envelope, size/deadline bounds, private JWT
reflection, and the upstream error flag. Only `renderedHTML` leaves the server.

An owning Student fetches
`GET /api/assessment-attempts/{assessment_attempt}/questions/{position}/answer-review-document`.
The route accepts no query or body. It authorizes completed history and the
copied answer-disclosure policy before resolving source or calling the renderer,
then repeats the history/cohort decision after rendering. The final database
read is the disclosure decision's read instant. It holds no database lock over
renderer I/O. A newly current Student can reclose the Quiz/Exam gate; expiration
alone does not replace committed automatic Submission evidence.

The optional history marker `backendAnswerReview: "available"` means disclosure
is permitted at that history read, not that rendering has succeeded. The Answer
section uses `OpaqueWebworkPreviewFrame`, titled "Correct answer", with only the
existing bounded resize message. Its frame and response-level CSP both grant
`allow-scripts` without forms or same-origin access. Responses are UTF-8 HTML,
`no-store`, `nosniff`, and `no-referrer`; direct navigation remains sandboxed.
Withheld, foreign, unavailable, or wrong-backend positions return concealed
404 without a renderer call. Authorized source/renderer failures return generic
sandboxed 503 HTML. Retry reloads authorized history and the frame, without
save, submission, or grading. The transient document replaces no retained work.

## Renderer fork and verification

The maintained renderer fork carries only the PLE functionality needed for
this boundary: the additive `ple_embed` template, its format selection, the
two generated-asset URL namespaces, and the static `/webwork2_files/` alias.
Keep future renderer changes small and tied to a demonstrated functional need.
Each fork change increases the merge surface for upstream WeBWorK renderer
updates.

`devel/webwork_render_probe.py` is a development probe for a running renderer.
It renders `ple_embed`, can send ordered form pairs for evaluation, records the
response envelope and document observations, and writes evidence outside the
tracked source tree. The one-time representative findings selected C2 and E1;
they are implementation evidence, not a permanent compatibility corpus or
fixture set.

Permanent tests cover stable contracts such as request construction, envelope
validation, ordered-pair forwarding, server-owned field refusal, score mapping,
document authorization, and the two-prefix asset route. Connected validation
uses a real local renderer and browser path when a renderer or document-boundary
change needs evidence. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for
the test-lifetime policy and [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md)
for the shared Question Backend lifecycle.
