# Course Appearance banner storage and sizing

## Status

Accepted and implemented for M6 and M8. This decision closed M3 in
[the archived Course Appearance plan](../../archive/cryptic_foraging_hennessy.md).

## Decision

A Course Banner retains one verified, immutable private source object and has
two immutable, normalized WebP delivery renditions:

- the course-entry hero is a 6:1 wide crop, with a 1200 by 200 CSS-pixel
  reference box; and
- the course-card preview is a 5:2 crop.

The server derives both centered crops from the source. The Appearance page
renders both from the selected unsaved file before save and from their saved
delivery URLs after save. There is no automatic focal-point detection or crop
editor in this capability. An Instructor judges the two center crops in the
page before promoting the upload.

The verified source remains in its decoded original PNG, JPEG, or WebP form.
Promotion produces a 1200 by 200 WebP hero and a 1000 by 400 WebP card
rendition, each with a positive byte length no greater than 2 MiB. This is
required by the existing browser delivery contract in
[`response.ts`](../../../src/api/http_client/response.ts), which already
accepts only `image/webp` course-banner responses with that bound. The image
crate already enables `webp` and its existing validation tests use
`WebPEncoder::new_lossless`, so normalizing both derivatives needs no new
codec dependency.

## Measured sizing evidence

The canonical screenshot profiles are [defined in the capture
manifest](../../screenshots/current_capture_manifest.json): laptop 1280 by
800, tablet 800 by 1280, phone 393 by 852, and square 800 by 800. The current
shell has a 16-pixel gutter and the reading page is capped at 72rem, so the
measured maximum hero widths are 1152, 768, 361, and 768 CSS pixels. The
current stale 1200 by 328 rule is approximately 3.66:1 and is not retained.

For each candidate, the table reports the resulting hero block size before the
phone floor. The selected rule is `aspect-ratio: 6 / 1` with
`block-size: clamp(120px, 100cqi / 6, 200px)` (or the equivalent width-based
fallback until container queries are appropriate). The floor prevents a
60-pixel phone strip; the cap avoids consuming more than 200 pixels above the
fold.

![Rendered candidate crops at every canonical profile. The selected 6:1 row
is framed in blue.](assets/course_appearance_banner_ratio_specimen.svg)

The committed rendered specimen uses the actual calculated page widths at
one-half scale, matching candidate heights, and identical center-marked
artwork in every crop. It confirms that the center region survives all four
candidates, but shows 7:1 as a strip at tablet and square widths and 5.5:1
crossing the laptop height budget. The selected 6:1 row is the shortest
identity-preserving result once the phone floor is applied.

| Candidate            | Laptop (1152px) | Tablet (768px) |           Phone (361px) | Square (768px) | Result                                                                           |
| -------------------- | --------------: | -------------: | ----------------------: | -------------: | -------------------------------------------------------------------------------- |
| 1200 by 328 / 3.66:1 |           315px |          210px |                    99px |          210px | Too tall on the three wider profiles; phone remains too short.                   |
| 5.5:1                |           209px |          140px |                    66px |          140px | Near the reference service, but exceeds the 200px above-fold budget.             |
| **6:1, selected**    |       **192px** |      **128px** | **60px -> 120px floor** |      **128px** | Shortest candidate retaining a distinct course-identity region at every profile. |
| 7:1                  |           165px |          110px |                    52px |          110px | Tablet and square become a decorative strip, not course identity.                |

The card rendition intentionally remains taller: at a representative 320px
card width, 5:2 provides 128px for the same centered region. The two previews
make the tradeoff visible: a source whose meaningful material is not near the
center is unsuitable until the Instructor supplies a better crop upstream.

## Evidence in the current repository

- [`ObjectAddress::CourseBannerUpload`](../../../crates/objects/src/bucket.rs)
  is a typed, non-signable, temporary address, and
  `ObjectAddress::CourseBanner` is the existing typed private-content saved
  address. There is no raw-path form. M6 extends this semantic address family
  with server-derived source, hero, and card identities; it does not add a
  raw-string key or reuse one rendition's address for another.
