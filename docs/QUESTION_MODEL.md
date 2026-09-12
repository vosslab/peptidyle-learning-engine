# Question model

PLE separates a reusable published Question from the exact immutable content
that was used. This guide describes the answer-free shared model and the
database facts that make that distinction durable. The terminology authority is
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md); the detailed human-facing
rules are in [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

## Question identity and lineage

A **Question ID** identifies one stable published Question lineage. It is a
human-facing reference, not a UUID, sequence number, credential, or authority
decision.

- PostgreSQL stores the compact seven-character uppercase Crockford Base32
  value, for example `7K3M9QP`.
- Browser display and copy use the grouped `AAA-BBBB` form, for example
  `7K3-M9QP`. The hyphen is presentation only.
- The first six characters are generated from a cryptographically secure
  source. The seventh is the server-validated HMAC-SHA-256-derived check
  character. Syntax parsing accepts documented transcription normalization;
  the server validates the check character before resolving the lineage.
- Creation order is not represented in the identifier. A uniqueness conflict
  at the Question-ID boundary causes a newly generated candidate to be tried.

`QuestionRevisionReference { question_id, revision_number }` identifies one
immutable published Question Revision. A revision number is positive and
monotonic only within its lineage. A Draft Question has a private workspace
identity and Draft Question Edit Number, but no Question ID or Question
Revision.

The database keeps the stable lineage in `published_question` and immutable
versions in `question_revision`. A Published Question has current
**Available** or **Archived** state and a qualified Availability Edit Number.
Append-only availability events retain actor, transition, edit number, reason
where required, and time. Publishing a later revision does not change that
current availability.

Available lineages appear in ordinary Question Library browsing and can be
newly selected. Archived lineages are absent from ordinary browsing and new
selection, while authorized exact revision reads, retained Assignment pins,
Issued Question evidence, grading, and audit continue to resolve. Archive is
an explicit owner transition with the exact title confirmation and Edit Number;
restore is the corresponding revision-checked transition.

Question IDs are references, never bearer tokens. A valid ID does not reveal
existence, grant Question Library access, establish ownership, or grant course
or Student authority. Library access is limited to the appropriate active
Instructor path; Student content is available only through authorized
Assignment access.

## Drafts, publication, and stewardship

A Draft Question is mutable authoring state in an Authoring Workspace. Draft
saves use its Draft Question Edit Number. Its metadata and complete source
binding are private. Publication validates the complete Draft and atomically
creates either:

- a new lineage, its first immutable Question Revision, initial Available
  state, owner, reviewed authorship, license, acceptance, exact source-object
  binding, and publication/availability evidence; or
- a later immutable Question Revision in an existing lineage, after locking
  the lineage, verifying current owner and workspace authority, and checking
  the expected parent revision.

The published source binding preserves the exact backend, format-specific
routing facts, source Object Record, checksum, size, media type, and object
address for that Question Revision. It is not reconstructed from mutable Draft
state. Publication evidence records the editor, accepter, reason, parent
revision where applicable, authorship, license, ownership, and fork ancestry.
These revision and stewardship facts are immutable.

The stable lineage holds current discovery metadata such as title, description,
and language. A same-lineage publication can update that current metadata while
leaving every earlier Question Revision and its provenance intact. A semantic
change requiring a different Question lineage is published from a fork Draft
and receives a new Question ID. A same-lineage publication receives the next
revision number under the existing ID.

Question ownership is an append-only lineage history. The current owner is
derived from it; authorship is historical credit, not owner authority.

## Assignment pins and Student Work evidence

An Assignment is one current aggregate with an Assignment Edit Number. It is
not a revision lineage. A fixed Assignment Entry and every Question Pool Item
pin an exact `QuestionRevisionReference`. A current Assignment update may
deliberately choose a different Available revision. Publishing a Question,
archiving or restoring its lineage, background work, and correction processing
do not advance an Assignment pin implicitly.

Released Assignment edits that pass current release validation affect future
Attempts. Existing Student Work remains interpretable because an Assignment
Attempt retains its effective policy and schedule facts, and each Issued
Question retains:

- its stable Assignment Entry identity and issued position;
- the exact Question Revision Reference;
- its server-generated Question Seed and reproduction/presentation binding;
- point value, scoring rule, statistics eligibility, and effective
  per-question limits; and
- Question Pool selection and item evidence when it came from a pool.

Question Pool selections preserve the selected exact revisions and their
historical source identities. They are evidence, not pointers to mutable
current Assignment configuration. Attempt delivery, response saving,
submission, grading, history, and statistics read retained Attempt and Issued
Question evidence; they do not reconstruct old work from current Assignment
content.

## Source, assets, and presentation

`crates/question_model` contains answer-free shared contracts that can cross a
browser-facing boundary. Correct responses, answer keys, and grading-only
facts remain server-side in `crates/grading` and backend-specific trusted
paths. A Question Backend retains its full format-specific source and maps
authorized results into shared presentation, response, evaluation, and
feedback boundaries.

Question Revisions support the currently registered `ple`, `webwork`, and
`imathas` backends. Their exact source binding carries the backend and its
format-specific routing facts. PLE Question JSON is an answer-bearing
authoring format, not a second public Question model.

Assets are bound to an exact Question Revision. The private source Object,
verified public rendition, delivery record, checksums, media type, dimensions,
and publication job form one constrained publication relationship. A rendition
is used for Student delivery only after its exact publication is ready and its
delivery is available. Attempt presentation retains the rendition facts it
used, so later changes to delivery state cannot alter a resumed presentation.

`QuestionPresentation` is the narrow per-issued-question Student contract. It
contains only the rendering facts needed for the exact revision and seed, plus
presentation-scoped response references and consistency data. These references
and checksums detect an incoherent or stale presentation; they are neither
credentials nor correctness data. The server resolves submitted response
references only against the retained issued presentation and grades using the
exact retained Question Revision and backend source.

Question statistics are revision-specific aggregate evidence. They are derived
from eligible surviving observations, subject to the product's privacy and
authorization rules; raw Student responses and small-cell information are not
Question Library data.

## Change boundaries

Question Revision and Blueprint Revision are PLE's only product Revision
concepts. Assignment, Course schedule, retention, and proposal state use their
own current-state models where those capabilities exist.

Forced Question Correction preserves the flawed immutable revision and records
the authorized replacement and remediation evidence. It does not rewrite
Issued Question or grading history. Its exact operational behavior is owned by
the correction and Student Work contracts.

Question Change Proposal is not a persisted capability in this baseline. The
implemented paths are owner publication, fork publication, and Forced Question
Correction. A future proposal workflow belongs in [TODO.md](TODO.md) until it
has a bounded Store, Server, authorization, and user workflow; it does not
require proposal Revision tables or compatibility fields.

## Related documents

- [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) defines Question-ID generation,
  storage, validation, and display.
- [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines the semantic
  lifecycle and stewardship vocabulary.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines Student
  Work and presentation payload boundaries.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines account and
  course authorization boundaries.
