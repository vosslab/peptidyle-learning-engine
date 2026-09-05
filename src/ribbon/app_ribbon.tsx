// app_ribbon.tsx - fixed-geometry, route-owned application Ribbon presentation.

import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  Show,
  type Accessor,
  type JSX,
} from "solid-js";

import "./app_ribbon.css";

import type {
  RibbonActionDescriptor,
  RibbonControlModel,
  RibbonModel,
  RibbonTaskAreaModel,
} from "./ribbon_contract";
import type { RibbonDestinationId } from "./ribbon_catalog";
import {
  RIBBON_ICON_ASSET_PATH,
  ribbonGlyphForContext,
  ribbonGlyphForDestination,
  type RibbonGlyphId,
} from "./ribbon_icons";
import {
  createRibbonPendingNavigation,
  type RibbonPendingNavigation,
} from "./ribbon_pending_navigation";
import {
  RibbonSelectedTabVisibilityController,
  type RibbonRowScrollport,
} from "./ribbon_selected_tab_visibility";

export interface AppRibbonProps {
  /** The complete synchronous presentation model; shell ownership stays outside this component. */
  readonly model: RibbonModel;
  /** Shell-owned navigation progress; absent in static and SSR presentation. */
  readonly routingInFlight?: Accessor<boolean>;
  /** User motion preference injected by the future shell; absent means ordinary motion. */
  readonly reducedMotion?: Accessor<boolean>;
}

// `focusable` is an SVG accessibility attribute supported by browsers but is
// absent from the DOM library's SVG attribute type.
const NON_FOCUSABLE_SVG = { focusable: "false" } as unknown as JSX.SvgSVGAttributes<SVGSVGElement>;

function visibleControl<Id extends RibbonDestinationId>(
  control: RibbonControlModel<Id>,
): control is RibbonControlModel<Id> & { href: string } {
  return control.availability === "Available" && control.href !== undefined;
}

function isUnmodifiedPrimaryActivation(event: MouseEvent): boolean {
  return (
    event.button === 0 &&
    !event.defaultPrevented &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey
  );
}

/**
 * Decorative, same-origin glyph rendering for the closed Ribbon vocabulary.
 * The adjacent text remains the control's accessible name.
 */
export function RibbonIcon(props: { readonly glyph: RibbonGlyphId }): JSX.Element {
  return (
    <svg
      class="ple-app-ribbon__icon"
      aria-hidden="true"
      {...NON_FOCUSABLE_SVG}
      data-ribbon-glyph={props.glyph}
    >
      <use href={`${RIBBON_ICON_ASSET_PATH}#${props.glyph}`} />
    </svg>
  );
}

function RibbonLink(props: {
  readonly control: RibbonControlModel;
  readonly pendingNavigation: RibbonPendingNavigation;
}): JSX.Element {
  // The closed map supplies a glyph only after the presentation model has
  // explicitly declared that this particular control earns one. This keeps a
  // model revision from acquiring a plausible-but-undeclared visual meaning.
  const glyph = (): RibbonGlyphId | undefined =>
    props.control.iconBearing ? ribbonGlyphForDestination(props.control.id) : undefined;
  const iconOnlySafe = (): boolean =>
    props.control.iconBearing && props.control.iconOnlySafe && glyph() !== undefined;
  const pending = (): boolean =>
    props.control.href !== undefined && props.pendingNavigation.isPending(props.control.href);
  return (
    <a
      class={`ple-app-ribbon__link ple-app-ribbon__link--${props.control.presentation}`}
      href={props.control.href}
      aria-current={props.control.selected ? "page" : undefined}
      aria-busy={pending() ? "true" : undefined}
      aria-label={iconOnlySafe() ? props.control.label : undefined}
      title={iconOnlySafe() ? props.control.label : undefined}
      data-ribbon-control={props.control.id}
      data-ribbon-presentation={props.control.presentation}
      data-ribbon-icon-only-safe={iconOnlySafe() ? "true" : undefined}
      data-ribbon-pending={pending() ? "true" : undefined}
      onClick={(event) => {
        if (props.control.href !== undefined && isUnmodifiedPrimaryActivation(event)) {
          props.pendingNavigation.activate(props.control.href);
        }
      }}
    >
      <Show when={glyph()}>{(id) => <RibbonIcon glyph={id()} />}</Show>
      <span class="ple-app-ribbon__control-label">{props.control.label}</span>
    </a>
  );
}

