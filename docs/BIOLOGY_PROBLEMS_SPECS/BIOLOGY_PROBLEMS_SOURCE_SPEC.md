# BiologyProblems.org source content

## Purpose and scope

This specification maps BiologyProblems.org source material to PLE Published Questions. It covers
source selection, provenance, and conversion choices. It does not set the Question lifecycle,
Question Pool rules, or Blueprint Course structure; those rules belong to the linked PLE
specifications.

The checked source mirror is `OTHER_REPOS/biology-problems-website/site_docs/`. Keep enough source
information to preserve attribution and connect selected content to the identities PLE returns.
A source path identifies source material. It is not a PLE Question ID.

## Source selection

- Use the source selected by the Course specification. A full website inventory does not itself
  select content for PLE.
- Exclude a Question explicitly labelled `BIOL 301` from the Fall 2026 pilot.
- Preserve the best original source. When parameterized WeBWorK source exists, preserve it rather
  than importing generated static examples.
- Preserve the distinction between PG and PGML. Label source PGML only when it is fully
  PGML-compliant; otherwise label it PG.
- One parameterized PG or PGML source becomes one Published Question, even when it creates many
  student-specific variants.
- A suitable static source becomes one PLE-native JSON Published Question. QTI can assist a
  conversion, but native JSON is the PLE representation after conversion.

## Source-to-PLE mapping

Course assembly uses the Question IDs and exact Revisions returned by PLE for the selected source
content. Preserve source attribution and report omitted content or failed conversions so the
result can be checked against the intended Course. These requirements do not prescribe a mapping
file schema, status vocabulary, or import tracking service.

After launch, BiologyProblems.org and PLE are independent; PLE does not track later source changes.

## Source representations

| Source material | PLE destination | Requirement |
| --- | --- | --- |
| Parameterized `.pg` | WeBWorK Published Question | Preserve PG bytes and backend behavior. |
| Fully compliant `.pgml` | WeBWorK Published Question | Preserve PGML bytes and backend behavior. |
| Reviewed static Question | Native JSON Published Question | Validate the native JSON source and publish it normally. |
| QTI package for a static Question | Native JSON after reviewed mapping | Use QTI only as an import path; record unsupported or lost information. |
| Generated static variant of an algorithmic source | No separate Published Question | Keep the algorithmic source as the one Published Question. |

Reuse the Rust `qti-package-maker-rs` project for its existing QTI models, readers, writers, and
package checks when it supports the needed conversion. PLE does not reimplement that work. Its
future Rust/WASM direction is not a present PLE dependency. Check reader support for an input;
writer support alone does not prove that PLE can import that format.

## Attribution, licenses, and assets

The importer preserves available source authorship, license information, source attribution, and
required image or other asset references. It supplies the normal PLE publication metadata and
does not bypass publication validation. A conversion that cannot preserve needed attribution,
license, or required content must be reported rather than silently published.

The source can contain duplicate exports of one Question in different packaging forms. The
canonical source chosen above determines the import. A Blackboard or Canvas export, human-readable
preview, and self-test page are evidence for review, not distinct Questions.

## Open rules

- The conversion coverage offered by the Rust QTI project must be checked for each requested
  source format before it becomes a supported path.
