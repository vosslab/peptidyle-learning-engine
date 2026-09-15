# Genetics example Blueprint

This directory ships the reusable `Genetics` / `Fall Genetics` example Blueprint.
It preserves 11 ordered Genetics topics, 119 static source banks, and 20,579
generated static WeBWorK PG staging wrappers. `manifest.yaml` is the portable
curriculum-content import input.

The upstream Biology Problems snapshot is identified by the `source_revision` SHA-256
tree fingerprint in `manifest.yaml`. It covers the 119 source banks and 11 title files.
It is a content fingerprint rather than a Git commit because the reviewed snapshot is
not itself a Git checkout. The adapted compiled PG sources preserve the source-row
variations; not every upstream source bank carries stable row identifiers.

Question content and source banks are CC BY 4.0; see `LICENSE.CC_BY_4_0` and
`ATTRIBUTION.md`. The generated WeBWorK PG wrapper code is LGPL-3.0-or-later; see
`LICENSE.LGPL_v3`.

`manifest.yaml` currently declares 13 algorithmic author-source definitions,
including HLA. A family author source may be an official PG/PGML file or a
generator; C824--C841 must record its provenance, hash, and licenses and then
retain or construct one canonical algorithmic PG/PGML file. Those entries
are migration inputs, not runtime proof or a completed migration. The 119
static banks remain shipped until each family independently passes that forward
transition. Algorithmic Questions ordinarily stand alone, though a Pool may
intentionally select among distinct similar BiologyProblems.org WeBWorK
Questions. Immutable Question/Pool Revisions, Blueprint pins,
and Student Work remain preserved during retirement.
