# WP-S2 schema-v2 J5/J8 tail state review

## Verdict

**ACCEPTED OFFLINE.** This accepts only the isolated schema-v2 J5/J8 state and report tail; it
does not claim retained-stack or full-walkthrough acceptance.

The private-state and public-report boundaries are otherwise suitably closed:

- `v2_j5_j8_state.ts` accepts J5 only after the exact ordered J11, J12, J13, J1, J2, J3, J4 prefix; J8 then accepts only that J5-closed prefix.  Every non-setup assignment binding must equal J13's assignment ID and every course binding must equal J13's course ID.
- J5 has only its three closed milestone codes.  J8 is derived from descriptor-parsed state, not caller-supplied observations, and has only its three closed cross-actor codes.  Neither tail record nor the schema-v2 report can contain score values, assignment titles, learner data, or run data.
- The tail and final parser reject inherited, hidden, symbol, and accessor properties; they also reject accessor-bearing arrays.  Canonical ASCII JSON with the one final newline rejects duplicate keys, reordered keys, CR/LF variants, and non-canonical encodings.
- The private path is exact `journeys.json` beneath a mode-0700, no-follow directory; its file must be a regular mode-0600 file of at most 4096 bytes.  Open descriptors are rechecked against the path parent, append uses the same descriptor and `fsyncSync`, and failed J8/report child processes leave stdout empty.

## Resolved close-order finding

The original review found that J5 appended before its browser context closed. The repair moves
that ordering into `closeThenAppendV2J5State`: it awaits the context-close promise before opening
and appending state. The J5 browser spec calls that helper from its `finally` only when visible
evidence exists. Its new hostile regression rejects context closure and proves the J11-through-J4
prefix remains byte-for-byte unchanged. A close failure therefore cannot leave a durable J5 PASS.

## Verification

The following offline focused gates passed:

```text
npx playwright test tests/playwright/simulator/v2_j5_j8_state.spec.ts tests/playwright/simulator/v2_visible_outcome_report.spec.ts tests/playwright/simulator/instructor_gradebook_j5.spec.ts --reporter=line
15 passed (492ms)

python3 -m pytest -q tests/test_walked_journey_baseline_v2.py tests/test_ui_walkthrough_harness_independence.py
20 passed in 0.06s

npx tsc --noEmit
exit 0 (no diagnostics)
```

No Podman process was started for this review.

## Compatibility note

The new tail imports only schema-v2 modules and introduces no new call from the tail into the
legacy v1 state/report modules.  This shared worktree already contains staged changes to the
historical `journey_state.ts`, `visible_outcome_report.ts`, and v1 child entrypoints; they predate
this review and are not attributable to the new J5/J8 tail. The resolved close-order repair is
confined to the schema-v2 J5 spec, tail module, and focused test.
