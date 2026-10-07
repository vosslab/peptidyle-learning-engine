# Lifecycle test split

[`lifecycle.rs`](../crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs)
reached the 1,000-line source limit. Its existing PostgreSQL acceptance test
now delegates the head-lock race and Revision-construction sealing phase to
[`lifecycle_write_serialization.rs`](../crates/learning-data-access/tests/blueprint_course_postgres/lifecycle_write_serialization.rs).

The helper owns the existing fixture-pool and inspection-connection cleanup
before opening the direct connections used by the race. The moved block keeps
its SQL, ordering, and assertions. The original `#[tokio::test]`, `#[ignore]`,
function name, daughter-work assertion, checksum assertion, and full selector
remain in `lifecycle.rs`; no test-driver registration or selector edit was
needed.

The resulting files are 689 and 334 physical lines, respectively.
A path-specific [`.gitignore`](../.gitignore) exception makes this report visible to Git.

The focused checks passed:

```sh
rustfmt --edition 2024 --check \
  crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs \
  crates/learning-data-access/tests/blueprint_course_postgres/lifecycle_write_serialization.rs
source ./source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run
source ./source_me.sh && python3 -m pytest -q tests/test_source_file_line_limit.py
```

The compile-only target built successfully. The line-limit gate passed with
1,971 tests and 10 advisory warnings for other files near the limit.
Compilation also reported the existing unused `question_pool_member_pins`
helper warning in `support.rs`; the extracted module introduced no warning.

No connected PostgreSQL test was run. The lifecycle selector remains an ignored
acceptance test requiring the disposable PostgreSQL runtime.
