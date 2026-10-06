# BiologyProblems.org import

## Purpose

This specification defines the supported import sequence for BiologyProblems.org content. The
importer uses authenticated PLE operations and ordinary creation and publication checks.
Direct database writes and SQL fixture loading do not establish that supported import workflow.

**Imported content supplies source data and relationships. PLE assigns PLE identities.**

## Import sequence

1. Read the source content selected for the Course.
2. Select the canonical representation using
   [BIOLOGY_PROBLEMS_SOURCE_SPEC.md](BIOLOGY_PROBLEMS_SOURCE_SPEC.md).
3. Convert only when necessary. Reuse `qti-package-maker-rs` for supported QTI conversion; review
   the output before it becomes PLE-native JSON.
4. Use the authenticated creation and publication operations described in
   [../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md](../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md).
5. Create a Draft, supply source, assets, and required metadata, and publish through the common
   Question workflow. PLE returns the generated Question ID and exact Revision.
6. Keep the source-to-PLE mapping needed for Course assembly and attribution. Course assembly uses
   the returned references.
7. Create a Question Pool only when selected Questions meet Pool rules and a Course definition
   explicitly calls for one. Do not turn every source problem set into a Pool.
8. Call the reusable Blueprint Course API using the returned Published Question and Pool
   references. See [BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md](BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md).
9. Read the created objects through PLE APIs and compare their source, metadata, exact Question
   Revisions, Pool membership, and Course/Assessment order with the import records.

## Source mapping and omissions

Retain the source-to-PLE mapping and report excluded content, validation failures, and mappings
that would lose meaning or require guessing. Use the source inventory defined in
[BIOLOGY_PROBLEMS_SOURCE_SPEC.md](BIOLOGY_PROBLEMS_SOURCE_SPEC.md). This document does not define
network recovery or a new operation-status model.

After PLE launches, BiologyProblems.org and PLE are independent. Later source changes have no
automatic effect in PLE and require no synchronization feature.

## Verification

A later system test exercises the supported import operations, then reads the created Questions,
Pools, and Blueprint Course through PLE APIs. It verifies generated IDs, exact revisions,
metadata, source linkage, Assessment order, and at least one Assessment use of imported content.
Focused database fixtures remain valid for low-level database tests, but they do not substitute
for this end-to-end check. The test transport and harness belong to implementation work.