function emitRibbonAction(event: MouseEvent, action: RibbonActionDescriptor): void {
  const target = event.currentTarget as HTMLButtonElement;
  target.dispatchEvent(
    new CustomEvent("ple-ribbon-action", {
      bubbles: true,
      composed: true,
      detail: { id: action.id, kind: action.kind },
    }),
  );
}

interface RibbonOverflowCueState {
  readonly atEnd: Accessor<boolean>;
  readonly atStart: Accessor<boolean>;
  /** Changes for observed layout mutations/resizes, including cue-stable resizes. */
  readonly geometryRevision: Accessor<number>;
  readonly setRow: (element: HTMLElement) => void;
}

/**
 * Each permanent Ribbon row owns its overflow affordance. The state observes
 * only the row's own layout and scroll position, so neither route content nor
 * model data can change its geometry.
 */
function createRibbonOverflowCueState(): RibbonOverflowCueState {
  const [row, setRow] = createSignal<HTMLElement>();
  const [atStart, setAtStart] = createSignal(false);
  const [atEnd, setAtEnd] = createSignal(false);
  const [geometryRevision, setGeometryRevision] = createSignal(0);
  let previousGeometry: string | undefined;

  function updateOverflowCue(geometryChanged = false): void {
    const element = row();
    if (element === undefined) return;
    const overflows = element.scrollWidth > element.clientWidth;
    setAtStart(overflows && element.scrollLeft > 0);
    setAtEnd(overflows && element.scrollLeft + element.clientWidth < element.scrollWidth);
    const geometry = `${element.clientWidth}:${element.scrollWidth}:${element.clientHeight}`;
    if (geometryChanged && geometry !== previousGeometry) {
      previousGeometry = geometry;
      setGeometryRevision((current) => current + 1);
    }
  }

  createEffect(() => {
    const element = row();
    if (element === undefined) return;
    const observer = new ResizeObserver(() => updateOverflowCue(true));
    const contentObserver = new MutationObserver(() => updateOverflowCue(true));
    // Browser text preferences can arrive as a root style/class change before
    // a row-local resize notification. Observe only that presentation seam so
    // selected controls are reconsidered after a real text-scale geometry
    // change, never after a user's ordinary horizontal scroll.
    const rootObserver = new MutationObserver(() => updateOverflowCue(true));
    observer.observe(element);
    contentObserver.observe(element, { childList: true, characterData: true, subtree: true });
    rootObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class", "style"],
    });
    const onScroll = (): void => updateOverflowCue();
    element.addEventListener("scroll", onScroll, { passive: true });
    queueMicrotask(() => updateOverflowCue(true));
    onCleanup(() => {
      observer.disconnect();
      contentObserver.disconnect();
      rootObserver.disconnect();
      element.removeEventListener("scroll", onScroll);
    });
  });

  return { atEnd, atStart, geometryRevision, setRow };
}

function RibbonOverflowCues(props: { readonly state: RibbonOverflowCueState }): JSX.Element {
  return (
    <>
      <span
        class="ple-app-ribbon__overflow-cue ple-app-ribbon__overflow-cue--start"
        aria-hidden="true"
        data-ribbon-overflow-cue="start"
        data-ribbon-overflow-active={props.state.atStart() ? "true" : undefined}
      />
      <span
        class="ple-app-ribbon__overflow-cue ple-app-ribbon__overflow-cue--end"
        aria-hidden="true"
        data-ribbon-overflow-cue="end"
        data-ribbon-overflow-active={props.state.atEnd() ? "true" : undefined}
      />
    </>
  );
}

function TaskArea(props: {
  readonly area: RibbonTaskAreaModel;
  readonly pendingNavigation: RibbonPendingNavigation;
}): JSX.Element {
  const controls = (): ReadonlyArray<RibbonControlModel & { href: string }> =>
    props.area.controls.filter(visibleControl);
  return (
    <Show when={controls().length > 0}>
      <span class="ple-app-ribbon__task-area" data-ribbon-task-area={props.area.id}>
        <span class="ple-app-ribbon__task-area-label">{props.area.label}</span>
        <For each={controls()}>
          {(control) => (
            <RibbonLink control={control} pendingNavigation={props.pendingNavigation} />
          )}
        </For>
      </span>
    </Show>
  );
}

