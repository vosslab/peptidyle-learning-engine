# Shared search Pool license review

Reviewed October 4, 2026 as the independent M9 entry check in
[the shared-search plan](../read-docs-active-plans-active-shared-sea-peaceful-mountain.md).
This is a product-policy and implementation review, not legal advice.

## Decision

The proposed table is acceptable as PLE's deliberately restrictive **Pool collection
policy**, with one required clarification: a calculated Pool license identifies the
Pool's own selection, arrangement, and PLE-created Pool material. It does not relicense
or replace any member Question's license.

| Member licenses present | Calculated Pool license | Review result |
| --- | --- | --- |
| CC0 only | CC0-1.0 | Acceptable if PLE/Pool creator can dedicate the Pool's own compilation rights. |
| CC BY, with or without CC0 | CC-BY-4.0 | Acceptable policy choice. The CC BY member remains CC BY. |
| CC BY-SA, with any CC0 or CC BY | CC-BY-SA-4.0 | Acceptable policy choice. This is not ShareAlike legally imposed on a mere collection. |

CC says all CC licenses permit inclusion in a collection and that a collection license
does not change the license on its original works. The proposed CC BY-SA result is thus
a PLE rule that makes the Pool's own contribution reusable under BY-SA; it must not be
described as converting CC0 or CC BY members to BY-SA. CC's ShareAlike conditions apply
when PLE shares **Adapted Material**, not merely because it assembles separate unchanged
works in a collection.

## Authority alignment

- [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) requires one Pool license compatible with
  every member and rejects a `Mixed` Pool license.
- [QUESTION_MODEL.md](../../QUESTION_MODEL.md) requires a calculated compatible Pool
  license, rejects incompatible combinations, and preserves each member's original license.
- The [interview decision record](../decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md)
  settles automatic calculation, the current CC0/CC BY/CC BY-SA scope, the CC BY plus
  CC BY-SA result, and that individual Question licenses remain intact. NC and ND are
  deferred.

No authority conflicts with the table. The plan's rows follow the settled PLE policy.

## M9 implementation requirements

M9 is ready to begin with these requirements in its SQL/API/view tests:

1. Calculate and persist the Pool value only from the current members using the three
   approved rows; reject values outside the table while NC and ND are deferred.
2. Keep every member's license and attribution/source data available and authoritative in
   Pool detail, export, and reuse paths. A Pool-level label must not be the only licensing
   information exposed for a member.
3. Label/document the Pool value as the calculated Pool/collection license. Do not claim
   that it grants rights in every member beyond that member's actual license.
4. Test CC0-only, CC0+BY, BY-only, CC0+BY-SA, BY+BY-SA, and all-three membership; assert
   the calculated value and that the member values do not change. Also test calculation at
   creation, membership edits, and every fork path.

The planned test explicitly covering CC BY plus CC BY-SA is necessary but not sufficient;
the retention and presentation of the original member license is the boundary that avoids
an accidental relicensing design.

## Creative Commons primary sources

