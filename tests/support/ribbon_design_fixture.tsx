// ribbon_design_fixture.tsx - static, treatment-neutral design laboratory using the real Ribbon.

import { For, Show, type JSX } from "solid-js";

import {
  COURSE_THEME_OPTIONS,
  courseThemeStyle,
} from "../../src/features/course_appearance/course_theme_registry";
import { AppRibbon, RibbonIcon } from "../../src/ribbon/app_ribbon";
import type { RibbonModel } from "../../src/ribbon/ribbon_contract";
import { RIBBON_GLYPH_IDS, type RibbonGlyphId } from "../../src/ribbon/ribbon_icons";
import "../design/ribbon_design_lab.css";
import "../design/ribbon_treatment_fieldstation.css";
import "../design/ribbon_treatment_atlas.css";
import {
  RIBBON_DESIGN_DECISION,
  RIBBON_DESIGN_SCHEMAS,
  RIBBON_DESIGN_STATE_SPECIMENS,
  RIBBON_DESIGN_TREATMENTS,
  type RibbonDesignAvailability,
  type RibbonDesignTreatment,
} from "./ribbon_design_models";

export {
  RIBBON_DESIGN_DECISION,
  RIBBON_DESIGN_SCHEMAS,
  RIBBON_DESIGN_STATE_SPECIMENS,
  RIBBON_DESIGN_TREATMENTS,
};

const GLYPH_ATLAS_LABELS: Readonly<Record<RibbonGlyphId, string>> = {
  "graduation-cap": "Courses",
  "book-open": "Question Library",
  "clipboard-list": "Assignments",
  users: "Students",
  "table-list": "Gradebook",
  gear: "Course Setup",
  "pen-to-square": "Attempt",
  "file-pen": "My Question Drafts",
  star: "Starred",
  eye: "Watched",
  "list-check": "Assignment Questions",
  "user-graduate": "Assignment Student View",
  palette: "Appearance",
  "arrow-left": "Back to Assignments",
  "circle-user": "Account context",
  "right-from-bracket": "Sign out",
};

const ICON_ONLY_VISUAL_SPECIMENS = [
  { glyph: "star", label: "Starred" },
  { glyph: "eye", label: "Watched" },
  { glyph: "arrow-left", label: "Back to Assignments" },
] as const satisfies ReadonlyArray<{ readonly glyph: RibbonGlyphId; readonly label: string }>;

function withheldAdmissions(
  model: RibbonModel,
): ReadonlyArray<{ readonly label: string; readonly availability: RibbonDesignAvailability }> {
  return [...model.tabs, ...model.taskAreas.flatMap((area) => area.controls)]
    .filter((control) => control.availability !== "Available")
    .map((control) => ({ label: control.label, availability: control.availability }));
}

function Panel(props: {
  readonly kind: "schema" | "specimen" | "theme";
  readonly name: string;
  readonly label: string;
  readonly model: RibbonModel;
  readonly themeStyle?: string;
}): JSX.Element {
  const withheld = (): ReturnType<typeof withheldAdmissions> => withheldAdmissions(props.model);
  return (
    <article
      class="ple-ribbon-design-lab__panel"
      data-ribbon-design-panel={props.kind}
      data-ribbon-design-name={props.name}
      style={props.themeStyle}
    >
      <h3 class="ple-ribbon-design-lab__subheading">{props.label}</h3>
      <Show when={withheld().length > 0}>
        <p
          class="ple-ribbon-design-lab__admission-note"
          data-ribbon-withheld={withheld()
            .map((control) => control.availability)
            .join(" ")}
        >
          Withheld controls:{" "}
          {withheld()
            .map((control) => `${control.label} (${control.availability})`)
            .join(", ")}
        </p>
      </Show>
      <AppRibbon model={props.model} />
    </article>
  );
}

