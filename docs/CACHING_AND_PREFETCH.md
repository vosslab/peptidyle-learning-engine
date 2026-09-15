# Caching

Caching reduces repeated safe work; it never creates Assessment truth,
authorization, timing, response, grading, or retention authority. Product
behavior comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

The former next-Question prefetch, reservation, promotion, `nextIssued`, and
`nextPending` design is retired. An Assessment Attempt fixes its Question set
and presents one Question at a time. Students navigate that set directly.

## Cache ownership

| Cache | May contain | Exact key | Must not contain or decide |
| --- | --- | --- | --- |
| Browser route memory | Current answer-free screen and unsaved local edit | Current authorized page/Attempt | Durable saved-work truth, submission, credit, or offline queue |
| Browser asset cache | Authorized immutable answer-free asset bytes | Delivery URL plus content checksum | Private source, Answer Keys, backend state, or Student records |
| Native render cache | Answer-free static presentation | Exact Question Revision and authored answer-choice order setting | Course/Student authority, responses, feedback, or grading inputs |
| Backend adapter cache | Only data the backend contract proves safe and reusable | Backend-owned immutable identity | Cross-Student session state, credentials, raw results, or PLE grading semantics |
| Retained Attempt data | Minimum evidence needed to interpret selected Questions and saved responses | Exact Student, Course, Assessment Attempt, position, and Revision/backend state | A browser-writable cache or generalized page/software snapshot archive |

Protected Assessment and Student API responses use `Cache-Control: no-store`.
Persistent browser storage does not hold session credentials, Student
responses, grades, private source, or backend state.

## Asset delivery

Published answer-free assets may use long-lived content-addressed caching.
Private authoring assets, Course banners, backend documents, and Student-record
objects require their own authorized delivery and cache policy. A signed URL or
cache hit does not grant wider Question Library or Course access.

## Native Questions

Native PLE Question JSON is static and receives no random seed. Its answer-free
presentation may be cached by exact Question Revision and safe authored
presentation settings. The server still verifies the authenticated Assessment
Attempt and position. The matching Answer Key or grading input
remains private and is never placed in the shared or browser cache.

## WeBWorK and backend-owned Questions

PLE treats the WeBWorK document and form response as opaque. It does not build a
cross-Student render cache, replay map, or control-specific prefetch layer.
Whether another backend can safely cache a presentation is owned by that
backend's explicit contract and must preserve Student isolation.

## Refusal behavior

- A foreign Account, Course, Student record, or Attempt is refused before cache
  delivery.
- A mismatched Revision, position, seed, backend state, schema, or checksum is
  refused.
- A cache miss uses the authoritative owner; it never invents a result.
- Cache failure does not submit an Assessment, change a deadline, erase a saved
  response, or assign credit.

Cache statistics may record bounded operational facts without Question source,
answers, Student identity, responses, grades, or backend secrets. New cache
layers require a measured need and an explicit data-classification review.
