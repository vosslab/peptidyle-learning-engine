# M12 Assessment release validation handoff

Status: the assigned M12 source is implemented. A final fresh static SPEC review and a different fresh static QUALITY review passed with no confirmed defects. The current isolated database proof passed in `output_question_spec/m12_release_current_20261007.log` (runner exit 0), including saving a request for three when the Pool has two valid Questions, reporting insufficiency, blocking release, allowing release after metadata rollback, and preserving issued tuples after Pool removal. `output_question_spec/M11_POOL_REFERENCE_READY` records source readiness only; M03/M11 dependency acceptance, browser evidence, and integrated M12 acceptance remain pending.

## Implemented behavior

- `ple_api.validate_assessment_release` retains generic blockers and returns Pool-specific issues with Assessment position, Pool title and public ID, requested count, and a closed issue kind. The Policies page presents the details without exposing internal Assessment Entry UUIDs or answer content.
- Assessment Save accepts a positive requested selection count above current Pool membership. Release validation counts currently valid exact members independently for each Assessment entry and reports specific Pool problems. Oracle 08 covers an oversized request, successful Save, blocked release, and an ordinary metadata correction that saves and leaves the affected Pool invalid for release. Exact missing metadata is unavailable and excluded from the valid count. Existing selections remain fixed.
- Pool Save checks removal of a Question ID against each available Assessment Entry referencing the changed Pool, including references from separate Assessments. It permits additions, metadata-only saves, and pin changes that retain the same Question ID. Attempt start takes a shared Pool lock. Issued Attempts keep their selected Revision tuples and Pool Edit Number when a reusable Pool later changes.
- The lifecycle oracle checks tuple continuity and rejection of removing an issued member. Retiring a whole Pool entry excludes its earned and possible points across Attempts; a separate fixed Question entry remains in scoring. Existing M17 current-score paths remain authoritative.
- Standalone complete fixed-Question removal remains tentative and unchanged.

## Focused verification

- `source ./source_me.sh && python3 schema_style/check_schema_style.py` - passed, clean.
- `source ./source_me.sh && cargo test -p learning-data-access --no-run` - passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` - passed.
- `source ./source_me.sh && node --import tsx --test tests/test_live_assignment_release_validation.mjs` - passed, 10/10.
- Focused whitespace checks passed. All M12 files remain below 1000 lines.
- SQLFluff parsed the changed function, grant, lifecycle, fairness, and driver paths; it returned parser output at `SAVEPOINT m12_missing_metadata` in oracle 08. This is parser output, not connected acceptance or a confirmed SQL defect.

## Root-owned runtime acceptance

The one-time current M12 proof uses `source ./source_me.sh && bash tests/_temp/e2e_assessment_saved_response_m12_proof.sh`; it installs a disposable PostgreSQL schema and runs the prerequisite fixtures plus `08_m12_release_validation.sql`. The canonical `source ./source_me.sh && ./tests/e2e/e2e_assessment_saved_response.sh` remains a broader root-owned run. An earlier canonical run stopped in oracle 01 on an ambiguous variable in `public_ids.sql`; the function now uses `new_public_id`, but that historical canonical failure is distinct from the current focused M12 pass.

Browser and integrated acceptance remain pending. The current isolated database pass establishes the stated M12 SQL behavior only; it does not establish full M12 acceptance.