function Treatment(props: { readonly treatment: RibbonDesignTreatment }): JSX.Element {
  const isSelected = (): boolean => props.treatment === RIBBON_DESIGN_DECISION.selectedTreatment;
  return (
    <section
      class="ple-ribbon-design-lab__treatment"
      data-ribbon-treatment={props.treatment}
      data-ribbon-design-decision={isSelected() ? "selected" : "retained-alternative"}
    >
      <header>
        <h1 class="ple-ribbon-design-lab__heading">
          {props.treatment === "fieldstation" ? "Fieldstation" : "Atlas"} Ribbon treatment
        </h1>
        <p class="ple-ribbon-design-lab__caption">
          {isSelected()
            ? "Selected production direction."
            : "Retained credible alternative for comparison."}
        </p>
      </header>
      <section
        class="ple-ribbon-design-lab__schema-grid"
        aria-label="Scope and Product Role schemas"
      >
        <For each={Object.entries(RIBBON_DESIGN_SCHEMAS)}>
          {([name, model]) => <Panel kind="schema" name={name} label={name} model={model} />}
        </For>
      </section>
      <section class="ple-ribbon-design-lab__specimen-grid" aria-label="Ribbon state specimens">
        <For each={Object.entries(RIBBON_DESIGN_STATE_SPECIMENS)}>
          {([name, model]) => <Panel kind="specimen" name={name} label={name} model={model} />}
        </For>
      </section>
      <section class="ple-ribbon-design-lab__theme-grid" aria-label="Course theme specimens">
        <For each={COURSE_THEME_OPTIONS}>
          {(option) => (
            <Panel
              kind="theme"
              name={option.id}
              label={option.tokens.name}
              model={RIBBON_DESIGN_STATE_SPECIMENS.selectedAndUnselected}
              themeStyle={courseThemeStyle(option.tokens)}
            />
          )}
        </For>
      </section>
    </section>
  );
}

/** Renders both complete visual treatments with no application, router, or session dependency. */
export function RibbonDesignFixture(): JSX.Element {
  return (
    <main class="ple-ribbon-design-lab" aria-label="Ribbon design laboratory">
      <aside class="ple-ribbon-design-lab__decision" data-ribbon-design-decision-record="true">
        <h1>Selected direction: Fieldstation</h1>
        <p>Retained alternative: Atlas. {RIBBON_DESIGN_DECISION.rationale}</p>
        <h2>Production non-negotiables</h2>
        <ul>
          <For each={RIBBON_DESIGN_DECISION.productionNonNegotiables}>
            {(nonNegotiable) => <li>{nonNegotiable}</li>}
          </For>
        </ul>
        <p>{RIBBON_DESIGN_DECISION.m9bBoundary}</p>
      </aside>
      <section
        class="ple-ribbon-design-lab__glyph-atlas"
        aria-labelledby="ribbon-glyph-atlas-heading"
      >
        <h2 id="ribbon-glyph-atlas-heading">Ribbon glyph atlas</h2>
        <p>Bundled production glyphs paired with their semantic control labels.</p>
        <ul>
          <For each={RIBBON_GLYPH_IDS}>
            {(glyph) => (
              <li data-ribbon-glyph-atlas-entry={glyph}>
                <RibbonIcon glyph={glyph} />
                <span>{GLYPH_ATLAS_LABELS[glyph]}</span>
              </li>
            )}
          </For>
        </ul>
        <section
          class="ple-ribbon-design-lab__icon-only-specimens"
          aria-labelledby="ribbon-icon-only-specimens-heading"
        >
          <h3 id="ribbon-icon-only-specimens-heading">Narrow-phone icon-only visual specimens</h3>
          <p>
            These are glyph and naming specimens only; they do not assert that a currently withheld
            destination is available or routed.
          </p>
          <div>
            <For each={ICON_ONLY_VISUAL_SPECIMENS}>
              {(specimen) => (
                <span
                  class="ple-app-ribbon__link ple-ribbon-design-lab__icon-only-specimen"
                  aria-label={specimen.label}
                  title={specimen.label}
                  data-ribbon-icon-only-safe="true"
                  data-ribbon-icon-only-specimen={specimen.glyph}
                >
                  <RibbonIcon glyph={specimen.glyph} />
                  <span class="ple-app-ribbon__control-label">{specimen.label}</span>
                </span>
              )}
            </For>
          </div>
        </section>
      </section>
      <For each={RIBBON_DESIGN_TREATMENTS}>
        {(treatment) => <Treatment treatment={treatment} />}
      </For>
    </main>
  );
}
