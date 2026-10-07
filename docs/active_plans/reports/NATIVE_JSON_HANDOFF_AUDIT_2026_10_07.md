# Native JSON handoff audit

## Scope

This focused audit covers the recent Native JSON handoff in the import and QTI specifications,
TODO, README, implementation ledger, and the referenced tuple, binder, publication, renderer, and
external writer evidence. It is not a whole-repository audit or runtime acceptance review. Six
independent passes were completed before these bounded documentation corrections.

## Findings and dispositions

| Severity | Finding | Evidence and disposition |
| --- | --- | --- |
| Medium | The import specification said M15/M16 were active and M29 runtime acceptance was pending. | The accepted ledger closes M01-M29. Replaced the stale status with a ledger link and stated that later converter and HTML/image integration remains open. See [QUESTION_IMPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md). |
| Low | The QTI specification implied that `externalResources` would be connected to fetching or rendering. | The inventory is source metadata. Removed the implied future behavior and kept supplied-file resolution tied to paths in imported content and packages. See [QTI_INTERCHANGE_SPEC.md](../../QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md). |
| Proposed, rejected | Remove ZIP preference from the transport wording. | The manager rejected this proposal because the user prefers avoiding ZIP files while asking how storage and data handoff should work. Transport remains unspecified; no new remote-fetch policy was added. |

## Independent passes

| Pass | Result |
| --- | --- |
| Plan | No findings. |
| Test | No findings; no permanent test additions warranted. |
| Style | Found the stale M15/M16/M29 status; corrected above. |
| Docs | Found the same stale status; corrected above. |
| Legacy | Found the low-severity `externalResources` implication; corrected above. |
| Comment | No findings. |

## Remaining limits and checks

HTML rendering, multiple inline-image publication, converter integration, and the HOTSPOT incoming
pre-bind shape remain implementation gaps. Sanitization detail remains deferred. These corrections
do not add implementation or runtime acceptance evidence. Prior link/ASCII check 2968 is supporting
history. For these edits, 32 selected Markdown-link checks passed, all five changed Markdown files
passed the ASCII/ISO-8859-1 checker, Human Guidance `--diff` and `--consistency` each passed at
1248/1248 bullets, and scoped `git diff --check` passed.
