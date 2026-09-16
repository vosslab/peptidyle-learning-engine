# Architecture And Implementation Changes

## Current heading reconciliation

Current status and retained evidence are recorded in [compliance_summary.md](compliance_summary.md)
and the [implementation checklist](../../audits/human_guidance_implementation_checklist.md).
Blueprint lifecycle/forks/comparison now belong to Course specifications (Part 08); Assessment
type appearance belongs to Instructor interface (Part 04). Earlier topical inventories are
historical context, not current wording, counts, source-line pointers, or ownership.

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to architecture and implementation. The bullet text is
copied verbatim from the generated checklist, and its source location is recorded beside it.
This report does not establish exhaustive or disjoint topical coverage; the checklist remains the
authority for each record's status.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Storage composition receipt

The object-storage cutover removes the direct `aws-config`/`aws.rs` fallback.
Server composition accepts only explicit `disposable-local` MinIO configuration;
the API/worker and publisher use separate configured credentials. The retained
S3-compatible SDK graph follows the existing latest-first dependency policy and
adds no dependency pin. This receipt changes no checklist status.

Focused offline evidence passed: a fresh root `cargo test -p server_core --lib`
ran 99 tests; `objects` with the `s3` feature ran 38 library tests, three
conformance tests, and one archive test. The isolated
`minio_object_store_conforms` run passed once against unique loopback MinIO.

Container-composition evidence is separate from that object-store conformance
result: the owned MinIO used a current locally available image with pull policy
`never`, a read-only root, a data tmpfs, and zero persistent volumes. Its health
check passed. Ordinary exact `stop --rm` cleanup removed the owned container and
temporary secrets; no owned container or temporary secret remained, and the
shared stack was untouched. This is not PLE HTTP, full-cloud, cloud IAM, or
production-deployment evidence.

## Topical inventory

### Development principles -- Codebase development rules

- PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior.
  - Source: `docs/HUMAN_GUIDANCE.md:46`

- PLE is pre-production with no users or durable production data. Improve the design directly.
  - Source: `docs/HUMAN_GUIDANCE.md:50`

- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
  - Source: `docs/HUMAN_GUIDANCE.md:45`

- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
  - Source: `docs/HUMAN_GUIDANCE.md:51`

- Adaptability should be a focus so the software can evolve as requirements and insights change.
  - Source: `docs/HUMAN_GUIDANCE.md:52`

- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
  - Source: `docs/HUMAN_GUIDANCE.md:53`

- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
  - Source: `docs/HUMAN_GUIDANCE.md:54`

- Do not create or leave placeholder database tables, states, APIs, workers, or compatibility scaffolding before the feature has an approved product design.
  - Source: `docs/HUMAN_GUIDANCE.md:55`

### Development principles -- PLE development rules

- The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
  - Source: `docs/HUMAN_GUIDANCE.md:65`
