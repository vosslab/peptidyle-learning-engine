// Instructor-facing Question Library usage statistics decoder.
import type { QuestionRevisionUsageStatistics } from "../../../generated/api/QuestionRevisionUsageStatistics";
import type { QuestionStatistics } from "../../../generated/api/QuestionStatistics";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

export function decodeQuestionStatistics(value: unknown, path: string): QuestionStatistics {
  const record = decodeRecord(value, path);
  const state = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "unavailable",
    "available",
  ] as const satisfies ReadonlyArray<QuestionStatistics["state"]>);
  if (state === "unavailable") {
    requireOnlyFields(record, path, ["state"]);
    return { state };
  }
  requireOnlyFields(record, path, [
    "state",
    "issued_count",
    "blank_count",
    "answered_count",
    "correct_count",
    "partial_count",
    "incorrect_count",
    "credit_sum",
    "credit_sum_sq",
    "blank_rate",
    "answered_rate",
    "correct_rate",
    "partial_rate",
    "incorrect_rate",
    "mean_credit",
    "revisions",
    "pool_issued_count",
  ]);
  const issuedCount = decodeNonnegativeInteger(
    field(record, "issued_count", path),
    `${path}.issued_count`,
  );
  const blankCount = decodeNonnegativeInteger(
    field(record, "blank_count", path),
    `${path}.blank_count`,
  );
  const answeredCount = decodeNonnegativeInteger(
    field(record, "answered_count", path),
    `${path}.answered_count`,
  );
  const correctCount = decodeNonnegativeInteger(
    field(record, "correct_count", path),
    `${path}.correct_count`,
  );
  const partialCount = decodeNonnegativeInteger(
    field(record, "partial_count", path),
    `${path}.partial_count`,
  );
  const incorrectCount = decodeNonnegativeInteger(
    field(record, "incorrect_count", path),
    `${path}.incorrect_count`,
  );
  const creditSum = decodeFiniteNumber(field(record, "credit_sum", path), `${path}.credit_sum`);
  const creditSumSq = decodeFiniteNumber(
    field(record, "credit_sum_sq", path),
    `${path}.credit_sum_sq`,
  );
  if (creditSum < 0 || creditSumSq < 0) {
    throw new DecodeError(`${path}.credit_sum`, "a nonnegative credit sum");
  }
  const issuedRate = issuedCount > 0;
  const answeredRate = answeredCount > 0;
  const blankRate = decodeOptionalRate(record["blank_rate"], `${path}.blank_rate`, issuedRate);
  const answeredRateValue = decodeOptionalRate(
    record["answered_rate"],
    `${path}.answered_rate`,
    issuedRate,
  );
  const correctRate = decodeOptionalRate(
    record["correct_rate"],
    `${path}.correct_rate`,
    answeredRate,
  );
  const partialRate = decodeOptionalRate(
    record["partial_rate"],
    `${path}.partial_rate`,
    answeredRate,
  );
  const incorrectRate = decodeOptionalRate(
    record["incorrect_rate"],
    `${path}.incorrect_rate`,
    answeredRate,
  );
  const meanCredit = decodeOptionalRate(record["mean_credit"], `${path}.mean_credit`, answeredRate);
  const revisions = Object.prototype.hasOwnProperty.call(record, "revisions")
    ? decodeArray(record["revisions"], `${path}.revisions`, decodeQuestionRevisionUsageStatistics)
    : undefined;
  const poolIssuedCount = Object.prototype.hasOwnProperty.call(record, "pool_issued_count")
    ? (decodeNullable(
        record["pool_issued_count"],
        `${path}.pool_issued_count`,
        decodeNonnegativeInteger,
      ) ?? undefined)
    : undefined;
  return {
    state,
    issued_count: issuedCount,
    blank_count: blankCount,
    answered_count: answeredCount,
    correct_count: correctCount,
    partial_count: partialCount,
    incorrect_count: incorrectCount,
    credit_sum: creditSum,
    credit_sum_sq: creditSumSq,
    ...(blankRate === undefined ? {} : { blank_rate: blankRate }),
    ...(answeredRateValue === undefined ? {} : { answered_rate: answeredRateValue }),
    ...(correctRate === undefined ? {} : { correct_rate: correctRate }),
    ...(partialRate === undefined ? {} : { partial_rate: partialRate }),
    ...(incorrectRate === undefined ? {} : { incorrect_rate: incorrectRate }),
    ...(meanCredit === undefined ? {} : { mean_credit: meanCredit }),
    ...(revisions === undefined ? {} : { revisions }),
    ...(poolIssuedCount === undefined ? {} : { pool_issued_count: poolIssuedCount }),
  };
}

