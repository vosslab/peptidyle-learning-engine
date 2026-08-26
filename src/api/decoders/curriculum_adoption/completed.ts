// Closed decoders for immutable B2 operation receipts.

import type { AlphaInstantiationCompleted } from "../../../../generated/api/AlphaInstantiationCompleted";
import type { AssignmentFastForwardCompleted } from "../../../../generated/api/AssignmentFastForwardCompleted";
import type { BlueprintInstantiationCompleted } from "../../../../generated/api/BlueprintInstantiationCompleted";
import type { CourseRolloverCompleted } from "../../../../generated/api/CourseRolloverCompleted";
import type { CourseTermShiftCompleted } from "../../../../generated/api/CourseTermShiftCompleted";
import type { CurriculumAdoptionReceiptBinding } from "../../../../generated/api/CurriculumAdoptionReceiptBinding";
import type { ForkAlphaCompleted } from "../../../../generated/api/ForkAlphaCompleted";
import type { SourceDerivedAssignmentCompleted } from "../../../../generated/api/SourceDerivedAssignmentCompleted";
import type { ObservedAlphaSource } from "../../../../generated/api/ObservedAlphaSource";
import { DecodeError, decodeRecord, decodeString, decodeStringEnum } from "../../decoder";
import { decodeAssignmentReference } from "../catalog_course";
import { decodeCourseTerm } from "../course_term";
import { field, requireOnlyFields } from "../shared";

function closed(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, keys);
  for (const key of keys) field(record, key, path);
  return record;
}

function routeReference(value: unknown, path: string, prefix: "C" | "A" | "AC"): string {
  const reference = decodeString(value, path);
  const match = new RegExp(`^${prefix}-([1-9][0-9]{0,9})$`, "u").exec(reference);
  if (match === null || Number(match[1]) > 2_147_483_647)
    throw new DecodeError(path, `a canonical ${prefix} route reference`);
  return reference;
}

function revision(value: unknown, path: string): string {
  const valueString = decodeString(value, path);
  if (!/^[1-9][0-9]{0,18}$/u.test(valueString) || BigInt(valueString) > 9_223_372_036_854_775_807n)
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
  return valueString;
}

function observedAlpha(value: unknown, path: string): ObservedAlphaSource {
  const record = closed(value, path, ["reference", "revision"]);
  return {
    reference: routeReference(field(record, "reference", path), `${path}.reference`, "AC"),
    revision: revision(field(record, "revision", path), `${path}.revision`),
  };
}

function receipt(value: unknown, path: string): CurriculumAdoptionReceiptBinding {
  const record = closed(value, path, ["idempotencyKey"]);
  const key = decodeString(field(record, "idempotencyKey", path), `${path}.idempotencyKey`);
  if (!/^[A-Za-z0-9._-]+$/u.test(key) || new TextEncoder().encode(key).length > 128)
    throw new DecodeError(`${path}.idempotencyKey`, "a bounded opaque idempotency key");
  return { idempotencyKey: key };
}

function replay(value: unknown, path: string): "applied" | "replayed" {
  return decodeStringEnum(value, path, ["applied", "replayed"] as const);
}

export function decodeForkAlphaCompleted(value: unknown, path = "response"): ForkAlphaCompleted {
  const record = closed(value, path, ["source", "alpha", "replay", "receipt"]);
  return {
    source: observedAlpha(field(record, "source", path), `${path}.source`),
    alpha: routeReference(field(record, "alpha", path), `${path}.alpha`, "AC"),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}

export function decodeBlueprintInstantiationCompleted(
  value: unknown,
  path = "response",
): BlueprintInstantiationCompleted {
  const record = closed(value, path, ["course", "assignment", "replay", "receipt"]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}

export function decodeAlphaInstantiationCompleted(
  value: unknown,
  path = "response",
): AlphaInstantiationCompleted {
  const record = closed(value, path, ["source", "course", "replay", "receipt"]);
  return {
    source: observedAlpha(field(record, "source", path), `${path}.source`),
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}

export function decodeCourseRolloverCompleted(
  value: unknown,
  path = "response",
): CourseRolloverCompleted {
  const record = closed(value, path, ["sourceCourse", "course", "replay", "receipt"]);
  return {
    sourceCourse: routeReference(field(record, "sourceCourse", path), `${path}.sourceCourse`, "C"),
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}

export function decodeCourseTermShiftCompleted(
  value: unknown,
  path = "response",
): CourseTermShiftCompleted {
  const record = closed(value, path, ["course", "term", "replay", "receipt"]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}

export function decodeAssignmentFastForwardCompleted(
  value: unknown,
  path = "response",
): AssignmentFastForwardCompleted {
  const record = closed(value, path, [
    "course",
    "assignment",
    "importRevision",
    "replay",
    "receipt",
  ]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    importRevision: revision(field(record, "importRevision", path), `${path}.importRevision`),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}

export function decodeSourceDerivedAssignmentCompleted(
  value: unknown,
  path = "response",
): SourceDerivedAssignmentCompleted {
  const record = closed(value, path, ["course", "assignment", "replay", "receipt"]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    replay: replay(field(record, "replay", path), `${path}.replay`),
    receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
  };
}
