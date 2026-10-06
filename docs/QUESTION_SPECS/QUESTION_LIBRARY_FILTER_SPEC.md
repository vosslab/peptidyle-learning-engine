# Question Library filter specification

Filters select Library Objects from one combined result set. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#search-question-library-filters) and
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-pool-metadata).
Search syntax and controls must agree on the meaning of an object's fields.

## Object kind and Questions in no Pool

Kind selects Both, Questions only, or Pools only. A separate filter selects all Published
Questions or Questions in no Pool. This filter leaves Pool results available.

**Questions in no Pool** means Published Questions that belong to no Pool. Pool forks are Pools.
Questions in those Pools therefore do not match this filter. A Question still belongs to its
Pool even when that Pool is absent from the current search results.

| Kind | Questions to include | Results before other filters |
| --- | --- | --- |
| Both | No Pool | Questions in no Pool plus Pools |
| Both | All | All eligible Questions plus Pools, including member Questions |
| Questions | No Pool | Only Questions in no Pool |
| Questions | All | All eligible Questions, including members |
| Pools | Either | Pools only |

The first row is a tentative default candidate because displaying Pool members redundantly makes
search noisy. HG says "probably"; the final default remains open. Including members remains available. A Pool does not automatically
stand in for every member-only search match.

## Per-field meanings

| Filter | Published Question | Pool | Absent-value behavior |
| --- | --- | --- | --- |
| Ordinary text | Its own searchable Title, Description, and metadata | Its own searchable Title, Description, and metadata | No member-only match |
| Discipline/Subject | Question's current required values | Pool's required values shared with members | Required; NULL is invalid Library state |
| Topic/Subtopic | Question's own values | Pool's own values | Absent value fails a positive requested value |
| Tags | Question's Tags | Pool's Tags | Empty collection matches no positive Tag |
| Question Type | Declared Type on represented exact Revision | Common required member Type | Required; Pool is not excluded by kind |
| Backend | Backend on represented exact Revision | Common required member Backend | Required for both |
| License | Exact Question Revision's license | Calculated compatible Pool license | Required for both; never "Mixed" |
| Bloom dimensions | Pair on represented exact Revision | Pool's own pair | NULL fails a requested value, remains visible without that filter |
| Owner | Question owner | Pool owner | Required; do not substitute author |
| Author / authored by me | Question's attributed authors | No Pool Author exists | A positive author predicate does not match a Pool |
| Backend capability (existing implementation filter) | Declared capability of its Backend | Same predicate applied to its common Backend | Missing implementation does not justify excluding Pools |

The Backend capability filter is recorded as implementation evidence, not a new approved search
requirement. Its implementation needs review for consistent treatment of both Library Object
kinds. [question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md)
records that engineering work; it is not a separate Pool product decision.

A missing optional field is distinct from a malformed required field. It must not become an
empty-string match or an invented value. An excluded Author term removes matching Question
authors; a Pool has no such Author match to exclude.

## Combining filters

Different filters combine as requirements. For established multi-value fields, any selected
value within a field may match; capabilities require every selected capability. Independent Bloom
dimensions both constrain results when supplied. Positive text terms and exclusions additionally
constrain the same objects.

Exact vocabulary selections use global vocabulary identities rather than guessing from similar
names. Classification selections must form a valid hierarchy. Subject requires a selected Discipline;
Topic requires Subject; Subtopic requires Topic. Explicit cross-Discipline Subject search removes
only the selected Discipline equality, keeping Subject and all other filters. Text-based Subject
or Topic terms add search conditions rather than replacing exact vocabulary selectors.

Owner and authorship filters use authenticated server identity and authorization. Arbitrary
Account IDs do not grant access or permission to change ownership.

## Counts and pagination

Filter before sorting and paging. Treat a matching Pool as one object. Its members are counted as
Question results only when the filters include them and they independently match.

Category counts answer how many Questions in no Pool, Questions in Pools, and Pools match the
domain filters before restricting by object kind, Questions in no Pool, or page. They help an Instructor
change those choices. Field counts describe the relevant whole matching set, never just loaded
rows. Label counts according to that scope; category counts and displayed-row counts are not
interchangeable. Bloom returns its complete six-plus-four values in guide order, including zeros.

The shared result count and cursor must describe the same normalized request. Changing a filter
invalidates continuation of the old query. This is not a requirement to retain historical search
snapshots or to keep database snapshots open across requests.

## Cross-kind examples

- A Numeric Native JSON Pool with Topic `Gene mapping` matches those three filters through its
  own fields. A member's separate `meiosis` Tag does not make the Pool match `tags:meiosis`.
- With an Author filter and Both selected, matching individual Questions can appear when member
  inclusion allows them; Pools do not acquire Authors to remain visible.
- A Bloom filter excludes unclassified objects from that filtered view without making them
  unpublished, invalid, or unavailable in the unfiltered Library.
