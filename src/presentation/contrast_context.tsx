// Account-backed optional contrast presentation; course content and theme identity are unchanged.

import {
  createContext,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  useContext,
  type Accessor,
  type JSX,
} from "solid-js";

import type { PresentationContrast } from "../api/enrollment";
import { useApiRuntime } from "../api/runtime";

export interface PresentationContrastContextValue {
  readonly contrast: Accessor<PresentationContrast>;
  readonly saving: Accessor<boolean>;
  readonly error: Accessor<string | null>;
  readonly setContrast: (contrast: PresentationContrast) => Promise<boolean>;
}

const PresentationContrastContext = createContext<PresentationContrastContextValue>();

export function PresentationContrastProvider(props: {
  readonly children: JSX.Element;
}): JSX.Element {
  const runtime = useApiRuntime();
  const [contrast, setContrastValue] = createSignal<PresentationContrast>("standard");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  onMount(() => {
    void runtime.client.getAccountPresentation().then(
      (preference) => setContrastValue(preference.contrast),
      () => setError("Your saved presentation preference could not be loaded."),
    );
  });

  createEffect(() => {
    document.documentElement.dataset["contrast"] = contrast();
  });
  onCleanup(() => {
    delete document.documentElement.dataset["contrast"];
  });

  async function setContrast(contrast: PresentationContrast): Promise<boolean> {
    setSaving(true);
    setError(null);
    try {
      const saved = await runtime.client.saveAccountPresentation({ contrast });
      setContrastValue(saved.contrast);
      return true;
    } catch {
      setError(
        "The presentation preference could not be saved. Your previous setting remains active.",
      );
      return false;
    } finally {
      setSaving(false);
    }
  }

  const value: PresentationContrastContextValue = { contrast, saving, error, setContrast };
  return (
    <PresentationContrastContext.Provider value={value}>
      <div class="presentation-scope" data-contrast={contrast()}>
        {props.children}
      </div>
    </PresentationContrastContext.Provider>
  );
}

export function usePresentationContrast(): PresentationContrastContextValue {
  const value = useContext(PresentationContrastContext);
  if (value === undefined) throw new Error("PresentationContrastProvider is missing");
  return value;
}
