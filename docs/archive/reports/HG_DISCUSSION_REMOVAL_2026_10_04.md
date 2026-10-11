# General Library discussion removal

HG says PLE is not an online forum. The Question and Pool detail pages, browser client,
server routes, Rust models, PostgreSQL tables, functions, policies, grants, indexes, and Watch
event variant no longer contain improvement threads or posts. A fresh installation therefore
has no hidden forum persistence or API behind the removed interface.

Impact notices remain as their own narrow boundary. Their create, update, and cancel routes,
store operations, table, and Watch event remain. Question owners and Sysadmins retain the
current authority checks. Pool notice authorship is still unresolved: the retained current
behavior permits Sysadmin management, while HG does not yet choose a replacement Pool-notice
author. This report records that seam; it does not settle it.

Validation completed:

- `cargo check -p question_model` passed.
- `cargo check -p learning-data-access --features postgres -p server_core` passed.
- `node --import tsx --test tests/test_library_watch_notification_client.mjs` passed: 2 tests.
- `source ./source_me.sh && python3 devel/generate_schema_tables_doc.py && python3 schema_style/check_schema_style.py` passed.
- `source ./source_me.sh && python3 -m pytest -q tests/test_guidance_doc_format.py tests/test_markdown_links.py` passed: 389 tests.
- `git diff --check` passed, and a source search found no remaining thread or post implementation symbols.

The Rust checks compile the native library and server path; the schema and Watch checks exercise
the retained SQL and browser wire contract separately.

Fresh live Question Library acceptance also passes: the opened Question detail has
no Discussion button or link, and an authenticated request to the removed general
`/api/library-objects/question/{id}/discussions` route returns 404. The receipt is
`/private/tmp/hg_live_question_library_browser.log`. This is a narrow Question
detail/router proof, not a claim that every unrelated UI surface was re-audited.
