// ribbon_schema.ts - Stable, synchronous Ribbon topology by scope and Product Role.

import type { ProductRole } from "../../generated/api/ProductRole";
import { RIBBON_TAB_IDS, type RibbonScope, type RibbonTabId } from "../route_contract";

/** The complete set of scopes for which the Application Shell owns a Ribbon. */
export const RIBBON_SCOPES = [
  "product",
  "courseInstance",
  "assignmentAttempt",
] as const satisfies ReadonlyArray<RibbonScope>;

/**
 * A relationship that may narrow a future suffix of a Ribbon Schema.
 *
 * `none` identifies a universally available position. Relationship-specific
 * positions must form an append-only suffix, so resolving a relationship can
 * add a control without moving a control already visible to the learner.
 */
export type RibbonRelationshipRequirement =
  "none" | "courseObserver" | "studentObserver" | "grader";

/** One stable position in an ordered Ribbon Schema. */
export interface RibbonSchemaSlot {
  readonly id: RibbonTabId;
  readonly relationshipRequirement: RibbonRelationshipRequirement;
}

type RibbonSchemaTable = Readonly<
  Record<RibbonScope, Readonly<Record<ProductRole, ReadonlyArray<RibbonSchemaSlot>>>>
>;

function universalSlot(id: RibbonTabId): RibbonSchemaSlot {
  return Object.freeze({ id, relationshipRequirement: "none" });
}

function immutableSchema(
  ...slots: ReadonlyArray<RibbonSchemaSlot>
): ReadonlyArray<RibbonSchemaSlot> {
  return Object.freeze([...slots]);
}

const SCHEMAS: RibbonSchemaTable = Object.freeze({
  product: Object.freeze({
    instructor: immutableSchema(
      universalSlot("courses"),
      universalSlot("questionLibrary"),
      universalSlot("blueprintCourses"),
    ),
    student: immutableSchema(universalSlot("courses")),
    sysadmin: immutableSchema(universalSlot("courses"), universalSlot("instructorAccounts")),
  }),
  courseInstance: Object.freeze({
    instructor: immutableSchema(
      universalSlot("assignments"),
      universalSlot("students"),
      universalSlot("gradebook"),
      universalSlot("teachingOperations"),
      universalSlot("blueprintUpdates"),
      universalSlot("courseSetup"),
    ),
    student: immutableSchema(universalSlot("assignments")),
    sysadmin: immutableSchema(universalSlot("teachingOperations")),
  }),
  assignmentAttempt: Object.freeze({
    instructor: immutableSchema(),
    student: immutableSchema(universalSlot("attempt")),
    sysadmin: immutableSchema(),
  }),
});

/**
 * Returns the designed topology for one immutable Product Role and scope.
 *
 * This intentionally does not ask whether a destination has shipped or is
 * authorized. The capability registry and route boundary apply those later;
 * topology remains synchronous and stable for the session.
 */
export function ribbonSchemaFor(
  scope: RibbonScope,
  productRole: ProductRole,
): ReadonlyArray<RibbonSchemaSlot> {
  return SCHEMAS[scope][productRole];
}

/** True when universal positions precede every relationship-narrowed suffix. */
export function hasAppendOnlyRelationshipSuffix(schema: ReadonlyArray<RibbonSchemaSlot>): boolean {
  let relationshipSuffixStarted = false;
  for (const slot of schema) {
    if (slot.relationshipRequirement === "none") {
      if (relationshipSuffixStarted) {
        return false;
      }
      continue;
    }
    relationshipSuffixStarted = true;
  }
  return true;
}

/** Runtime evidence that schema positions always use declared Ribbon tab IDs. */
export function isRibbonTabId(id: string): id is RibbonTabId {
  return RIBBON_TAB_IDS.includes(id as RibbonTabId);
}
