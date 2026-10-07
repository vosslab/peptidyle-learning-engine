# Question-spec implementation uncertainties

Purpose: record unresolved product decisions that are expensive to change after release or block
release, with concise dispositions for decisions already reviewed.

## Current review

The M01 coverage reconciliation found no new material uncertainty and did not reopen questions
already deferred or settled. The M12 source review's cross-entry duplicate-tuple question below is
withdrawn: the human already rejected speculative overlap gates as bikeshedding. See the existing
[Question-spec open questions](question_specs_open_questions.md) log for other deferred product
decisions; do not duplicate them here. Re-evaluating submitted Native JSON responses after an
answer-key or grading-rule correction is deferred and stays outside the grill unless it blocks
release. Assessment point-value and partial-credit changes recalculate points from stored Backend
credit without another Backend evaluation. Current bulk-edit evidence has one corrected documentation
detail recorded in the implementation spec. Draft autosave implementation drift was corrected under
existing Human Guidance. The numeric editor behavior below is settled as an implementation detail;
Pool search metadata storage and calculation timing are also settled below, with scheduled backend
recalculation explicitly deferred. The 2026-10-07 Native JSON image-reference question below is
withdrawn: it mistook an incomplete converter-to-server trace for uncertainty in the established
image identity model. A separate read-only audit of representative Revision identity, scoring,
and Pool reuse areas found no further product uncertainty; it was not a full-repository proof.

Integrated retries on 2026-10-06 raised no new product question. The connected grading lifecycle
failure and isolated lifecycle-module import cycle are implementation/integration issues, not
uncertainties for Neil to decide. Their evidence and pending status are recorded in the
[implementation ledger](../reports/question_spec_implementation_ledger.md#integrated-retry-evidence-2026-10-06).

## Open question

There is no current fundamental image-identity question. The existing
`QuestionImageAssetTuple` remains authoritative; converter-to-server binding and storage integration
are implementation work tracked in [TODO](../../TODO.md#future-product-capabilities).

## Withdrawn question

### Native JSON image references across the converter and PLE boundary (withdrawn, 2026-10-07)

- **Correction:** Keep `QuestionImageAssetTuple={questionImageAssetId, checksum}`. Logical image
  identity remains distinct from the physical object-store identity, and the checksum is SHA-256
  of verified bytes. QTI derives its logical ID from those bytes at bind time. Other creation paths
  may mint a logical ID independently; neither strategy is a universal requirement.
- **Evidence:** The tuple is defined in
  [`question_content.rs`](../../../crates/question_model/src/question_content.rs); logical asset and
  `ObjectId` are distinct in [`identity.rs`](../../../crates/question_model/src/identity.rs). The
  QTI adapter verifies image bytes before binding the tuple and derives the ID from their SHA-256
  checksum in [`parser.rs`](../../../crates/adapters/qti/src/parser.rs) and
  [`model.rs`](../../../crates/adapters/qti/src/model.rs). Draft upload mints a UUIDv7 logical ID
  in [`draft_question_images.rs`](../../../crates/server/src/draft_question_images.rs), while
  publication preserves that logical ID and creates physical object identity in
  [`question_publication_images.rs`](../../../crates/server/src/question_publication_images.rs).
  Native HOTSPOT compilation accepts the same tuple in
  [`source_compile.rs`](../../../crates/adapters/ple/src/question_json/source_compile.rs).
- **Scope:** The QTI parser/worker handoff exists, but the actual server integration and external
  `qti-package-maker-rs` Native JSON generator remain future engineering work. That gap does not
  change the existing source format or identity rule. Converter binding, storage integration, and
  transport details remain implementation work.
- **Status:** Withdrawn because the question proposed changing an already-settled identity model
  from an incomplete integration trace. Preserve the tuple and complete the import integration
  under the existing import TODO.

## Entry format

Add an entry only for an unresolved decision that would be expensive to change after release, such as
persisted schema or meaning, identity/ownership/relationships, Revision/Edit Number, foundational
domain boundaries, cross-layer contracts, major architecture, or a release blocker. Check Human
Guidance and explicit decisions first; correct drift from settled guidance. Split prerequisite
decisions into atomic questions and ground recommendations in evidence. Resolve implementation
choices from repository guidance, tests, or engineering review. Leave cheap UI preferences, transient
state, and explicitly deferred features for later.

Each entry records:

- **Question:** the specific unresolved point.
- **Authority/evidence:** exact document sections, code, test, or observed result.
- **Milestone/consequence:** affected plan milestone and what depends on the answer.
- **Recommendation:** only when existing evidence grounds one.
- **Temporary approach:** how settled independent work proceeds meanwhile.
- **Status:** open, resolved with a decision link, or withdrawn with reason.

## Question disposition

### M12: cross-entry duplicate-tuple question (withdrawn)

- **Question:** The source review asked whether a Question Revision Tuple selected by one Assessment
  entry must be excluded from other entries in the same Attempt.
- **Authority/evidence:** Human Guidance says each Assessment sets its own count from a reusable Pool
  ([Question Pool use and selection](../../HUMAN_GUIDANCE.md#question-pool-use-and-selection)). The
  human had already rejected speculative overlap gates, including a directly added Question also
  appearing in a Pool, as "bikeshedding speculative gates." Ordinary selection samples without
  replacement within each entry's own Pool selection; that per-entry implementation evidence does
  not establish a cross-entry deduplication requirement. No broken content workflow supports adding
  that gate.
- **Milestone/consequence:** No additional M12 selection gate. The repeated-Pool release oracle
  checks each entry's requested count and remains valid ([M12 release validation
  report](../reports/QUESTION_SPEC_M12_RELEASE_VALIDATION.md)).
- **Recommendation:** Keep existing per-entry behavior; add no cross-entry rule.
- **Temporary approach:** Keep the existing per-entry sampling and continue per-entry release
  validation.
- **Status:** Withdrawn because the human already rejected speculative overlap gates and no broken
  content workflow justifies reopening this hypothetical.

### Draft Question: invalid numeric editor input (settled implementation detail)

- **Decision:** Typed numeric fields retain their types. Temporarily invalid input such as `6.02e`
  stays local in the editor until it becomes valid; Draft autosave preserves the valid,
  representable source state. Save does not run Question Publication Validation.
- **Reason:** Draft autosave and publication-only content requirements follow [Draft Question
  guidance](../../HUMAN_GUIDANCE.md#draft-question-specifications). Invalid raw text is not part of
  the typed source, so the storage contract need not change.
- **Status:** Settled implementation detail; no product question remains.

### Question Pool calculated metadata (settled; periodic backend job deferred)

- **Decision:** Store calculated Pool metadata on the Pool. Keep these values up to date when the
  Pool is created or saved. Search reads the stored values. Defer periodic backend cron
  recalculation.
- **Reason:** Stored values avoid calculating Pool metadata during search. This keeps existing field
  ownership, first-member Discipline/Subject, and license calculation rules intact; it adds no
  derived-field list or scheduled job.
- **Status:** Settled; scheduled backend recalculation is explicitly deferred.
