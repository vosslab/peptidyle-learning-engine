// ribbon_selected_tab_visibility.ts - browser-neutral selected Ribbon Tab visibility behavior.

/** The horizontal bounds exposed by DOMRect and the test geometry adapter. */
export interface RibbonHorizontalBounds {
  readonly left: number;
  readonly right: number;
}

/** The minimum element surface needed to decide and perform a horizontal reveal. */
export interface RibbonTabElement {
  readonly getBoundingClientRect: () => RibbonHorizontalBounds;
  readonly scrollIntoView?: (options: ScrollIntoViewOptions) => void;
}

/** The minimum row scrollport surface needed for a fully-visible comparison. */
export interface RibbonRowScrollport {
  readonly getBoundingClientRect: () => RibbonHorizontalBounds;
  readonly scrollBy?: (options: ScrollToOptions) => void;
}

export type ReducedMotionPreference = boolean | (() => boolean);

/**
 * Returns true only when the complete horizontal Tab rectangle lies in the
 * row scrollport. Boundary contact is visible and therefore counts as inside.
 */
export function isFullyVisibleInRibbonRow(
  tabBounds: RibbonHorizontalBounds,
  scrollportBounds: RibbonHorizontalBounds,
): boolean {
  return tabBounds.left >= scrollportBounds.left && tabBounds.right <= scrollportBounds.right;
}

function reducedMotionRequested(preference: ReducedMotionPreference): boolean {
  const requested = typeof preference === "function" ? preference() : preference;
  return requested;
}

function revealOptions(preference: ReducedMotionPreference): ScrollIntoViewOptions {
  const behavior: ScrollBehavior = reducedMotionRequested(preference) ? "auto" : "smooth";
  return { behavior, block: "nearest", inline: "nearest" };
}

/**
 * A row may call `observe` after either a model selection or a geometry
 * revision. An already-visible control never moves; a selected control that
 * becomes clipped after a text-size or viewport change is eligible again.
 */
export class RibbonSelectedTabVisibilityController {
  /**
   * The selection that owns the current suppression record. This deliberately
   * is not a historical cache: returning to a previously selected Tab begins
   * a new selection epoch, even when its rectangle is numerically identical
   * to its earlier visit.
   */
  private observedSelection: string | undefined;
  private previousRevealInSelection: string | undefined;

  private beginSelectionEpoch(selectedKey: string | undefined): void {
    if (this.observedSelection === selectedKey) return;
    this.observedSelection = selectedKey;
    this.previousRevealInSelection = undefined;
  }

  /**
   * Reveals a selected, horizontally clipped control. The caller passes no
   * element for unavailable or absent controls, which intentionally performs
   * no scroll operation. A control wider than its usable viewport gets one
   * deterministic reveal attempt per geometry, avoiding a scroll/observer
   * loop while still retrying after a meaningful resize.
   */
  observe(
    selectedKey: string | undefined,
    tab: RibbonTabElement | undefined,
    scrollport: RibbonRowScrollport | undefined,
    reducedMotion: ReducedMotionPreference,
  ): boolean {
    this.beginSelectionEpoch(selectedKey);
    if (selectedKey === undefined || tab === undefined || scrollport === undefined) {
      return false;
    }
    const tabBounds = tab.getBoundingClientRect();
    const scrollportBounds = scrollport.getBoundingClientRect();
    if (isFullyVisibleInRibbonRow(tabBounds, scrollportBounds)) return false;

    // A repeated observer notification with unchanged clipped geometry cannot
    // reveal anything new. Treat it as settled. Any scroll, resize, or text
    // scale that changes the rectangles produces a fresh signature and is
    // allowed to try again. This also bounds an over-wide control to one
    // attempt for each geometry.
    const revealSignature = [
      tabBounds.left,
      tabBounds.right,
      scrollportBounds.left,
      scrollportBounds.right,
    ].join(":");
    if (this.previousRevealInSelection === revealSignature) return false;

    const options = revealOptions(reducedMotion);
    if (scrollport.scrollBy !== undefined) {
      // `scrollIntoView` is permitted to treat the physical edge as visible
      // even when our pinned fade reserves that edge for paint. A real row
      // can therefore make the cue-safe delta explicit.
      const delta =
        tabBounds.left < scrollportBounds.left
          ? tabBounds.left - scrollportBounds.left
          : tabBounds.right - scrollportBounds.right;
      scrollport.scrollBy({ behavior: options.behavior, left: delta });
    } else if (tab.scrollIntoView !== undefined) {
      tab.scrollIntoView(options);
    } else {
      return false;
    }
    this.previousRevealInSelection = revealSignature;
    return true;
  }
}
