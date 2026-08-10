# Frequently asked questions

This page answers common orientation questions about Peptidyle's learning model,
browser boundary, and question formats. It links to the authoritative contracts
for readers who need implementation detail.

## Does mastery end practice?

No. Mastery, scoring, continued practice, and variation are independent
assignment policies. An instructor can require mastery, keep the highest score,
allow unlimited practice after completion, and issue fresh parameter seeds for
each new run. A resumed attempt keeps its original seed so its question does not
change mid-attempt. See [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).

## What runs in Solid and Wasm?

The Solid single-page application presents routes, input controls, progress, and
recovery states. Its one browser-safe Rust WebAssembly module generates allowed
parameters and validates response format. `src/wasm/index.ts` is the sole
browser import boundary for generated `wasm-bindgen` glue; components use its
typed facade rather than raw exports. See [FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md)
and [SOLID_MODEL.md](SOLID_MODEL.md).

## Why is grading server-only?

The browser may check response format, but it never receives answer keys,
grading implementations, or correctness decisions. Those live in
`crates/grading`, which is outside the WebAssembly dependency closure. The
server repeats format validation and then makes the authoritative grading
decision. If WebAssembly is unavailable, the browser uses a key-free server
format-validation route; it does not fall back to local grading. See
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and
[QUESTION_MODEL.md](QUESTION_MODEL.md).

## Is PLE flat-question JSON QTI?

No. PLE flat-question JSON is the small, versioned, answer-bearing authoring
format for ordinary static questions. The native adapter compiles it into an
answer-free public question model and separate grader-only material. QTI is a
bounded import/export adapter and archival interchange format, so vendor XML
and QTI expression trees do not become PLE's internal schema. Version 1
`singleChoice` remains stable while PLE flat JSON version 2 supplies all eight
native families at one versioned compiler boundary. A future QTI-JSONL format
would be an external adapter, not the internal source model. See
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) and
[flat_question_family_evolution_plan.md](active_plans/active/flat_question_family_evolution_plan.md).

## Can a student browser contact WeBWorK?

No. PLE is the sole WeBWorK client. The optional renderer profile is private;
the browser continues to call PLE through its same-origin gateway. The current
integration is limited to one server-rendered PGML `RadioButtons` path, with
the Rust client and local profile implemented but live acceptance still
pending. Native questions and the normal local stack do not require it. See
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) and
[CONTAINER.md](CONTAINER.md).
