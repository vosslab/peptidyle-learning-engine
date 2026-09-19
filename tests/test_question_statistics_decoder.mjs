import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionStatistics } from "../src/api/decoders/question_statistics.ts";

test("Question Statistics unavailable stays a closed state tag", () => {
  assert.deepEqual(decodeQuestionStatistics({ state: "unavailable" }, "evidence"), {
    state: "unavailable",
  });
  assert.throws(
    () => decodeQuestionStatistics({ state: "unavailable", issued_count: 0 }, "evidence"),
    DecodeError,
  );
});

test("Question Statistics available uses snake_case counts and omits zero-denominator rates", () => {
  assert.deepEqual(
    decodeQuestionStatistics(
      {
        state: "available",
        issued_count: 0,
        blank_count: 0,
        answered_count: 0,
        correct_count: 0,
        partial_count: 0,
        incorrect_count: 0,
        credit_sum: 0,
        credit_sum_sq: 0,
      },
      "evidence",
    ),
    {
      state: "available",
      issued_count: 0,
      blank_count: 0,
      answered_count: 0,
      correct_count: 0,
      partial_count: 0,
      incorrect_count: 0,
      credit_sum: 0,
      credit_sum_sq: 0,
    },
  );
  const available = decodeQuestionStatistics(
    {
      state: "available",
      issued_count: 4,
      blank_count: 1,
      answered_count: 3,
      correct_count: 2,
      partial_count: 1,
      incorrect_count: 0,
      credit_sum: 2.5,
      credit_sum_sq: 2.25,
      blank_rate: 0.25,
      answered_rate: 0.75,
      correct_rate: 2 / 3,
      partial_rate: 1 / 3,
      incorrect_rate: 0,
      mean_credit: 2.5 / 3,
      revisions: [
        {
          revision_number: 1,
          issued_count: 4,
          blank_count: 1,
          answered_count: 3,
          correct_count: 2,
          partial_count: 1,
          incorrect_count: 0,
          credit_sum: 2.5,
          credit_sum_sq: 2.25,
          blank_rate: 0.25,
          answered_rate: 0.75,
          correct_rate: 2 / 3,
          partial_rate: 1 / 3,
          incorrect_rate: 0,
          mean_credit: 2.5 / 3,
        },
      ],
    },
    "evidence",
  );
  assert.equal(available.state, "available");
  if (available.state !== "available") {
    throw new Error("expected available Question Statistics");
  }
  assert.equal(available.issued_count, 4);
  assert.equal(available.blank_rate, 0.25);
  assert.equal(available.revisions?.length, 1);
  assert.throws(
    () =>
      decodeQuestionStatistics(
        {
          state: "available",
          issued_count: 0,
          blank_count: 0,
          answered_count: 0,
          correct_count: 0,
          partial_count: 0,
          incorrect_count: 0,
          credit_sum: 0,
          credit_sum_sq: 0,
          blank_rate: 0,
        },
        "evidence",
      ),
    DecodeError,
  );
});
