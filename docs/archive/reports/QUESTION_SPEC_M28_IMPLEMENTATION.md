# M28 Supported Import Status

M28 remains unaccepted. The synthetic `question_spec_acceptance.rs` CLI and its child were
withdrawn before runtime execution and have been removed. Two completed diagnostic wrappers were
also removed; their logs remain. A one-time readback through the existing import and ordinary
services now passed. This is scoped evidence, not full M28 acceptance.

The product import paths remain available through the ordinary Pilot and curriculum publishers.
`installation-data apply` publishes Pilot Questions and creates the Live Demo Blueprint; the
ordinary curriculum publisher remains available for bundled Genetics content.

The installation-data E2E provisions and replays the Live Demo, then checks its Course and Released
Assessment through authenticated APIs. Separately, the one-time readback in
`output_question_spec/m28_import_readback_reviewed_20261007.log` passed: eight generated Pilot
Question identities had exact owner and metadata readback and matching stored source bytes/checksums;
ordinary Pool and Blueprint fixed/Pool references and adopted Course Theme were read back; and the
installed Blueprint retained its four exact ordered Pilot Native Question Tuples. The helper also
checked the manifest source and payload hashes. These checks use existing import and ordinary
assembly paths; they do not test importer asset ingestion. Native image publication and delivery
are covered separately by the existing M16 authoring path. Reconcile that evidence with M28's asset
criterion during final acceptance; this readback alone does not establish full M28 acceptance.
