# Student page visuals

This document preserves the intended student and access-evidence contract alongside historical
screenshot references. Its fictional students and sample course data describe a former deterministic
demo fixture, not the current built-app Browser Surface. Browser behavior and no-transport assertions
remain the authority for access control; retained screenshots show historical student-visible
composition and establish no current acceptance.

## Evidence contract

The historical student captures used these exact CSS-pixel viewports. The current canonical capture
profiles and closed artifact list live in [SCREENSHOT_CONTRACT.md](SCREENSHOT_CONTRACT.md) and
`docs/screenshots/current_capture_manifest.json`; the percentages below remain planning weights,
not test quotas or telemetry targets.

| Viewport    | Aspect                | Planning weight |
| ----------- | --------------------- | --------------- |
| 1280 by 800 | 16:10 desktop profile | 40%             |
| 800 by 1280 | 10:16 portrait tablet | 30%             |
| 393 by 852  | iPhone Pro aspect     | 20%             |
| 800 by 800  | square                | 10%             |

Current canonical captures include an allowed Student surface. Any later access-denial capture must
show composition and visible denial without claiming that pixels prove authorization. Its evidence
also needs no-transport assertions that the denied Server Route returned no Instructor payload,
plus direct route probes for the same session.

The denial boundary is one centrally derived, fail-closed route decision. It runs before instructor
components render or transport requests begin. It covers every instructor-only route, including roster and
gradebook, and does not depend on a component hiding itself after a request. A direct navigation to
an instructor-only route must receive the same denial and no instructor transport as an in-app link.

## Historical visual references

The first four retained screenshot references show a historical allowed assignment overview with
plain-text instructions and server-resolved course-zone delivery details. The next four show that
historical student session denied access to a representative Instructor Gradebook route. Each set
follows the table's desktop, portrait tablet, iPhone Pro, and square order. These historical images
do not establish current acceptance. The current Live Demo has separate Student delivery and
Instructor-route-denial browser evidence under [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

<!-- screenshots:begin (managed by screenshot-docs) -->

![Student assignment overview at the 1280 by 800 desktop viewport](screenshots/student/access/allowed_assignment_overview/01_assignment_overview_laptop.png)
![Student assignment overview at the 800 by 1280 portrait-tablet viewport](screenshots/student/access/allowed_assignment_overview/01_assignment_overview_tablet.png)
![Student assignment overview at the 393 by 852 iPhone Pro viewport](screenshots/student/access/allowed_assignment_overview/01_assignment_overview_iphone_pro.png)
![Student assignment overview at the 800 by 800 square viewport](screenshots/student/access/allowed_assignment_overview/01_assignment_overview_square.png)
![Student denied the instructor gradebook route at the 1280 by 800 desktop viewport](screenshots/student/access/instructor_route_denial/01_student_instructor_denial_laptop.png)
![Student denied the instructor gradebook route at the 800 by 1280 portrait-tablet viewport](screenshots/student/access/instructor_route_denial/01_student_instructor_denial_tablet.png)
![Student denied the instructor gradebook route at the 393 by 852 iPhone Pro viewport](screenshots/student/access/instructor_route_denial/01_student_instructor_denial_iphone_pro.png)
![Student denied the instructor gradebook route at the 800 by 800 square viewport](screenshots/student/access/instructor_route_denial/01_student_instructor_denial_square.png)
<!-- screenshots:end -->

## Refreshing evidence

`./devel/capture_screenshots.sh` rebuilds only the manifest-listed current captures; `--verify`
validates their publication state without starting the stack. These older images remain historical
references, not current acceptance. Student visual changes require a fresh capture and human visual
review before they can claim visual acceptance.

## Planned surfaces

| Surface                                  | Role           | Evidence purpose                    | Screenshot area                    |
| ---------------------------------------- | -------------- | ----------------------------------- | ---------------------------------- |
| Student assignment list                  | Student        | Allowed course work                 | `docs/screenshots/student/`        |
| Student Assignment or Assignment Attempt | Student        | Allowed Student task                | `docs/screenshots/student/`        |
| Student access denial                    | Student/access | Fail-closed instructor-route denial | `docs/screenshots/student/access/` |
| Roster denial probe                      | Student/access | No instructor transport             | `docs/screenshots/student/access/` |
| Gradebook denial probe                   | Student/access | No instructor transport             | `docs/screenshots/student/access/` |

This historical table is a retained product coverage target. The current
manifest, rather than this table, declares executable screenshot artifacts.

## Evidence boundaries

The current Browser Surface exercises seeded identity entry, Student Course Invitation claim,
Assignment Access, delivery, and submission. Email-code and passkey authentication remain future
work. Keep authenticated Student artifacts under `docs/screenshots/student/` and pre-authentication
artifacts under `docs/screenshots/public/`; keep current evidence free of Answer Keys, Question
Grader code, private source, real email, real identifying records, UUIDs, and FERPA records.
Deterministic fictional fixture addresses in the reserved `example.invalid` domain are permitted
and are not real identifying records.

Screenshots become acceptance evidence only after a fresh capture at the required viewport, visual
inspection of the captured files, and passing behavior and no-transport assertions. Retained images
alone do not establish current acceptance.

## Validation handoff

The retained historical T1 and S4 references describe the broader student/access matrix. The current
browser owner covers allowed Student delivery and fail-closed direct-route denial; future additions
still need behavior-named browser evidence before fresh visual capture can provide acceptance. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the repository-wide evidence model and
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) for the durable viewport decision.
