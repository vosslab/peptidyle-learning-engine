# M03 current metadata editor browser regression

## Scope

This focused browser case mounts the real `QuestionMetadataEditor` with a deliberately stale
Question detail title and description, then supplies current metadata through the component's
current-read API. It checks that Owner and Sysadmin viewer projections can open and save the editor
using the current Title, Description, Tags, classification, and metadata Edit Number.

The harness controls API responses to isolate browser behavior. It demonstrates the component's
controlled API projection and does not establish connected SQL read or write authorization; the
exclusive fresh database gate remains responsible for those checks.

## Evidence

Passed: `source ./source_me.sh && node --import tsx
tests/playwright/question_metadata_editor_current_snapshot.mjs` ran before the broad snapshot proof
moved out of the permanent browser lane. The one-time component evidence is retained in the ignored
`tests/_temp/` workspace. The script passed for Owner and Sysadmin viewer projections. The real editor
displayed the current Title, Description,
Tags, Discipline, Subject, and empty Topic/Subtopic values despite stale Title and Description in
the supplied detail props. Save submitted the current metadata Edit Number and current values.

The captured Save payload contains the exact fixture Question Revision Tuple (`ABCD-XEFG`, Revision
1), metadata Edit Number 42, current Title, Description, and Tags, plus the exact Discipline and
Subject UUIDs and null Topic/Subtopic UUIDs. `npx prettier --check` on the harness, runner, and report
passed. Scoped `git diff --check` passed.

Both Owner and Sysadmin labels use the same controlled API response,
`viewerMayEditMetadata: true`. They identify two named component scenarios; they do not distinguish
role authorization or establish connected API/SQL behavior.

## Remaining acceptance

Run the fresh specification and quality review after this report is complete. Connected current-read
and ordinary-save authorization for Owner, Sysadmin, and denied roles still requires the fresh
database gate. M29 real-backend browser acceptance remains a separate requirement.
