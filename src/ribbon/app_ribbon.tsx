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
  RibbonContextControlModel,
  RibbonControlModel,
  RibbonModel,
  RibbonTaskAreaModel,
} from "./ribbon_contract";
import type { RibbonDestinationId } from "./ribbon_catalog";
import {
  ribbonGlyphForContext,
  ribbonGlyphForDestination,
  type RibbonGlyphId,
} from "./ribbon_icons";
import { RibbonIcon } from "./ribbon_icon";
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
  /** Optional selected-avatar presentation; the shared control falls back to the generic user glyph. */
  readonly renderProfileAvatar?: () => JSX.Element;
}

function visibleControl<Id extends RibbonDestinationId>(
  control: RibbonControlModel<Id>,
): control is RibbonControlModel<Id> & { href: string } {
  return control.availability === "Available" && control.href !== undefined;
}

function visibleAccountControl(
  control: RibbonContextControlModel,
): control is RibbonContextControlModel & { href: string } {
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
  const hasReservedTaskRow = (): boolean => props.model.taskAreas.length > 0;
  const tabScrollport: { current: HTMLElement | undefined } = { current: undefined };
  const taskScrollport: { current: HTMLElement | undefined } = { current: undefined };
  const topOverflow = createRibbonOverflowCueState();
  const taskOverflow = createRibbonOverflowCueState();
  let observationVersion = 0;
  let taskObservationVersion = 0;
  let disposed = false;
  const [profileMenuOpen, setProfileMenuOpen] = createSignal(false);
  let profileTrigger: HTMLButtonElement | undefined;
  let profileMenu: HTMLDivElement | undefined;
  const visibleTabs = (): ReadonlyArray<RibbonControlModel & { href: string }> =>
    props.model.tabs.filter(visibleControl);
  const selectedTab = createMemo(() => visibleTabs().find((control) => control.selected));
  const selectedTask = createMemo(() =>
    props.model.taskAreas
      .flatMap((area) => area.controls)
      .filter(visibleControl)
      .find((control) => control.selected),
  );
  const profileLink = createMemo(() =>
    props.model.context.accountControls.find(visibleAccountControl),
  );

  function menuItems(): ReadonlyArray<HTMLElement> {
    return [...(profileMenu?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
  }

  function openProfileMenu(focus: "first" | "last" | "none" = "first"): void {
    setProfileMenuOpen(true);
    if (focus === "none") return;
    queueMicrotask(() => {
      const items = menuItems();
      (focus === "first" ? items[0] : items[items.length - 1])?.focus();
    });
  }

  function closeProfileMenu(restoreFocus = false): void {
    setProfileMenuOpen(false);
    if (restoreFocus) queueMicrotask(() => profileTrigger?.focus());
  }

  function handleProfileTriggerKeyDown(event: KeyboardEvent): void {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      openProfileMenu("first");
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      openProfileMenu("last");
    } else if (event.key === "Escape" && profileMenuOpen()) {
      event.preventDefault();
      closeProfileMenu();
    }
  }

  function handleProfileMenuKeyDown(event: KeyboardEvent): void {
    const items = menuItems();
    const currentIndex = items.findIndex((item) => item === document.activeElement);
    if (event.key === "Escape") {
      event.preventDefault();
      closeProfileMenu(true);
      return;
    }
    if (
      event.key === "ArrowDown" ||
      event.key === "ArrowUp" ||
      event.key === "Home" ||
      event.key === "End"
    ) {
      event.preventDefault();
      if (items.length === 0) return;
      const nextIndex =
        event.key === "Home"
          ? 0
          : event.key === "End"
            ? items.length - 1
            : event.key === "ArrowDown"
              ? (currentIndex + 1 + items.length) % items.length
              : (currentIndex - 1 + items.length) % items.length;
      items[nextIndex]?.focus();
    }
  }

  createEffect(() => {
    if (!profileMenuOpen()) return;
    const closeOnOutsidePointer = (event: PointerEvent): void => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (profileTrigger?.contains(target) || profileMenu?.contains(target)) return;
      closeProfileMenu();
    };
    document.addEventListener("pointerdown", closeOnOutsidePointer);
    onCleanup(() => document.removeEventListener("pointerdown", closeOnOutsidePointer));
  });

  function cueSafeViewport(
    scrollport: HTMLElement | undefined,
    explicitCueSafeScroll = false,
  ): RibbonRowScrollport | undefined {
    if (scrollport === undefined) return undefined;
    return {
      getBoundingClientRect: (): { readonly left: number; readonly right: number } => {
        const bounds = scrollport.getBoundingClientRect();
        const style = getComputedStyle(scrollport);
        const overflows = scrollport.scrollWidth > scrollport.clientWidth;
        const startInset = overflows ? Number.parseFloat(style.scrollPaddingInlineStart) || 0 : 0;
        const endInset = overflows ? Number.parseFloat(style.scrollPaddingInlineEnd) || 0 : 0;
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
    topOverflow.geometryRevision();
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
        cueSafeViewport(scrollport, true),
        reducedMotion,
      );
    });
  });

  // Task selection receives the same cue-safe reveal guarantee as Tabs. The
  // overflow revision is intentionally observed so a 200% text setting or a
  // viewport resize can reveal a now-clipped selected task without changing
  // model state or any Ribbon box geometry.
  createEffect(() => {
    if (!hasReservedTaskRow()) return;
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
        cueSafeViewport(taskScrollport.current, true),
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
      data-ribbon-product-role={props.model.context.productLabel.toLowerCase()}
      data-ribbon-scope={props.model.scope}
      data-ribbon-task-row={hasReservedTaskRow() ? "reserved" : "absent"}
    >
      <section class="ple-app-ribbon__row-frame" data-ribbon-row-frame="top">
        <section
          class="ple-app-ribbon__row ple-app-ribbon__top-bar"
          aria-label="Ribbon navigation"
          data-ribbon-row="top"
          ref={(element): void => {
            tabScrollport.current = element;
            topOverflow.setRow(element);
          }}
        >
          <div class="ple-app-ribbon__context-identity">
            <a class="ple-app-ribbon__brand" href="/" aria-label="Peptidyle home">
              <span class="ple-app-ribbon__brand-mark" aria-hidden="true">
                P
              </span>
              <span class="ple-app-ribbon__brand-word">Peptidyle</span>
            </a>
            <span
              class="ple-app-ribbon__product-role"
              data-product-role={props.model.context.productLabel.toLowerCase()}
            >
              {props.model.context.productLabel}
            </span>
          </div>
          <div class="ple-app-ribbon__context-details">
            <Show when={props.model.context.scopeLabel}>
              {(label) => <span class="ple-app-ribbon__course-scope-label">{label()}</span>}
            </Show>
            <Show when={props.model.context.assessmentLabel}>
              {(label) => <span>{label()}</span>}
            </Show>
            <Show when={props.model.context.assessmentAttemptProgress}>
              {(label) => <span>{label()}</span>}
            </Show>
          </div>
          <nav class="ple-app-ribbon__tabs" aria-label="Ribbon tabs">
            <For each={visibleTabs()}>
              {(control) => <RibbonLink control={control} pendingNavigation={pendingNavigation} />}
            </For>
          </nav>
        </section>
        <RibbonOverflowCues state={topOverflow} />
      </section>
      <span class="ple-app-ribbon__profile-endcap">
        <button
          class="ple-app-ribbon__profile"
          type="button"
          aria-label="Profile"
          title="Profile"
          aria-haspopup="menu"
          aria-expanded={profileMenuOpen() ? "true" : "false"}
          aria-controls="ple-profile-menu"
          data-ribbon-context-control="profile"
          data-ribbon-profile-avatar={
            props.renderProfileAvatar === undefined ? "generic" : "selected"
          }
          ref={(element): void => {
            profileTrigger = element;
          }}
          onClick={() => (profileMenuOpen() ? closeProfileMenu() : openProfileMenu("none"))}
          onKeyDown={handleProfileTriggerKeyDown}
        >
          {props.renderProfileAvatar?.() ?? <RibbonIcon glyph={ribbonGlyphForContext("profile")} />}
        </button>
        <Show when={profileMenuOpen()}>
          <div
            id="ple-profile-menu"
            class="ple-app-ribbon__profile-menu"
            role="menu"
            aria-label="Profile menu"
            ref={(element): void => {
              profileMenu = element;
            }}
            onKeyDown={handleProfileMenuKeyDown}
          >
            <Show when={profileLink()}>
              {(control) => (
                <>
                  <a
                    class="ple-app-ribbon__profile-menu-item"
                    role="menuitem"
                    href={control().href}
                    onClick={() => closeProfileMenu()}
                  >
                    {control().label}
                  </a>
                  <a
                    class="ple-app-ribbon__profile-menu-item"
                    role="menuitem"
                    href="/account-settings"
                    onClick={() => closeProfileMenu()}
                  >
                    Account settings
                  </a>
                </>
              )}
            </Show>
            <button
              class="ple-app-ribbon__profile-menu-item"
              type="button"
              role="menuitem"
              data-ribbon-action={props.model.context.signOutAction.id}
              onClick={(event) => {
                emitRibbonAction(event, props.model.context.signOutAction);
                closeProfileMenu();
              }}
            >
              <RibbonIcon glyph={ribbonGlyphForContext("signOut")} />
              {props.model.context.signOutAction.label}
            </button>
          </div>
        </Show>
      </span>
      {/* taskAreasFor derives task areas from route topology, not admission state;
          server authorization remains the trusted layer (ASVS 8.3.1). */}
      <Show when={hasReservedTaskRow()}>
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
      </Show>
    </section>
  );
}