- [`ObjectStore`](../../../crates/objects/src/lib.rs) has immutable `put`,
  verified `get`, and exact-address `delete`; it has neither a transaction nor
  an image-rendition operation.
- [`verify_still_image`](../../../crates/objects/src/image_validation.rs)
  already decodes a full still image, detects canonical media type, applies
  EXIF orientation to measured dimensions, rejects animation and trailing
  container bytes, caps input at 8 MiB and decoded pixels at 20 million.
- [`ple_data.course_banner` and
  `ple_data.course_banner_delivery`](../../../schemas/migrations/2026082928_object_delivery_storage_checks_and_cleanup.sql)
  currently model only one object per banner and therefore cannot represent
  both generated delivery objects. The migration also provides the
  object-storage check and cleanup-manifest lineage that repair work must join
  rather than bypass.
- [`CourseEntryIdentity`](../../../src/features/course_appearance/course_entry_identity.tsx)
  is the only current banner presentation and still carries the rejected
  1200 by 328 assumption. There is no course-card banner or server-side resize
  pipeline today.
- [`fetchCourseBanner`](../../../src/api/http_client/response.ts) requires a
  same-origin, no-store, `nosniff` WebP attachment with positive
  `Content-Length` at most 2 MiB. Storing original PNG/JPEG bytes as a delivery
  object would contradict that closed browser contract.

These facts require physical source-plus-derivative storage for this
milestone: the source preserves future re-crop evidence, and the two bounded
WebP objects satisfy the established browser delivery contract. The existing
object, delivery, and cleanup primitives remain the owner; M6 adds only the
typed rendition identities and exact per-rendition ownership rows they need.

## Schema and integrity shape for M6

M6 must make the existing delivery tables represent the two delivery objects;
it must not claim that the current one-object foreign key already does so.
The forward migration has this exact shape:

1. Keep `ple_data.course_banner(course_id, course_banner_id, object_id)`, but
   define its `object_id` as the retained private **source** object.
2. Add `ple_data.course_banner_rendition` with columns `course_id`,
   `course_banner_id`, `rendition_kind`, and `object_id`; use primary key
   `(course_id, course_banner_id, rendition_kind)`, unique key
   `(course_id, course_banner_id, rendition_kind, object_id)`, and a closed
   check for exactly `hero` and `card`. Its composite foreign key references
   `course_banner(course_id, course_banner_id)`, so no rendition can outlive
   its exact source-owned banner.
3. Alter `ple_data.course_banner_delivery` to add `rendition_kind`, drop its
   current three-column foreign key to `course_banner`, and replace it with
   `(course_id, course_banner_id, rendition_kind, object_id)` referencing the
   matching `course_banner_rendition` row. Retain its existing
   `(delivery_id, object_id)` foreign key to `object_delivery` and its
   `delivery_id` primary key. Thus every available hero/card delivery is bound
   to one exact rendition object, while the existing
   `require_exact_available_object_delivery_owner` trigger still counts one
   `course_banner_delivery` owner and needs no semantic weakening.
4. The `2026090903` forward migration may require the unused pre-production
   `course_banner` and `course_banner_delivery` tables to be empty before it
   reshapes them. There are no users or application-created banner records to
   preserve, so M6 deliberately has no legacy backfill, compatibility
   rendition, or transitional card behavior.
5. New promotions create one source plus exactly both `hero` and `card`
   rendition rows before they become current. Database constraints and the
   promotion procedure reject an incomplete pair. The read model exposes one
   banner reference only; the fixed delivery route selects the already-known
   rendition kind, never a caller-supplied storage identity.

The new table and altered relation carry the same forced-RLS, revocation, and
definer-only mutation model as their existing course-banner counterparts. M6
updates the relevant RLS policies and the catalog acceptance oracle with the
new keys, check, and foreign key.

