// appearance_owner.tsx - the sole document-level Theme and display-mode owner.

import {
  createEffect,
  createMemo,
  createResource,
  createRenderEffect,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import type { AccountId } from "../../generated/api/AccountId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { Theme } from "../../generated/api/Theme";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import {
  useRouteScopeData,
  useRouteScopeIdentityAccessor,
  useRouteScopeLoadState,
} from "../ribbon/route_scope_context";
import { resolveDisplayMode, resolveTheme } from "./appearance_rules";
import {
  currentAccountSettingsUpdate,
  settingsForCurrentAccount,
  type AccountScopedSettings,
} from "./account_scoped_settings";
import {
  AppearanceContext,
  type AppearanceContextValue,
  type AppearanceSettingsState,
  type ResolvedAppearance,
} from "./appearance_context";
import { courseRouteView } from "./theme_context";
import { themeStyle, themeTokens } from "./theme_registry";

export interface AppearanceOwnerProps {
  readonly children: JSX.Element;
}

interface CourseThemePreview {
  readonly courseInstanceId: CourseInstanceId;
  readonly theme: Theme;
}

function browserDisplayMode(): DisplayMode {
  return typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function rootStyleEntries(style: string): ReadonlyArray<readonly [string, string]> {
  return style.split(";").flatMap((entry) => {
    const [name, value] = entry.split(":", 2);
    return name === undefined || value === undefined ? [] : [[name.trim(), value.trim()] as const];
  });
}

/**
 * Resolves appearance from current route, Account settings, and browser preference.
 * It is the only component that writes Theme variables, display mode, and color scheme to html.
 */
export function AppearanceOwner(props: AppearanceOwnerProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const routeData = useRouteScopeData();
  const routeScopeIdentity = useRouteScopeIdentityAccessor();
  const routeScopeLoadState = useRouteScopeLoadState();
  const [browserMode, setBrowserMode] = createSignal<DisplayMode>(browserDisplayMode());
  const [browserMedia, setBrowserMedia] = createSignal<MediaQueryList>();
  const authenticatedAccountId = createMemo(() => {
    const state = session.state();
    return state.kind === "authenticated" ? state.session.account.id : undefined;
  });
  const signedInInstructor = createMemo(() => {
    const state = session.state();
    return state.kind === "authenticated" && state.session.account.userRole === "instructor";
  });
  const [settingsLoad, { mutate: setSettingsLoad }] = createResource(
    authenticatedAccountId,
    async (accountId): Promise<AccountScopedSettings> => ({
      accountId,
      settings: await applicationApi.client.getAccountSettings(),
    }),
  );
  const settings = createMemo(() =>
    settingsForCurrentAccount(settingsLoad(), authenticatedAccountId()),
  );
  const routeTheme = createMemo(() => {
    const data = routeData();
    if (data === undefined) return undefined;
    switch (data.kind) {
      case "course":
        return courseRouteView(data).appearance.theme;
      case "assessmentAttempt":
        return data.context.course.theme;
      case "assessmentAttemptHistory":
        return data.history.course.theme;
    }
  });
  const currentCourseInstanceId = createMemo(() => {
    const data = routeData();
    return data?.kind === "course" ? courseRouteView(data).summary.id : undefined;
  });
  const [coursePreview, setCoursePreview] = createSignal<CourseThemePreview>();
  const [busy, setBusy] = createSignal(false);
  const [mutationError, setMutationError] = createSignal<string>();
  const presentCourseTheme = (theme: Theme | undefined): void => {
    const courseInstanceId = currentCourseInstanceId();
    if (theme === undefined || courseInstanceId === undefined) {
      setCoursePreview(undefined);
      return;
    }
    setCoursePreview({ courseInstanceId, theme });
  };
  createEffect(() => {
    if (coursePreview()?.courseInstanceId !== currentCourseInstanceId())
      setCoursePreview(undefined);
  });
  createEffect(() => {
    authenticatedAccountId();
    setBusy(false);
    setMutationError(undefined);
  });

  const displayModePreference = createMemo(() =>
    authenticatedAccountId() === undefined ? null : (settings()?.displayModePreference ?? null),
  );
  const appearance = createMemo<ResolvedAppearance>(() => {
    const preview = coursePreview();
    const courseTheme =
      preview !== undefined && preview.courseInstanceId === currentCourseInstanceId()
        ? preview.theme
        : routeTheme();
    const theme = resolveTheme({
      courseTheme,
      instructorPersonalTheme: settings()?.personalTheme ?? undefined,
      signedInInstructor: signedInInstructor(),
    });
    const mode = resolveDisplayMode(displayModePreference(), browserMode());
    return { theme, mode, tokens: themeTokens(theme, mode) };
  });
  const settingsState = createMemo<AppearanceSettingsState>(() => {
    if (authenticatedAccountId() === undefined) return "idle";
    if (settingsLoad.loading) return "loading";
    return settingsLoad.error === undefined ? "ready" : "error";
  });
  const currentAppearanceCanPaint = createMemo(() => {
    if (session.state().kind === "loading") return false;
    if (authenticatedAccountId() === undefined) return true;
    if (settings() === undefined && settingsLoad.error === undefined) return false;
    const scope = routeScopeIdentity();
    if (scope.kind === "product" || scope.kind === "invalid") return true;
    return routeData() !== undefined || routeScopeLoadState() === "rejected";
  });
  const [readyAccountId, setReadyAccountId] = createSignal<AccountId>();
  createEffect(() => {
    const accountId = authenticatedAccountId();
    if (accountId === undefined) {
      setReadyAccountId(undefined);
      return;
    }
    if (currentAppearanceCanPaint()) setReadyAccountId(accountId);
  });
  const initialAppearanceReady = createMemo(() => {
    if (session.state().kind === "loading") return false;
    const accountId = authenticatedAccountId();
    return accountId === undefined || readyAccountId() === accountId || currentAppearanceCanPaint();
  });

  onMount(() => setBrowserMedia(window.matchMedia("(prefers-color-scheme: dark)")));
  createEffect(() => {
    const media = browserMedia();
    if (media === undefined || displayModePreference() !== null) return;
    const updateBrowserMode = (): void => {
      setBrowserMode(media.matches ? "dark" : "light");
    };
    updateBrowserMode();
    media.addEventListener("change", updateBrowserMode);
    onCleanup(() => media.removeEventListener("change", updateBrowserMode));
  });

  createRenderEffect(() => {
    if (typeof document === "undefined") return;
    const resolved = appearance();
    const root = document.documentElement;
    const entries = rootStyleEntries(themeStyle(resolved.tokens));
    for (const [name, value] of entries) root.style.setProperty(name, value);
    root.dataset.theme = resolved.theme;
    root.dataset.displayMode = resolved.mode;
    root.style.colorScheme = resolved.mode;
    return;
  });

  const context: AppearanceContextValue = {
    appearance,
    initialAppearanceReady,
    settings,
    settingsState,
    busy,
    error: mutationError,
    async updateTimeZone(timeZone): Promise<void> {
      const accountId = authenticatedAccountId();
      if (accountId === undefined) return;
      setBusy(true);
      setMutationError(undefined);
      try {
        const nextSettings = await applicationApi.client.updateAccountSettings({ timeZone });
        const update = currentAccountSettingsUpdate(
          accountId,
          authenticatedAccountId(),
          nextSettings,
        );
        if (update !== undefined) setSettingsLoad(update);
      } catch (error: unknown) {
        if (authenticatedAccountId() === accountId) {
          setMutationError(error instanceof Error ? error.message : "Time zone could not save.");
        }
        throw error;
      } finally {
        if (authenticatedAccountId() === accountId) setBusy(false);
      }
    },
    async updateDisplayModePreference(preference): Promise<void> {
      const accountId = authenticatedAccountId();
      if (accountId === undefined) return;
      setBusy(true);
      setMutationError(undefined);
      try {
        const nextSettings = await applicationApi.client.updateDisplayModePreference({
          displayModePreference: preference,
        });
        const update = currentAccountSettingsUpdate(
          accountId,
          authenticatedAccountId(),
          nextSettings,
        );
        if (update !== undefined) setSettingsLoad(update);
      } catch (error: unknown) {
        if (authenticatedAccountId() === accountId) {
          setMutationError(
            error instanceof Error ? error.message : "Appearance preference could not save.",
          );
        }
        throw error;
      } finally {
        if (authenticatedAccountId() === accountId) setBusy(false);
      }
    },
    async updatePersonalTheme(theme): Promise<void> {
      const accountId = authenticatedAccountId();
      if (accountId === undefined) return;
      setBusy(true);
      setMutationError(undefined);
      try {
        const nextSettings = await applicationApi.client.updateInstructorPersonalTheme({ theme });
        const update = currentAccountSettingsUpdate(
          accountId,
          authenticatedAccountId(),
          nextSettings,
        );
        if (update !== undefined) setSettingsLoad(update);
      } catch (error: unknown) {
        if (authenticatedAccountId() === accountId) {
          setMutationError(
            error instanceof Error ? error.message : "Personal theme could not save.",
          );
        }
        throw error;
      } finally {
        if (authenticatedAccountId() === accountId) setBusy(false);
      }
    },
    presentCourseTheme,
  };

  return <AppearanceContext.Provider value={context}>{props.children}</AppearanceContext.Provider>;
}
