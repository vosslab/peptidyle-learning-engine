# Question-spec connected test compile correction

Status: The M08 lineage and M03 revision-metadata PostgreSQL test target compiles. Connected
database execution remains pending the fresh database gate.

## Corrections

- Import `BlueprintModuleReplacementInput` and `ReplaceBlueprintCourseContentInput` from their
  `question_model` exports in the M08 lineage test.
- Clone the application pool when constructing the lineage store so the test can still close its
  pool after assertions.
- Clone the Question ID when constructing Revision 2 because later M03 assertions read it again.

These ownership and import corrections preserve the existing assertions and test behavior.

## Evidence

- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` - passed; the connected test binary compiled.
- `rustfmt --edition 2024` on the two owned test modules and `git diff --check` on the owned files - passed.
- The ignored connected PostgreSQL selectors have not run; this report does not claim runtime
  acceptance.
