# Pilot final retained live review

## Verdict

ACCEPTED. An independent, unmodified retained-stack rerun passed the complete
no-email pilot walkthrough. It used the same forced-build command as the
manager run and no volume reset, manual data setup, pointer intervention, or
browser-artifact inspection.

## Scope

The accepted path is limited to the active pilot charter:

- instructor creates a course;
- instructor adds an active local-development student without email;
- instructor creates a corpus-backed Mastery assignment;
- the student completes the keyboard-only take, score, and repeat journey;
- the instructor sees the gradebook summary and two completed run history;
- shared assignment and student-completion outcomes agree.

Email registration, mail delivery, SMTP, mailbox access, and canonical
onboarding remain intentionally outside this acceptance.

## Independent execution

Run on 2026-08-11 from the dirty worktree, without changing its source:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build
```

The runner exited after its normal cleanup. It was allowed to retain the
existing volumes; it did not receive a volume-reset flag or any manual
intervention.

## Public acceptance evidence

The sole retained report is canonical ASCII schema-v2 JSON at
`test-results/ui_walkthrough/ui_walkthrough_seed_42.json`. Its parent directory
is mode `0700`; the regular report file is mode `0600`.

The strict public reader accepted the report with seed 42. It records status
`PASS`, stage `complete`, empty diagnostics, one label-only arrangement
`api-retry-corpus-publication`, and these exact ordered PASS journeys:

| Journey | Public visible outcomes                                      |
| ------- | ------------------------------------------------------------ |
| J11     | course created; course opened                                |
| J12     | local student active                                         |
| J13     | assignment created; catalog problem selected; Mastery policy |
| J1      | feedback; response; retry; submit                            |
| J2      | completion; feedback; fresh practice; submit                 |
| J3      | controls cleared; leave; resume; start                       |
| J4      | back action; completion; controls cleared; submit            |
| J5      | gradebook; score summary; two-run history                    |
| J8      | instructor gradebook; student completion; shared assignment  |

The top-level elapsed time equals the nine journey elapsed values. The public
schema admits no IDs, titles, student identity, email, score value, answer,
credential, trace, screenshot, video, or private state field.

## Lifecycle and retention evidence

After runner exit, `podman ps --all` was empty. The report was the only file
under `test-results` for this run, and no `/private/tmp/ple-ui-walkthrough-*`
directory remained.

The runner cleanup contract invokes `down --remove-orphans`; its source has no
`--volumes` option. This confirms selected-stack cleanup without deleting the
retained PostgreSQL or object-storage volumes needed for the repeatable pilot.

## Limits

This is a real local Podman and Playwright pilot acceptance, not production
email/onboarding, AWS deployment, all-question-family, or release acceptance.