### Cleanup subject bridge

`ple_private.object_storage_check` currently has a required `delivery_id`, so
it can check hero/card rendition objects but cannot truthfully represent the
private source or a temporary upload. M6 extends that existing primitive,
rather than introducing a parallel cleanup system:

1. Add `ple_private.course_banner_storage_subject` for exact non-delivery
   objects. It contains the closed subject kind (`upload` or `source`), exact
   course, exactly one upload-or-banner reference as appropriate, `object_id`,
   expected checksum, and storage area. It has no raw path.
2. Alter `object_storage_check` so exactly one of `delivery_id` or nullable
   `course_banner_storage_subject_id` is present. Preserve the existing unique
   `delivery_id` behavior with a partial unique index and add an equivalent
   partial unique index for the subject. Existing rows retain their delivery
   anchor unchanged.
3. Reuse the existing `object_cleanup_manifest` and its job/receipt lineage
   through that extended check row. A source or upload deletion that cannot be
   confirmed therefore has the same durable check, authorized cleanup job, and
   audit receipt as a retired rendition; it is not an untracked best-effort
   delete.

The persisted promotion work record names the matching delivery or storage
subject for every target. This is the required repair bridge for temporary and
source objects, while rendition objects keep using their normal delivery ID.

## Ingest and storage contract

1. `POST /api/courses/{courseId}/appearance/banner-uploads` accepts one body
   containing image bytes. The route derives its storage address and all IDs
   server-side; a filename, client media type, object path, Course ID in the
   body, or Account ID in the body is never trusted.
2. The caller must have a current Instructor Course Membership for the route
   course. The persistence transition repeats that exact course-and-account
   authorization check.
3. M6 calls `verify_still_image` before any object write. The accepted source set is
   still PNG, JPEG, and WebP only, each at most 8 MiB and 20,000,000 decoded
   pixels. The decoded canonical media type, oriented dimensions, checksum,
   and byte length are persisted; declared media type and filename are not
   authoritative.
4. The server generates a `CourseBannerUploadReference`, writes bytes only to
   `ObjectAddress::CourseBannerUpload { course, upload }`, and persists an
   upload record bound to that exact Course Instance, authenticated Account,
   expiry, and verified object facts. The returned receipt contains only the
   opaque upload reference.
5. A successful promotion retains the verified source at a server-derived
   private `ObjectAddress::CourseBannerSource { course, banner }` address,
   then decodes it again and writes exactly two server-derived immutable
   `ObjectAddress::CourseBannerRendition { course, banner, rendition }`
   targets. The closed server-owned rendition enum is `hero` (1200 by 200,
   6:1) and `card` (1000 by 400, 5:2). Both generated byte streams must be
   positive and at most 2 MiB; otherwise promotion fails before any banner
   becomes current.
6. The promotion creates the existing `course_banner`, `object_delivery`, and
   `course_banner_delivery` ownership records for each delivery rendition,
   makes that banner the course's current banner in one database transaction,
   and retires the former deliveries. Theme state remains separate on the
   Course Instance row.

The retained source is not public or signable. Each delivery route selects
only the persisted `hero` or `card` rendition for the current banner after the
requesting Account passes the same course-member read authorization used by
the appearance read model. It returns the pre-existing closed WebP attachment
contract, including `no-store`, `nosniff`, same-origin, positive length, and
the 2 MiB maximum.

## Promotion, compensation, and repair

`ObjectStore` and PostgreSQL have no shared transaction. M6 must therefore
implement a persisted saga, not claim atomic cross-store writes:

1. Before each external `put` or `delete`, persist an exact course-banner
   promotion/cleanup work record naming the typed source and target address,
   intended operation, relevant Course Banner reference, and state.
2. Write the immutable source and both immutable rendition targets, then perform the database transaction
   that rechecks Instructor Course Membership, verifies the Account-bound
   unexpired upload, records the exact delivery ownership, and switches the
   current banner.
