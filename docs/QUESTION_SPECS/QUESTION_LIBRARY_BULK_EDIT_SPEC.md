# Question Library bulk edit specification

> **Deferred:** Instructor bulk editing remains deferred as a delivery requirement. The existing
> implementation remains available while the product decision is deferred. Mixed-save behavior
> remains an open question for a future product decision.

Bulk editing lets Instructors clean up the shared metadata of many Library Objects, especially
after large imports. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-library-specifications).
This document owns the workflow, unresolved mixed-save behavior, and current implementation
evidence. Existing endpoints do not make the feature a delivery requirement or settle its open
product choices.

## Selection and changes

Select explicit Library Objects from combined search and review the intended changes before
saving. Make the selection scope clear: selected rows are not an implicit instruction to edit
every result on unseen pages. Keep Questions and Pools identifiable while using common fields
with common meanings.

| Field group | Bulk-edit meaning |
| --- | --- |
| Discipline and Subject | Required classification; validate hierarchy and affected Pool constraints |
| Topic and Subtopic | Optional classification; a clear operation removes the optional value |
| Tags | Replace or change labels only as the selected API operation states; an empty collection clears Tags |
| Other search metadata | Only explicitly supported fields with defined validation and authorization |
| Source, answer key, Question Revision | Separate content workflow, not a search-metadata patch |
| Pool members and license | Change the Questions in the Pool through ordinary Pool editing; calculate its license automatically |
| Bloom | Ordinary metadata editing on the Question or Pool record |

An omitted patch field means unchanged. Clearing an optional value is a deliberate action,
distinct from leaving its control untouched. Required fields cannot be cleared to NULL. Pool
common Type and Backend come from its Questions and do not become editable labels merely
because the interface is spreadsheet-style.

## Applying an edit

The server validates the actor, each selected object, allowed fields, ordinary expected Edit
Numbers, and resulting classification. A Question metadata edit does not create a Question
Revision. Pool metadata editing does not replace Question members or rewrite their metadata.
Classification changes can expose Pool mismatch; apply
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).

Show whether an operation saved, was unchanged, or failed. On stale metadata, preserve the
proposed edits, refresh affected current values, and let the Instructor choose a later Save.
Do not report success for unsubmitted objects or automatically overwrite another edit.

## Unresolved mixed-save behavior

HG records bulk editing of shared Library metadata as desired but currently deferred. It does not choose whether a mixed-kind
Save is one all-or-none operation or several clearly reported operations. Existing Question and Pool
commands have different field sets. Their separate transaction guarantees do not imply a single all-or-none
request for both kinds. Keep that question explicit in
[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md).

For example, changing Tags on a selected Question and Pool affects their own Tags, not the Pool's
members. If one write fails, the interface must accurately report what committed. Whether both
must roll back is a product/API decision still to settle, not permission to claim both succeeded.

The workflow uses existing Instructor authority. It does not create curator roles, content
approval queues, or unrestricted owner-content editing.

## Current implementation evidence

The API has separate Published Question and Pool commands for a bounded selection. The request
fields, bounds, transaction behavior, and errors below describe the current API. Their separate
transaction behavior does not settle a mixed-kind Save. The commands exclude source, Answer Keys,
Revisions, Pool members, availability, ownership, Backend, Question Type, arbitrary JSON, and
Student Work. These endpoint limits are not new restrictions on ordinary metadata editing.

Ordinary Question and Pool record concurrency remains with their metadata editors.

### Published Question command

The route is `POST /api/questions/bulk-metadata`. The IDs below are schematic; callers
used canonical public IDs returned by PLE rather than generating these example strings.

```json
{"selection":[{"questionId":"AAAA-ZBBB","metadataEditNumber":7}],"patch":{"tags":["genetics"],"disciplineUuid":"5d9a2c29-3ff2-4f1c-8c7b-69a5e6c60d9a","subjectUuid":"da849b4f-1e89-45ba-94b7-fdce6bcb2ceb","topicUuid":null,"subtopicUuid":null}}
```

Selection is nonempty, has distinct canonical IDs, and has at most 1,000 items. Each item
has an expected edit number. Omitted fields stay unchanged; `null` clears only nullable fields.
Invalid hierarchy or tags reject the command. Success returns:

```json
{"results":[{"questionId":"AAAA-ZBBB","metadataEditNumber":8}]}
```

The server checks Instructor authority and every token, applies a valid command atomically, advances
tokens, and returns results in the normalized selection order. Current browser code canonicalizes
Question IDs before sending and checks that returned IDs are the same ordered set.

### Question Pool command

The `POST /api/question-pools/bulk-search-metadata` route uses the separate Pool metadata-edit
token:

```json
{"selection":[{"questionPoolId":"CCCC-ZDDD","questionPoolMetadataEditNumber":4}],"patch":{"tags":["inheritance"],"topicUuid":null,"subtopicUuid":null}}
```

Results contain `questionPoolId` and next `questionPoolMetadataEditNumber`. Pool Discipline and
Subject come from members and cannot change here. The server checks authority, tokens, Pool
constraints, and metadata before committing.

### Errors and transaction behavior

The current handlers conceal an invalid or missing session and a non-Instructor as `404`.
They return `415` for the wrong media type, `413` for an oversized body, `422` for a malformed
request, empty or duplicate selection, forbidden fields, bad IDs, or invalid values, `412` for a
stale token or retryable transaction, and `409` for an otherwise conflicting lifecycle. Concealed
or unauthorized selected objects also return `404`. A failed atomic command leaves all selected
objects unchanged. A future mixed command remains deferred with Instructor bulk editing; see
[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md).
