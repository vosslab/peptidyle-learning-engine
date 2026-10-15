# Genetics example Blueprint

This directory supplies bundled Genetics PGML and attribution to the shared
[BPW importer](../../launchers/import_bpw.py). The importer combines these
sources with the complete BPW Genetics collection, keeping direct PG/PGML source
unchanged. `manifest.yaml` supplies source metadata; its former miniature
curriculum publication policy is no longer an installation path.

The upstream Biology Problems snapshot is identified by the `source_revision` SHA-256 tree
fingerprint in `manifest.yaml`. Question content is CC BY 4.0; see `LICENSE.CC_BY_4_0` and
`ATTRIBUTION.md`. Canonical source code is LGPL-3.0-or-later; see `LICENSE.LGPL_v3`.

`import_inventory.yaml` is explicitly nonpublished. It preserves the prior mixed inventory:
42 canonical-source mappings, 119 static banks, and 20,579 static-row records. Chargaff and
the BIOL 301 RNA/DNA variant are excluded from the Fall 2026 pilot. The 76 unresolved banks and
13,434 rows are retained there as import evidence, not as active install input. The inventory
also records the 42 already-replaced families.
