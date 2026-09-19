// Narrow Sysadmin control for the existing Blueprint Course ID promotion endpoint.
import { Show, createSignal, type JSX } from "solid-js";
import type {
  BlueprintPromotion as Promotion,
  BlueprintStewardshipClient,
} from "../../api/blueprint_stewardship";
import { BlueprintCourseConflictError } from "../../api/http_client";
import { parseBlueprintCourseId } from "../../navigation/public_route";

interface Props {
  readonly client: BlueprintStewardshipClient;
}

/** Mounted only in the Sysadmin home; the server independently enforces Sysadmin authorization. */
export function BlueprintPromotion(props: Props): JSX.Element {
  const [blueprintCourseId, setBlueprintCourseId] = createSignal("");
  const [loaded, setLoaded] = createSignal<{
    readonly blueprintCourseId: string;
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
    const parsed = parseBlueprintCourseId(blueprintCourseId());
    if (parsed === null) {
      setFailed(true);
      setNotice("Enter a valid Blueprint Course ID, such as BP7K3M2QAF.");
      return;
    }
    setBusy(true);
    try {
      setLoaded({
        blueprintCourseId: parsed,
        promotion: await props.client.getBlueprintPromotion(parsed),
      });
    } catch {
      setFailed(true);
      setNotice(
        "Blueprint promotion could not load. Check the Blueprint Course ID and your session, then try again.",
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
        current.blueprintCourseId,
        !current.promotion.promoted,
        current.promotion.blueprintEditNumber,
      );
      setLoaded({ blueprintCourseId: current.blueprintCourseId, promotion });
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
        <label for="blueprint-promotion-id">Blueprint Course ID</label>
        <input
          id="blueprint-promotion-id"
          value={blueprintCourseId()}
          placeholder="BP7K3M2QAF"
          disabled={busy()}
          onInput={(event) => {
            setBlueprintCourseId(event.currentTarget.value);
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
              {current().blueprintCourseId}:{" "}
              {current().promotion.promoted ? "Promoted" : "Not Promoted"}
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
