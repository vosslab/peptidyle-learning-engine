# WP-O2 live Playwright configuration

## Current disposition

The 2026-08-11 WP-O2 smoke acceptance is historical. It validates the prior
real-gateway `/health` check. The strengthened instructor copy/paste workflow,
refactored explicit child-input path, and current live teaching acceptance
passed on 2026-08-12 under
[WP-HG1 human-guidance workflow](../peptidyle-walkthrough-plan.md#wp-hg1-contract).

## Current contract

- `playwright.config.ts` remains the ordinary repository configuration. It no
  longer activates a walkthrough mode through `PLE_UI_WALKTHROUGH_*`, and
  `ui_walkthrough_live_config.ts` no longer exists.
- The Python runner creates one private config file and invokes Playwright with
  `--config PATH`. That config imports
  `tests/playwright/ui_walkthrough_config_factory.ts` and calls
  `createUiWalkthroughConfig(inputPath)`.
- The factory reads one explicit private `walkthrough-inputs.json` before
  Chromium starts. It requires ASCII canonical JSON, schema version 1, a
  mode-0700 parent, a mode-0600 regular input file, a validated loopback
  origin, and exact stage-specific fields.
- The setup and learner stages receive credentials only as validated private
  file paths. Specs read a role credential at the visible local-login action;
  the config boundary does not export credentials, answer material, or raw
  child output through environment variables or reports.
- The factory sets the absolute walkthrough test directory, a private artifact
  directory beside the runner state, the selected gateway `baseURL`, and a
  single `ui-walkthrough` project. Fixed Node children likewise use
  `--inputs PATH` for arrangement, cross-actor evidence, and report rendering.
- The current live suite covers visible instructor setup followed by J1--J5
  and J8. J13 uses the published catalog to visibly copy each exact
  `AAA-BBBB` Question ID and paste the growing list into the add-by-ID control;
  it does not read or write the clipboard through page evaluation and does not
  extract a UUID from the DOM.

## Current validation and acceptance

Permanent focused tests validate the private-input schema and metadata, strict
Playwright configuration, keyboard-visible student and instructor interactions,
and the J13 copy/paste contract. They are offline, deterministic behavior
checks, not a substitute for the live stack.

One-time WP-HG1 evidence passed: the rebuilt canonical run covered J13 and
J1--J8 with the Podman WebWork renderer, clipboard permission, and a redacted
report. The clean-stack screenshot capture produced and inspected the exact
eleven public images; the separate all-eight Chapter 1 browser oracle passed;
and independent architecture, security, and HCI reviews closed their settled
boundaries. These remain recorded one-time evidence, not permanent networked tests.

## Separate onboarding follow-up

Source inspection finds the inactive simulator-only onboarding preflight still
uses `PLE_UI_WALKTHROUGH_ONBOARDING_MAILBOX_READY` and
`PLE_UI_WALKTHROUGH_ONBOARDING_DELIVERED_LINK_READY`. Those are outside the
Python walkthrough runner and outside this no-email pilot. The active
[WP-RC8 production identity package](../active/release_completion_plan.md#wp-rc8-complete-production-identity-and-enrollment) must either replace those
operator confirmations with an explicit interface or document their bounded
deployment contract before onboarding is accepted; WP-O2 does not silently
adopt them.

## Historical evidence

The earlier elevated runner command passed against the public IPv4 `/health`
origin with mock preview disabled and cleaned up its containers. It remains
historical browser-smoke evidence only; see the independent
[WP-O2 review](../audits/wp_o2_live_playwright_review.md).
