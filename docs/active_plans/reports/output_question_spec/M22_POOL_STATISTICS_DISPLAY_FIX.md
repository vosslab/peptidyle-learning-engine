# M22 Pool statistics display fix

## Source trace

- `QuestionPoolView.evidence` is already a required `QuestionStatistics` field in
  `generated/api/QuestionPoolView.ts`. The strict Pool decoder requires the field and calls
  `decodeQuestionStatistics` in `src/api/decoders/question_pool_detail.ts`.
- The Pool API already builds this evidence from deliveries attributed to the originating Pool.
  The Pool projection has no per-Revision list; it does not derive outcomes from current members or
  combine historical Question Revisions. Existing contributor-floor privacy behavior remains in
  place.
- Question detail already mounts `QuestionStatisticsPanel` with its own evidence. The Pool detail
  now mounts that same panel with `value().evidence`, immediately after Pool metadata. The Question
  detail caller, its exact-Revision evidence, and its optional per-Revision breakdown are unchanged;
  the shared labels and credit display format apply to both routes.

## Display

The shared panel uses the heading **Statistics**. It explains that Received counts committed
deliveries, Graded responses have stored credit, Average stored credit is the mean for graded
responses, and full-credit and zero-credit percentages use graded responses. Counts beside
percentages and average credit show the graded-response denominator. Average stored credit is
formatted as a percentage with the same formatter as the two rates.

When the existing API marks evidence unavailable, the panel says: "Statistics are unavailable until
enough Students have contributed to protect their privacy." No threshold, endpoint, counter, or
privacy rule changed.

## M29 browser handoff

The Pool detail now exposes a section headed **Statistics**, with the shared panel's existing
`question-statistics-panel` class and Received, Graded responses, Average stored credit, Full credit,
and Zero credit measures when available. `tests/playwright/e2e/m29_library_scoring_evidence.spec.ts`
currently expects the old **Learning evidence** heading and the sentence "Question Statistics are
unavailable until shared learning measures can be shown." for Question detail, and checks for the old
**Graded** and **Mean credit** labels. Its M29 owner should update those assertions to **Statistics**,
"Statistics are unavailable until enough Students have contributed to protect their privacy,"
**Graded responses**, and **Average stored credit**. The current
`output_question_spec/M29_REMAINING_BROWSER_COVERAGE.md` also says the Pool panel is missing; its M29
owner should refresh that source-coverage statement. No M29 test, registry, or coverage report was
changed here.

The current seeded Course has three Students while the existing statistics privacy floor is five.
Its browser scenario can verify the withheld panel, not numeric Pool metrics. Numeric browser proof
needs an existing safe cohort with at least five distinct Student contributors and remains with the
root runtime owner.

## Checks and limits

- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` - passed.
- `source ./source_me.sh && npx prettier --check src/pages/question_pool_detail.tsx src/pages/question_statistics_panel.tsx` - passed.
- `source ./source_me.sh && ./node_modules/.bin/eslint src/pages/question_pool_detail.tsx src/pages/question_statistics_panel.tsx` - passed.
- `git diff --check -- src/pages/question_pool_detail.tsx src/pages/question_statistics_panel.tsx` - passed.
- No panel-render test exists. No API, decoder, SQL, database, container, full build, generated TypeScript, browser, or five-contributor proof was run.
