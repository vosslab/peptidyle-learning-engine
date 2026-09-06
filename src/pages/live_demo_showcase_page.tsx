// live_demo_showcase_page.tsx - deployment-gated Ribbon and mailer developer showcase.

import { A } from "@solidjs/router";
import { Match, Switch, createMemo, createSignal, onMount, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { AppRibbon } from "../ribbon/app_ribbon";
import {
  COURSE_INSTRUCTOR_SHOWCASE_TASK_IDS,
  COURSE_INSTRUCTOR_SHOWCASE_TAB_IDS,
  DEFAULT_RIBBON_SHOWCASE_SELECTION,
  courseInstructorRibbonShowcaseModel,
  type CourseInstructorShowcaseTabId,
  type CourseInstructorShowcaseTaskId,
  type RibbonShowcaseSelection,
} from "../ribbon/ribbon_showcase_model";
import "./live_demo_showcase_page.css";

type ShowcaseGateState = "checking" | "ready" | "unavailable";

const MAILER_PREVIEW_COMMAND =
  "source source_me.sh && python3 launchers/send_invitations.py output-email/roster_export.json";
const MAILER_SEND_COMMAND = [
  "source source_me.sh && python3 launchers/send_invitations.py \\",
  "  output-email/roster_export.json --send --limit 5",
].join("\n");

function isShowcaseTab(id: string): id is CourseInstructorShowcaseTabId {
  return COURSE_INSTRUCTOR_SHOWCASE_TAB_IDS.some((candidate) => candidate === id);
}

function isShowcaseTask(id: string): id is CourseInstructorShowcaseTaskId {
  return COURSE_INSTRUCTOR_SHOWCASE_TASK_IDS.some((candidate) => candidate === id);
}

/** Authenticated, deployment-gated preview of the real Ribbon component and attended mailer path. */
export function LiveDemoShowcasePage(): JSX.Element {
  const runtime = useApplicationApi();
  const [gate, setGate] = createSignal<ShowcaseGateState>("checking");
  const [selection, setSelection] = createSignal<RibbonShowcaseSelection>(
    DEFAULT_RIBBON_SHOWCASE_SELECTION,
  );
  const [previewStatus, setPreviewStatus] = createSignal(
    "Select a Ribbon control to inspect its selected state.",
  );
  const model = createMemo(() => courseInstructorRibbonShowcaseModel(selection()));

  async function checkShowcaseAvailability(): Promise<void> {
    setGate("checking");
    try {
      // ASVS 6.3.4: reuse the documented seeded-demo pathway; this route creates no second login.
      await runtime.client.listSeededDemoAccounts();
      setGate("ready");
    } catch {
      setGate("unavailable");
    }
  }

  function selectPreviewControl(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const link = target.closest<HTMLElement>("[data-ribbon-control]");
    if (link === null) return;
    const id = link.dataset.ribbonControl;
    if (id === undefined) return;

    // This laboratory activates visual selection only. It cannot navigate to an unbacked route.
    event.preventDefault();
    const label = link.textContent?.trim() || id;
    // ASVS 2.2.1: accept only the closed structural-fixture control vocabulary.
    if (isShowcaseTab(id)) {
      setSelection((current) => ({ ...current, tab: id }));
    } else if (isShowcaseTask(id)) {
      setSelection((current) => ({ ...current, task: id }));
    } else {
      return;
    }
    setPreviewStatus(`${label} selected in the structural preview.`);
  }

  function previewSignOut(event: Event): void {
    event.stopPropagation();
    setPreviewStatus("Sign out is visible here as a preview action; your demo session stays open.");
  }

  onMount(() => void checkShowcaseAvailability());

  return (
    <section class="page live-demo-showcase-page" data-route-surface="liveDemoShowcase">
      <p class="eyebrow">Developer showcase</p>
      <h1>Ribbon implementation and invitation email trial</h1>
      <p class="page-lede">
        This page exposes the current visual implementation without claiming that the teaching
        destinations have server handlers.
      </p>

      <Switch>
        <Match when={gate() === "checking"}>
          <p class="calm-status" role="status" aria-live="polite">
            Confirming that this deployment enables the developer showcase...
          </p>
        </Match>
        <Match when={gate() === "unavailable"}>
          <section class="inline-error" role="alert">
            <h2>Developer showcase unavailable</h2>
            <p>This route is enabled only by the disposable seeded Live Demo deployment.</p>
            <button
              class="quiet-action"
              type="button"
              onClick={() => void checkShowcaseAvailability()}
            >
              Try again
            </button>
          </section>
        </Match>
        <Match when={gate() === "ready"}>
          <section aria-labelledby="ribbon-showcase-heading">
            <h2 id="ribbon-showcase-heading">Populated Instructor Ribbon</h2>
            <aside class="live-demo-showcase-note">
              <p>
                <strong>Structural fixture:</strong> these controls demonstrate the real production
                Ribbon component, catalog labels, icons, density, overflow, and selected states.
              </p>
              <p>
                Activating a control changes this preview only. It does not make the destination
                available or change the production capability registry.
              </p>
            </aside>
            <div
              class="live-demo-ribbon-preview"
              on:click={selectPreviewControl}
              on:ple-ribbon-action={previewSignOut}
            >
              <AppRibbon model={model()} />
              <p class="live-demo-showcase-status" role="status" aria-live="polite">
                {previewStatus()}
              </p>
            </div>
          </section>

          <section class="live-demo-mailer-panel" aria-labelledby="mailer-showcase-heading">
            <h2 id="mailer-showcase-heading">Try the attended invitation email hack</h2>
            <p>
              The browser cannot send local mail. The existing launcher intentionally hands each
              message to visible macOS Mail.app composition, with dry-run as the default.
            </p>
            <p>First preview your private export:</p>
            <pre>
              <code>{MAILER_PREVIEW_COMMAND}</code>
            </pre>
            <p>After inspecting the preview, send no more than five while you remain at the Mac:</p>
            <pre>
              <code>{MAILER_SEND_COMMAND}</code>
            </pre>
            <p>
              Recipient domains must be allowed by <code>invitation_mailer.yaml</code>; signup URLs
              must use HTTPS. Full input and resend details are in <code>docs/USAGE.md</code>.
            </p>
          </section>
          <p>
            <A href="/sign-in">Choose another demo Account</A>
          </p>
        </Match>
      </Switch>
    </section>
  );
}
