# HG Profile fixes — 2026-10-04

## Implemented boundary

- `GET /api/instructor-profiles/{Account ID}` lets any active signed-in Account read one active
  Instructor's derived display name and current avatar choice. The closed response contains only
  `displayName` and `avatar`; it excludes email, affiliation, Account settings, Course data, and
  Student data.
- `POST /api/profile/avatar/profile-images/{Profile Image UUID}/delivery` now lets any active
  signed-in Account retrieve an active Instructor's current uploaded Profile image. Anonymous
  callers, inactive viewers, inactive Instructors, Student images, and retired images receive the
  concealed not-found response.
- PostgreSQL performs both viewer and target Account-state checks. The browser only validates and
  encodes the canonical Account ID. This applies ASVS 8.2.1, 8.2.2, and 8.3.1 at the trusted
  database boundary.

`read_instructor_profile` derives its display name through
`ple_private.instructor_display_name`, supplied by the Instructor-account setup work. It must run
after that function is present in the fresh-schema order.

## Consumer integration

- The signed-in Instructor Profile route, Question detail and Library author links, Question and
  Pool Star links, and Blueprint Course owner link now use a real canonical Account ID. The ID is
  carried from SQL through the Rust DTO and browser contract into `InstructorProfileLink`; it is
  route identity rather than ordinary displayed content.
- Question authorship remains immutable attribution. Each ordered author has a nullable account
  link: external or deactivated authors keep their display name and a null link instead of a
  fabricated identity. Blueprint ownership is Account-backed and therefore always uses its
  `owner_account_id` link.
- Star lists remain Instructor-only. The public Profile route remains available to every signed-in
  role through the existing access-checked narrow Profile response.

## Validation

- `node --import tsx --test tests/test_instructor_profile_decoder.mjs tests/test_profile_image_crop.mjs`: 6 passed.
- `cargo check -p server_core -p learning-data-access`: passed. Existing warnings are from the
  concurrent removal of the rejected library-discussion feature.
- `tests/e2e/e2e_live_demo_profile_avatar.sh` now checks Student visibility of an Instructor image
  and anonymous denial. It was not run because no Live Demo target was available in this task.

## Live Profile-image repair — 2026-10-05

The running Profile image path exposed three database/result-shape defects that source checks did
not exercise. The PostgreSQL functions return `object_record_id`; the Rust adapter had read
`object_id` for image preparation, deletion preparation, and finalization. The preparation
function also used `clock_timestamp()` for records whose `updated_at` defaults to the earlier
transaction timestamp, violating their `updated_at >= created_at` check. The adapter now aliases
the returned field deliberately, and every Profile-image work, delivery, and object-record insert
uses `transaction_timestamp()` for this invariant.

- The canonical SQL functions were hot-replaced only in the disposable Live Demo database for the
  runtime proof. A fresh isolated PostgreSQL 17 install of the full current schema also passed at
  `/private/tmp/hg_schema_fairness_final.log`.
- The canonical application-only rebuild completed at `https://localhost:8263/`; it recreated API,
  worker, and public-asset publisher while retaining the data services.
- The live oracle proved Instructor image upload and finalization, Student delivery of that
  Instructor image, anonymous delivery denial, a second Instructor upload, and exact retired-image
  cleanup before its non-HG Sysadmin-upload check. The Sysadmin upload still returned 404 even
  though its session and direct preparation call are authorized; that separate lifecycle seam is
  unresolved and is not claimed as passing evidence for Instructor Profile visibility.

## Consumer completion — 2026-10-04

- Added the signed-in `/instructors/:accountId` browser route. It accepts only a
  canonical Account ID, performs the existing access-checked Profile read, and renders the
  Instructor's display name and current provided avatar or uploaded Profile image. The Account ID
  is route identity only; it is not shown as ordinary list content.
- Question and Question Pool Star responses now carry each active Instructor's real canonical
  Account ID alongside the display name. PostgreSQL derives both fields from the same ordered active
  Star rows. The browser validates the ID, links the displayed name and avatar to that Profile,
  and does not invent an ID from a name. Star lists remain rendered only on Instructor surfaces.
- The public Profile route is available to every signed-in role. Its access boundary stays in
  `read_instructor_profile`; Students receive the same narrow display-name/avatar response as
  other signed-in Accounts.
- Added canonical Account-ID route parsing and Ribbon fixtures so route construction and
  breadcrumb coverage reject malformed account paths.
- Added the executable `instructor_public_profile` screenshot scenario. It reads the authenticated
  Instructor's own canonical ID from Profile settings, then captures the access-checked Instructor
  Profile route. The published 247-image corpus records it as `instructor_public_profile` at
  `instructor/public_profile.webp`; its manifest identifies the route as `instructorProfile`.

### Authorship and curation boundary

Question authorship is immutable attribution rather than a required PLE Account identity.
`author_names` and its ordered nullable `author_account_ids` travel together through the Question
Library and detail contracts. `InstructorProfileLink` is used only when the paired ID identifies an
active Instructor; external and deactivated authors remain visible as unlinked text. The current
work does not add a link from a Question Pool to its owner's Instructor Profile.

### Validation

- `node --import tsx --test tests/test_public_navigation.mjs tests/test_ribbon_contract.mjs tests/test_frontend_contract.mjs tests/test_instructor_profile_decoder.mjs tests/test_screenshot_corpus_definition.mjs`: 46 passed.
- `npx tsc --noEmit --pretty false`: passed.
- `cargo check -p learning-data-access`: passed (existing dead-code warnings).
- `cargo check -p server_core`: passed.
- `cargo test -p learning-data-access question_star --lib`: passed; this filter selected no unit tests.
- The final focused consumer lane ran 44 Node tests covering the public route, author links,
  nullable/external/deactivated authors, Star links, and Blueprint owner links; `npx tsc --noEmit
  --pretty false` and the focused `question_model` check also passed. These checks do not replace
  a Live Demo browser capture.

## Repeated list representations

Profile and image reads share only simultaneous requests for the same identity
within one API client. Settled successes and failures leave the request map;
this creates no retained search or Profile cache. A failed Profile read preserves
the displayed name and link with an unavailable-avatar fallback. Instructor
links open in a separate tab, including those in search and Star lists.
`tests/test_instructor_profile_requests.mjs` verifies request sharing and release
on both success and failure; TypeScript and lint checks pass.
