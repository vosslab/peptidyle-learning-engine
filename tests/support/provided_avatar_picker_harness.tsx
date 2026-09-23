// provided_avatar_picker_harness.tsx - browser-only mount of the real avatar picker.

import { createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";

import "../../src/browser_environment";

import { ProvidedAvatarPicker } from "../../src/features/profile_avatar/provided_avatar_picker";

function ProvidedAvatarPickerHarness(): JSX.Element {
  const [currentAvatarId, setCurrentAvatarId] = createSignal<string>("amber-arch");

  return (
    <main>
      <h1>Provided avatar picker evidence</h1>
      <ProvidedAvatarPicker
        currentAvatarId={currentAvatarId()}
        onSelect={(avatarId) => setCurrentAvatarId(avatarId)}
      />
      <p aria-live="polite" data-provided-avatar-picker-current={currentAvatarId()}>
        Selected avatar: {currentAvatarId()}
      </p>
    </main>
  );
}

export function mountProvidedAvatarPickerHarness(target: HTMLElement): void {
  render(() => <ProvidedAvatarPickerHarness />, target);
}