- [Creative Commons FAQ: collections](https://creativecommons.org/faq/#if-i-create-a-collection-that-includes-a-work-offered-under-a-cc-license-which-licenses-may-i-choose-for-the-collection): all CC licenses allow collection inclusion; a collection license does not change the license on original material.
- [Creative Commons FAQ: adaptations and ShareAlike](https://creativecommons.org/faq/#if-i-derive-or-adapt-material-offered-under-a-creative-commons-license-which-cc-license-s-can-i-use): distinguishes adaptations from collections and explains ShareAlike for adaptations.
- [CC BY-SA 4.0 legal code](https://creativecommons.org/licenses/by-sa/4.0/legalcode.en): defines Adapted Material and applies ShareAlike when sharing Adapted Material.
- [Creative Commons compatible licenses](https://creativecommons.org/compatible-licenses/): identifies licenses permitted for contributions to BY-SA adaptations.

## Readiness

**Ready for M9.** There is no serious conflict in the current three-license table. The
implementation must enforce and present it as a Pool collection policy while preserving
the individual Questions' separate licenses; treating the Pool label as a relicensing
operation would be a serious design error.

## M9 source-backed implementation handoff

### Canonical source and stored result

`ple_data.question_pool_member` already pins both `published_question_id` and
`question_revision_number`; its foreign key names the exact `question_revision`.
The canonical member license is therefore
`ple_data.question_revision_license.spdx_expression` joined on **both** columns. That
row is required by published-Question stewardship and immutable after creation. Pool
calculation must never use current Question metadata or the Library's latest-revision
projection: either can describe a later Revision than the one the Pool pins.

Store the required calculated value on `ple_data.question_pool` using the existing
`ple_data.license_spdx` domain. The calculation must read all current member pins and
their exact Revision licenses under the same Pool write lock. An empty set is invalid
already, so no empty-Pool license rule is needed.

### Required SQL mutation coverage

| Current write boundary | M9 responsibility |
| --- | --- |
| `ple_data.create_question_pool` | Calculate from the supplied exact member IDs and Revision numbers before/with Pool insertion; insert the non-null calculated value. |
| `ple_data.save_question_pool_members` | Recalculate from the proposed complete replacement before it becomes current; reject a combination outside the approved table and update the Pool value atomically with the member replacement. |
| `ple_data.construct_question_pool_fork` | Copy members and calculate from the copied exact pins in the child transaction. Do not copy a parent label without recomputing. |
| `ple_data.fork_question_pool` | Its ordinary Assessment import wrapper reaches `construct_question_pool_fork`; covered by that primitive. |
| `ple_data.fork_question_pool_for_course_adoption` | Its Course-adoption wrapper reaches the same primitive; covered by that primitive. |
| `ple_data.append_assessment_question_pool_fork_members` | It delegates to `save_question_pool_members`; verify the child Pool's calculated value changes through that path. |

The schema must make the calculated Pool column `NOT NULL`, limit it to the three
current SPDX expressions, and ensure the three mutations above cannot leave a stale
value. The existing unique `(question_pool_id, published_question_id)` member constraint
and exact-revision foreign key stay in place. Forks retain `owner_account_id` and
`source_question_pool_id`; those are Pool lineage facts, not provenance-table fields.

### Manual provenance removal

M9 removes the whole manual `ple_data.question_pool_provenance` table, rather than
keeping source text or URL beside the calculated license. Remove its table comments,
RLS policies in `60_policies/question_pools.sql`, grants in
`70_grants/question_pools.sql`, and both data/API functions
`save_question_pool_provenance` and `read_question_pool_provenance`. Remove the
associated receipt in `tests/e2e/assessment_saved_response/03_course_pool_forks.sql`.
Regenerate schema documentation and generated API types after the contract changes.

This removal does **not** remove the Pool owner/source-Pool contract required by HG
1288 and the schematic, nor does it alter member Question authorship, citation, source
binding, ownership, or exact Revision license. `question_revision_license` and the
other per-Revision credit tables remain the member-credit authority.

### Tests that prove M9

Use a PostgreSQL store test with exact Revision pins to cover every table row and all
supported mixtures:

| Exact member-license set | Expected Pool license |
| --- | --- |
| CC0 | CC0-1.0 |
| CC BY | CC-BY-4.0 |
| CC0 + CC BY | CC-BY-4.0 |
| CC BY-SA | CC-BY-SA-4.0 |
| CC0 + CC BY-SA | CC-BY-SA-4.0 |
| CC BY + CC BY-SA | CC-BY-SA-4.0 |
| CC0 + CC BY + CC BY-SA | CC-BY-SA-4.0 |

For each result, assert the individual `question_revision_license` rows are unchanged.
Exercise creation, a member-list replacement that changes the result in both directions,
ordinary Assessment import, Course-adoption fork, and Assessment-fork member replacement.
Also assert that manual provenance reads/writes no longer exist and that the Pool API and
generated types expose only the calculated Pool license. A fixture with two Revisions of
one Question should use different license values to prove the calculation reads the
member's pinned Revision, never a current/latest Question projection.

## Final M9 implementation review

The independent source re-review on October 5, 2026 is clean. Exact-pin calculation protects
creation, complete member replacement, and forks; member license and authorship use the existing
Question projection in Pool views. Manual Pool provenance is removed. Review found missing PUBLIC
revokes on the two internal calculation helpers; explicit revokes and an application-role denial
assertion now close that finding.

Manager acceptance passes the seven-combination SQL oracle, same-Question/different-Revision
license case, BY-to-CC0 replacement, fork paths, connected Course adoption/co-Instructor lifecycle,
full Rust including Wasm, schema checks, and 544 Node tests. Receipts are
`/tmp/ple_shared_search_m9_{sql,connected,rust,schema,code}.log`.