/**
 * A model-only Ribbon with narrow local interaction state. Derivation, labels,
 * admission, and data ownership stay external; this component owns only
 * pending feedback and selected-Tab presentation behavior.
 */
export function AppRibbon(props: AppRibbonProps): JSX.Element {
  const routingInFlight: Accessor<boolean> = props.routingInFlight ?? ((): boolean => false);
  const reducedMotion: Accessor<boolean> = props.reducedMotion ?? ((): boolean => false);
  const pendingNavigation = createRibbonPendingNavigation({ routingInFlight });
  const selectedTabVisibility = new RibbonSelectedTabVisibilityController();
  const selectedTaskVisibility = new RibbonSelectedTabVisibilityController();
  const tabScrollport: { current: HTMLElement | undefined } = { current: undefined };
  const taskScrollport: { current: HTMLElement | undefined } = { current: undefined };
  const contextOverflow = createRibbonOverflowCueState();
  const tabOverflow = createRibbonOverflowCueState();
  const taskOverflow = createRibbonOverflowCueState();
  let observationVersion = 0;
  let taskObservationVersion = 0;
  let disposed = false;
  const visibleTabs = (): ReadonlyArray<RibbonControlModel & { href: string }> =>
    props.model.tabs.filter(visibleControl);
  const selectedTab = createMemo(() => visibleTabs().find((control) => control.selected));
  const selectedTask = createMemo(() =>
    props.model.taskAreas
      .flatMap((area) => area.controls)
      .filter(visibleControl)
      .find((control) => control.selected),
  );

  function cueSafeViewport(
    scrollport: HTMLElement | undefined,
    overflow: RibbonOverflowCueState,
    explicitCueSafeScroll = false,
  ): RibbonRowScrollport | undefined {
    if (scrollport === undefined) return undefined;
    return {
      getBoundingClientRect: (): { readonly left: number; readonly right: number } => {
        const bounds = scrollport.getBoundingClientRect();
        const style = getComputedStyle(scrollport);
        const startInset = overflow.atStart()
          ? Number.parseFloat(style.scrollPaddingInlineStart) || 0
          : 0;
        const endInset = overflow.atEnd()
          ? Number.parseFloat(style.scrollPaddingInlineEnd) || 0
          : 0;
        return { left: bounds.left + startInset, right: bounds.right - endInset };
      },
      ...(explicitCueSafeScroll
        ? { scrollBy: (options: ScrollToOptions): void => scrollport.scrollBy(options) }
        : {}),
    };
  }

  // Solid writes the keyed Tab DOM before this queued observation runs. A new
  // model revision invalidates its predecessor, so a rapid Tab change cannot
  // reveal a stale destination after the current selection has moved on.
  createEffect(() => {
    const selectedKey = selectedTab()?.id;
    tabOverflow.geometryRevision();
    const version = ++observationVersion;
    onCleanup(() => {
      if (version === observationVersion) observationVersion += 1;
    });
    queueMicrotask(() => {
      if (disposed || version !== observationVersion || selectedTab()?.id !== selectedKey) {
        return;
      }
      const tab = [
        ...(tabScrollport.current?.querySelectorAll("[data-ribbon-control]") ?? []),
      ].find((element) => element.getAttribute("data-ribbon-control") === selectedKey);
      const scrollport = tabScrollport.current;
      selectedTabVisibility.observe(
        selectedKey,
        tab instanceof HTMLAnchorElement ? tab : undefined,
        cueSafeViewport(scrollport, tabOverflow),
        reducedMotion,
      );
    });
  });

  // Task selection receives the same cue-safe reveal guarantee as Tabs. The
  // overflow revision is intentionally observed so a 200% text setting or a
  // viewport resize can reveal a now-clipped selected task without changing
  // model state or any Ribbon box geometry.
  createEffect(() => {
    const selectedKey = selectedTask()?.id;
    taskOverflow.geometryRevision();
    const version = ++taskObservationVersion;
    onCleanup(() => {
      if (version === taskObservationVersion) taskObservationVersion += 1;
    });
    queueMicrotask(() => {
      if (disposed || version !== taskObservationVersion || selectedTask()?.id !== selectedKey) {
        return;
      }
      const task = [
        ...(taskScrollport.current?.querySelectorAll("[data-ribbon-control]") ?? []),
      ].find((element) => element.getAttribute("data-ribbon-control") === selectedKey);
      selectedTaskVisibility.observe(
        selectedKey,
        task instanceof HTMLAnchorElement ? task : undefined,
        cueSafeViewport(taskScrollport.current, taskOverflow, true),
        reducedMotion,
      );
    });
  });

  onCleanup(() => {
    disposed = true;
    observationVersion += 1;
    taskObservationVersion += 1;
    tabScrollport.current = undefined;
    taskScrollport.current = undefined;
  });

  return (
    <section
      class="ple-app-ribbon"
      aria-label="PLE application Ribbon"
      data-ribbon-scope={props.model.scope}
    >
      <section class="ple-app-ribbon__row-frame" data-ribbon-row-frame="context">
        <section
          class="ple-app-ribbon__row ple-app-ribbon__context"
          aria-label="Ribbon context"
          data-ribbon-row="context"
          ref={contextOverflow.setRow}
        >
          <div class="ple-app-ribbon__context-identity">
            <span class="ple-app-ribbon__product-name">Peptidyle Learning Engine</span>
            <span class="ple-app-ribbon__product-role">{props.model.context.productLabel}</span>
          </div>
          <div class="ple-app-ribbon__context-details">
            <span class="ple-app-ribbon__account-label">
              <RibbonIcon glyph={ribbonGlyphForContext("account")} />
              <span>{props.model.context.accountLabel}</span>
            </span>
            <Show when={props.model.context.scopeLabel}>
              {(label) => <span class="ple-app-ribbon__course-scope-label">{label()}</span>}
            </Show>
            <Show when={props.model.context.assignmentLabel}>
              {(label) => <span>{label()}</span>}
            </Show>
            <Show when={props.model.context.assignmentAttemptProgress}>
              {(label) => <span>{label()}</span>}
            </Show>
          </div>
          <button
            class="ple-app-ribbon__sign-out"
            type="button"
            aria-label={props.model.context.signOutAction.label}
            title={props.model.context.signOutAction.label}
            data-ribbon-action={props.model.context.signOutAction.id}
            data-ribbon-icon-only-safe="true"
            onClick={(event) => emitRibbonAction(event, props.model.context.signOutAction)}
          >
            <RibbonIcon glyph={ribbonGlyphForContext("signOut")} />
            <span class="ple-app-ribbon__control-label">
              {props.model.context.signOutAction.label}
            </span>
          </button>
        </section>
        <RibbonOverflowCues state={contextOverflow} />
      </section>
      <section class="ple-app-ribbon__row-frame" data-ribbon-row-frame="tabs">
        <nav
          class="ple-app-ribbon__row ple-app-ribbon__tabs"
          aria-label="Ribbon tabs"
          data-ribbon-row="tabs"
          ref={(element): void => {
            tabScrollport.current = element;
            tabOverflow.setRow(element);
          }}
        >
          <For each={visibleTabs()}>
            {(control) => <RibbonLink control={control} pendingNavigation={pendingNavigation} />}
          </For>
        </nav>
        <RibbonOverflowCues state={tabOverflow} />
      </section>
      <section class="ple-app-ribbon__row-frame" data-ribbon-row-frame="tasks">
        <nav
          class="ple-app-ribbon__row ple-app-ribbon__tasks"
          aria-label="Ribbon tasks"
          data-ribbon-row="tasks"
          ref={(element): void => {
            taskScrollport.current = element;
            taskOverflow.setRow(element);
          }}
        >
          <For each={props.model.taskAreas}>
            {(area) => <TaskArea area={area} pendingNavigation={pendingNavigation} />}
          </For>
        </nav>
        <RibbonOverflowCues state={taskOverflow} />
      </section>
    </section>
  );
}