3. If the database transaction cannot commit after a target write, the old
   current banner remains visible. The work record remains actionable and
   causes deletion of the uncommitted target; it must be linked into the
   existing storage-check/cleanup-manifest lifecycle when deletion needs a
   repair job.
4. Only after the database transaction commits may the temporary upload and
   retired former source and rendition objects be deleted. If any deletion cannot be confirmed,
   its persisted work record remains pending for repair. The object is never
   silently forgotten or delivered merely because it exists in storage.
5. Expired unpromoted uploads use the same persisted cleanup path. A failed
   upload registration likewise leaves a repair record before any best-effort
   deletion attempt.

This is the required compensation boundary: failure returns an error and
never advances the visible current-banner pointer, while every possible
unconfirmed external object has a durable cleanup/repair record. M6 must add
the narrowly scoped record and procedures needed to connect it to the existing
`object_storage_check` and `object_cleanup_manifest` primitives; it must not
invent a second generic object-cleanup subsystem.

## Mutation API

The banner operations are independent of the theme mutation.

| Operation      | Route and body                                                                                                                                               | Required result                                                                                                          |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Stage upload   | `POST /api/courses/{courseId}/appearance/banner-uploads` with raw image bytes                                                                                | Returns `CourseBannerUploadReceipt`; no current appearance changes.                                                      |
| Set or replace | `PUT /api/courses/{courseId}/appearance/banner` with `{ "upload": "...", "alternativeText": { "kind": "decorative" } }` or the validated informative variant | Promotes exactly that current Account-and-Course-bound unexpired upload, then returns the updated appearance read model. |
| Remove         | `DELETE /api/courses/{courseId}/appearance/banner`                                                                                                           | Clears only the current banner and returns the updated appearance read model.                                            |

Set and replace intentionally share one route: whether the course previously
had a banner is server state, not a client-controlled operation. A stale,
foreign, expired, or already-promoted upload is concealed and refused. There
is no `Keep` banner action, no combined theme-and-banner body, no revision,
and no ETag precondition.

The existing protected hero delivery stays
`POST /api/course-banners/{bannerReference}/delivery`; M6 adds the fixed card
delivery route `POST /api/course-banners/{bannerReference}/delivery/card`.
Neither route accepts a caller-selected storage key or free-form rendition
name. Both return the same closed WebP response contract. The hero route is
the established route used by the current browser client; the card route is
used only by course-card presentation and the M8 saved-state preview.

## UI and accessibility contract for M8

The page labels the two previews in words ("Course entry" and "Course card")
as well as showing their shapes; crop acceptability is never expressed by
color alone. The file picker is a native labeled control. Alternative text is
an explicit decorative/informative choice with a labeled informative-text
field; decorative is the default because adjacent course identity already
names the course, but an Instructor can describe meaningful artwork.

Course identifier text is outside the artwork, below the hero. Do not overlay
it and do not apply a scrim or gradient to instructor artwork. This preserves
art that already contains a course number, title, or term and avoids a
contrast obligation against arbitrary pixels. The image alt treatment remains
the explicit saved alternative-text choice, independent of its visual crop.

## Dispatch checklist

M6 implements the three mutation routes, two delivery-rendition routes, and
the persisted lifecycle above. It adds typed source/hero/card ownership rather
than trusting a caller path, and proves set, replace, remove, foreign-upload
refusal, malformed/animated image refusal, each WebP delivery header and byte
bound, and fault-injected compensation/repair persistence.

M8 renders the selected 6:1 course-entry and 5:2 course-card crops before
save, uses the matching saved WebP delivery URLs after save, applies the phone
floor and 200px cap, and keeps a pending file local until explicit banner
save. The page must not disturb a pending theme selection when a banner is
saved.

The only deferred matter is optional visual refinement of the 120--200px
height clamp after owner use. It is not an implementation, test, or closure
gate.