function decodeOptionalRate(value: unknown, path: string, required: boolean): number | undefined {
  if (value === undefined || value === null) {
    if (required) {
      throw new DecodeError(path, "a rate from 0 through 1");
    }
    return undefined;
  }
  if (!required) {
    throw new DecodeError(path, "omitted when the denominator is zero");
  }
  const rate = decodeFiniteNumber(value, path);
  if (rate < 0 || rate > 1) {
    throw new DecodeError(path, "a rate from 0 through 1");
  }
  return rate;
}

function decodeQuestionRevisionUsageStatistics(
  value: unknown,
  path: string,
): QuestionRevisionUsageStatistics {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "revision_number",
    "issued_count",
    "blank_count",
    "answered_count",
    "correct_count",
    "partial_count",
    "incorrect_count",
    "credit_sum",
    "credit_sum_sq",
    "blank_rate",
    "answered_rate",
    "correct_rate",
    "partial_rate",
    "incorrect_rate",
    "mean_credit",
  ]);
  const issuedCount = decodeNonnegativeInteger(
    field(record, "issued_count", path),
    `${path}.issued_count`,
  );
  const answeredCount = decodeNonnegativeInteger(
    field(record, "answered_count", path),
    `${path}.answered_count`,
  );
  const issuedRate = issuedCount > 0;
  const answeredRate = answeredCount > 0;
  const blankRate = decodeOptionalRate(record["blank_rate"], `${path}.blank_rate`, issuedRate);
  const answeredRateValue = decodeOptionalRate(
    record["answered_rate"],
    `${path}.answered_rate`,
    issuedRate,
  );
  const correctRate = decodeOptionalRate(
    record["correct_rate"],
    `${path}.correct_rate`,
    answeredRate,
  );
  const partialRate = decodeOptionalRate(
    record["partial_rate"],
    `${path}.partial_rate`,
    answeredRate,
  );
  const incorrectRate = decodeOptionalRate(
    record["incorrect_rate"],
    `${path}.incorrect_rate`,
    answeredRate,
  );
  const meanCredit = decodeOptionalRate(record["mean_credit"], `${path}.mean_credit`, answeredRate);
  return {
    revision_number: decodePositiveInteger(
      field(record, "revision_number", path),
      `${path}.revision_number`,
    ),
    issued_count: issuedCount,
    blank_count: decodeNonnegativeInteger(
      field(record, "blank_count", path),
      `${path}.blank_count`,
    ),
    answered_count: answeredCount,
    correct_count: decodeNonnegativeInteger(
      field(record, "correct_count", path),
      `${path}.correct_count`,
    ),
    partial_count: decodeNonnegativeInteger(
      field(record, "partial_count", path),
      `${path}.partial_count`,
    ),
    incorrect_count: decodeNonnegativeInteger(
      field(record, "incorrect_count", path),
      `${path}.incorrect_count`,
    ),
    credit_sum: decodeFiniteNumber(field(record, "credit_sum", path), `${path}.credit_sum`),
    credit_sum_sq: decodeFiniteNumber(
      field(record, "credit_sum_sq", path),
      `${path}.credit_sum_sq`,
    ),
    ...(blankRate === undefined ? {} : { blank_rate: blankRate }),
    ...(answeredRateValue === undefined ? {} : { answered_rate: answeredRateValue }),
    ...(correctRate === undefined ? {} : { correct_rate: correctRate }),
    ...(partialRate === undefined ? {} : { partial_rate: partialRate }),
    ...(incorrectRate === undefined ? {} : { incorrect_rate: incorrectRate }),
    ...(meanCredit === undefined ? {} : { mean_credit: meanCredit }),
  };
}
