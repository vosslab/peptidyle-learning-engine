// Narrow Sysadmin control for the existing reference-addressed promotion endpoint.
import { Show, createSignal, type JSX } from "solid-js";
import type {
  BlueprintPromotion as Promotion,
  BlueprintStewardshipClient,
} from "../../api/blueprint_stewardship";
import { BlueprintCourseConflictError } from "../../api/http_client";
import { parseBlueprintCourseReference } from "../../navigation/public_route";

interface Props {
  readonly client: BlueprintStewardshipClient;
}

/** Mounted only in the Sysadmin home; the server independently enforces Sysadmin authorization. */
export function BlueprintPromotion(props: Props): JSX.Element {
  const [reference, setReference] = createSignal("");
  const [loaded, setLoaded] = createSignal<{
    readonly reference: string;
    readonly promotion: Promotion;
  }>();
  const [busy, setBusy] = createSignal(false);
  const [notice, setNotice] = createSignal("");
  const [failed, setFailed] = createSignal(false);

  async function load(): Promise<void> {
    if (busy()) return;
    setLoaded(undefined);
    setNotice("");
    setFailed(false);
    const parsed = parseBlueprintCourseReference(reference());
    if (parsed === null) {
      setFailed(true);
      setNotice("Enter a valid Blueprint Course reference, such as BP7K3M2QAF.");
      return;
    }
    setBusy(true);
    try {
      setLoaded({ reference: parsed, promotion: await props.client.getBlueprintPromotion(parsed) });
    } catch {
      setFailed(true);
      setNotice(
        "Blueprint promotion could not load. Check the reference and your session, then try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function toggle(): Promise<void> {
    const current = loaded();
    if (current === undefined || busy()) return;
    setBusy(true);
    setNotice("");
    setFailed(false);
    try {
      const promotion = await props.client.setBlueprintPromotion(
        current.reference,
        !current.promotion.promoted,
        current.promotion.metadataEtag,
      );
      setLoaded({ reference: current.reference, promotion });
      setNotice(
        promotion.promoted
          ? "Blueprint Course is now Promoted."
          : "Blueprint Course is no longer Promoted.",
      );
    } catch (error: unknown) {
      setLoaded(undefined);
      setFailed(true);
      setNotice(
        error instanceof BlueprintCourseConflictError
          ? "Blueprint metadata changed elsewhere. Load the current promotion before trying again."
          : "Blueprint promotion could not be confirmed. Load its current state before trying again.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="auth-panel" aria-labelledby="blueprint-promotion-heading" aria-busy={busy()}>
      <h2 id="blueprint-promotion-heading">Blueprint Course promotion</h2>
      <p>Manage the Promoted flag used in Instructor Blueprint Course search.</p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void load();
        }}
      >
        <label for="blueprint-promotion-reference">Blueprint Course reference</label>
        <input
          id="blueprint-promotion-reference"
          value={reference()}
          placeholder="BP7K3M2QAF"
          disabled={busy()}
          onInput={(event) => {
            setReference(event.currentTarget.value);
            setLoaded(undefined);
            setNotice("");
          }}
        />
        <button type="submit" class="quiet-action" disabled={busy()}>
          Load promotion
        </button>
      </form>
      <Show when={loaded()}>
        {(current) => (
          <>
            <p>
              {current().reference}: {current().promotion.promoted ? "Promoted" : "Not Promoted"}
            </p>
            <button
              type="button"
              class="quiet-action"
              disabled={busy()}
              aria-pressed={current().promotion.promoted}
              onClick={() => void toggle()}
            >
              {current().promotion.promoted ? "Remove Promoted flag" : "Mark as Promoted"}
            </button>
          </>
        )}
      </Show>
      <Show when={notice()}>
        <p role={failed() ? "alert" : "status"}>{notice()}</p>
      </Show>
    </section>
  );
}
