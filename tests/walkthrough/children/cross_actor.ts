// cross_actor.ts - fixed private-state J8 derivation child.

import { passedJ8CrossActorFragment } from "../../playwright/simulator/j8_cross_actor_fragment";
import {
  appendJourneyState,
  readJourneyStatePrefix,
} from "../../playwright/simulator/journey_state";

const path = process.env.PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE;
if (path === undefined) process.exitCode = 1;
else {
  try {
    const prefix = readJourneyStatePrefix(path);
    if (prefix.length !== 5 || prefix[3]?.journey !== "J4" || prefix[4]?.journey !== "J5")
      throw new Error("invalid private state");
    appendJourneyState(path, passedJ8CrossActorFragment(prefix[3], prefix[4], 0));
  } catch {
    process.exitCode = 1;
  }
}
