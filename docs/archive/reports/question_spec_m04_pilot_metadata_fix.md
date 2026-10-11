# M04 Pilot manifest metadata correction

The Pilot manifest producer now accepts omitted Question language as `None` and accepts empty
Tag collections. Supplied language still receives the existing trim and length validation. The
source-project content license remains required and maps to Question record metadata; supplied
Tags, license, citation, and language map through the publication plan into the ordinary Draft
creation metadata. Citation remains explicitly optional.

The Pilot publication caller already copies `PublicationSource.metadata` into
`CreateAuthoringDraftInput.metadata`, so no caller change was needed. Native payload content
validation remains in place; only the manifest-specific nonempty-Tags requirement was removed.

## Focused checks

- `source ./source_me.sh && cargo test -p project-tools pilot_content` - passed: 6 matching tests.
- `source ./source_me.sh && cargo tools pilot-content` - passed: 2 chapters, 8 reviewed Questions,
  and 4 adapted PGML sources.
- `rustfmt --edition 2024 --check crates/project-tools/src/pilot_content.rs crates/project-tools/src/pilot_content/tests.rs` - passed.
- `git diff --check -- crates/project-tools/src/pilot_content.rs crates/project-tools/src/pilot_content/tests.rs` - passed.

The tests cover an omitted manifest language with empty Tags, the resulting nullable metadata,
and preservation of supplied Tags, project license, citation, and language in the metadata mapping.
This is producer-level evidence only; it does not claim M04 milestone acceptance.
