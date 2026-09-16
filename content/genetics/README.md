# Genetics example Blueprint

This directory ships the reusable `Genetics` / `Fall Genetics` example Blueprint.
`manifest.yaml` is the active portable curriculum-content input. It defines 42 accepted
canonical PGML sources (41 from biology-problems-website plus HLA) as direct Fixed entries
across nine nonempty Genetics topics. It contains no static banks or migration-only
replacement mappings.

The upstream Biology Problems snapshot is identified by the `source_revision` SHA-256 tree
fingerprint in `manifest.yaml`. Question content is CC BY 4.0; see `LICENSE.CC_BY_4_0` and
`ATTRIBUTION.md`. Canonical source code is LGPL-3.0-or-later; see `LICENSE.LGPL_v3`.

`import_inventory.yaml` is explicitly nonpublished. It preserves the prior mixed inventory:
all 43 canonical-source mappings, 119 static banks, and 20,579 static-row records. Chargaff
is unaccepted. The 76 unresolved banks and 13,434 rows are retained there as import evidence,
not as active install input. The inventory also records the 43 already-replaced families.
