// ribbon_schema.ts - Stable, synchronous Ribbon topology by scope and Product Role.

import type { ProductRole } from "../../generated/api/ProductRole";
import { RIBBON_TAB_IDS, type RibbonTabId } from "../route_contract";

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

export type ProductTierOne = Readonly<Record<ProductRole, ReadonlyArray<RibbonSchemaSlot>>>;

function universalSlot(id: RibbonTabId): RibbonSchemaSlot {
  return Object.freeze({ id, relationshipRequirement: "none" });
}

function immutableSchema(
  ...slots: ReadonlyArray<RibbonSchemaSlot>
): ReadonlyArray<RibbonSchemaSlot> {
  return Object.freeze([...slots]);
}

export const PRODUCT_TIER_ONE: ProductTierOne = Object.freeze({
  instructor: immutableSchema(
    universalSlot("courses"),
    universalSlot("questions"),
    universalSlot("productAssessments"),
  ),
  student: immutableSchema(
    universalSlot("coursework"),
    universalSlot("grades"),
    universalSlot("courses"),
  ),
  sysadmin: immutableSchema(
    universalSlot("courses"),
    universalSlot("questions"),
    universalSlot("instructorAccounts"),
    universalSlot("disciplines"),
  ),
});

/**
 * Returns the designed tier-one topology for one immutable Product Role.
 *
 * This intentionally does not ask whether a destination has shipped or is
 * authorized. The capability registry and route boundary apply those later;
 * topology remains synchronous and stable for the session.
 */
export function ribbonSchemaFor(productRole: ProductRole): ReadonlyArray<RibbonSchemaSlot> {
  return PRODUCT_TIER_ONE[productRole];
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
