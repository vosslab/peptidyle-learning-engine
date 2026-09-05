# Ribbon destination ledger

This ledger distinguishes what the Ribbon is designed to name from what the
current product can truthfully offer as a live destination.

<!-- BEGIN GENERATED RIBBON DESTINATION LEDGER -->

## Generated capability evidence

This section is machine-owned. Run `node --import tsx devel/generate_ribbon_destination_ledger.mjs`
after changing the catalog or capability registry; do not edit the table by hand.

Ribbon Availability is projected with every Product Role and a resolved, allowed relationship.
This documents the role ceiling before relationship denial: `ribbonAvailability(entry, role,
{ kind: "resolved", allowed: true })`. Runtime authorization remains at the route and server
boundaries.

| Canonical label     | Route id or future identity                                                                                         | Client method                                                                                                                                 | Backing handler evidence                                                                                                                                                                                                                                                                                                                    | Derived Ribbon Availability                                              |
| ------------------- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Courses             | courses ([src/route_contract.ts](../../src/route_contract.ts))                                                      | ApiClient.listCourses ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listCourses)                               | No complete handler: Course listing has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listCourses                                                | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Question Library    | library ([src/route_contract.ts](../../src/route_contract.ts))                                                      | ApiClient.searchQuestionLibrary ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.searchQuestionLibrary)           | No complete handler: Question Library search has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.searchQuestionLibrary                             | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Blueprint Courses   | blueprintCourses ([src/route_contract.ts](../../src/route_contract.ts))                                             | ApiClient.listBlueprintCourses ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listBlueprintCourses)             | No complete handler: Blueprint Course listing has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listBlueprintCourses                             | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Assignments         | courseAssignments ([src/route_contract.ts](../../src/route_contract.ts))                                            | ApiClient.listAssignments ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listAssignments)                       | No complete handler: Assignment listing has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listAssignments                                        | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Students            | courseRoster ([src/route_contract.ts](../../src/route_contract.ts))                                                 | ApiClient.listCourseRoster ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listCourseRoster)                     | No complete handler: Course roster has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listCourseRoster                                            | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Gradebook           | gradebook ([src/route_contract.ts](../../src/route_contract.ts))                                                    | ApiClient.getCalculatedGradebook ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.getCalculatedGradebook)         | No complete handler: Calculated Gradebook has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.getCalculatedGradebook                               | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Teaching Operations | teachingOperations ([src/route_contract.ts](../../src/route_contract.ts))                                           | No declared client method.                                                                                                                    | No complete handler: Teaching Operations has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/pages/teaching_operations_page.tsx](../../src/pages/teaching_operations_page.tsx)::TeachingOperationsPage                    | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Blueprint Updates   | Future identity: blueprintUpdates (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))   | No declared client method.                                                                                                                    | No complete handler: Blueprint Updates has no declared route, page, client method, or registered handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::blueprintUpdates                                 | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Course Setup        | Future identity: courseSetup (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))        | No declared client method.                                                                                                                    | No complete handler: Course Setup is a future destination identity, not a declared usable path.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::courseSetup                                                | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Attempt             | assignmentAttempt ([src/route_contract.ts](../../src/route_contract.ts))                                            | ApiClient.getAssignmentAttemptScreen ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.getAssignmentAttemptScreen) | No complete handler: Assignment Attempt screen has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.getAssignmentAttemptScreen                      | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Instructor Accounts | Future identity: instructorAccounts (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)) | No declared client method.                                                                                                                    | No complete handler: Instructor Accounts has no declared route, page, client method, or registered handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::instructorAccounts                             | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| All Questions       | library ([src/route_contract.ts](../../src/route_contract.ts))                                                      | ApiClient.searchQuestionLibrary ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.searchQuestionLibrary)           | No complete handler: All Questions has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.searchQuestionLibrary                                       | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| My Questions        | Future identity: myQuestions (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))        | No declared client method.                                                                                                                    | No complete handler: My Questions has no declared route state or registered production handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::myQuestions                                                | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| My Question Drafts  | Future identity: myQuestionDrafts (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))   | No declared client method.                                                                                                                    | No complete handler: My Question Drafts is a retained future destination without a declared usable path or registered production handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::myQuestionDrafts | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Starred             | Future identity: starredQuestions (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))   | No declared client method.                                                                                                                    | No complete handler: Starred has no declared route, page, client method, or registered handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::starred                                                    | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Watched             | Future identity: watchedQuestions (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))   | No declared client method.                                                                                                                    | No complete handler: Watched has no declared route, page, client method, or registered handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::watched                                                    | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Overview            | assignmentWorkspaceOverview ([src/route_contract.ts](../../src/route_contract.ts))                                  | No declared client method.                                                                                                                    | No complete handler: Assignment workspace Overview has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/routes.ts](../../src/routes.ts)::routeComponents                                                                   | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Questions           | assignmentWorkspaceQuestions ([src/route_contract.ts](../../src/route_contract.ts))                                 | No declared client method.                                                                                                                    | No complete handler: Assignment workspace Questions has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/routes.ts](../../src/routes.ts)::routeComponents                                                                  | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Policies            | assignmentWorkspacePolicies ([src/route_contract.ts](../../src/route_contract.ts))                                  | No declared client method.                                                                                                                    | No complete handler: Assignment workspace Policies has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/routes.ts](../../src/routes.ts)::routeComponents                                                                   | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Grading Operations  | assignmentWorkspaceGradingOperations ([src/route_contract.ts](../../src/route_contract.ts))                         | No declared client method.                                                                                                                    | No complete handler: Assignment workspace Grading Operations has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/routes.ts](../../src/routes.ts)::routeComponents                                                         | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Student View        | assignmentWorkspaceStudentView ([src/route_contract.ts](../../src/route_contract.ts))                               | No declared client method.                                                                                                                    | No complete handler: Assignment workspace Student View has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/routes.ts](../../src/routes.ts)::routeComponents                                                               | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Grade Settings      | courseGradeSettings ([src/route_contract.ts](../../src/route_contract.ts))                                          | ApiClient.getCourseGradeSettings ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.getCourseGradeSettings)         | No complete handler: Grade Settings has no registered production teaching/data handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.getCourseGradeSettings                                     | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Appearance          | Future identity: courseAppearance (no route) ([src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts))   | No declared client method.                                                                                                                    | No complete handler: Appearance has no declared usable route and no registered production handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/ribbon/ribbon_catalog.ts](../../src/ribbon/ribbon_catalog.ts)::appearance                                              | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |
| Back to Assignments | courseAssignments ([src/route_contract.ts](../../src/route_contract.ts))                                            | ApiClient.listAssignments ([src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listAssignments)                       | No complete handler: Back to Assignments leads to a surface without a registered production handler.<br>[crates/server/src/composition.rs](../../crates/server/src/composition.rs)::production_router_from_env<br>[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listAssignments                               | instructor: Unavailable<br>student: Unavailable<br>sysadmin: Unavailable |

<!-- END GENERATED RIBBON DESTINATION LEDGER -->

## Editorial destination notes

This section is human-owned. It explains the teaching purpose of each canonical
destination; generation and tests intentionally leave this prose alone.

### Courses

Courses is the signed-in Account's Course Instance starting surface. It helps a
reader orient to teaching work; it remains omitted until its complete path lands.

### Question Library

Question Library is the place to discover Published Questions. Its current
client search evidence does not make the missing production handler usable.

### Blueprint Courses

Blueprint Courses is the workspace for reusable course blueprints. The declared
route is not sufficient evidence of a complete production capability.

### Assignments

Assignments names the selected Course Instance's assignment list. It becomes a
live Ribbon Tab only with its teaching-data handler, not merely a client call.

### Students

Students is the Course Instance roster and invitation surface. It stays omitted
while the registered production roster path is absent.

### Gradebook

Gradebook presents calculated course grades. The present client method records
intent, while the absent handler keeps the Ribbon claim truthful.

### Teaching Operations

Teaching Operations groups course-lifecycle teaching work. A visible page file
does not establish a backed production destination, so the Ribbon withholds it.

### Blueprint Updates

Blueprint Updates will show reviewed changes from a parent Blueprint Course.
It is a closed future identity with no route and is intentionally omitted.

### Course Setup

Course Setup will collect Course Instance configuration. It remains a future
identity until a route, client path, and complete handler are declared.

### Attempt

Attempt names a Student's current Assignment Attempt surface. It remains out of
the shipped Ribbon until its screen data path is complete in production.

### Instructor Accounts

Instructor Accounts is the future Sysadmin surface for instructor vetting and
account state work. There is no route or client method to advertise today.

### All Questions

All Questions is the Question Library view of Published Questions available to
the current Account. It stays omitted until the existing search client reaches a
registered production handler.

### My Questions

My Questions will show Published Questions owned by the current Account. Its
future identity is preserved without inventing route state or a live link.

### My Question Drafts

My Question Drafts will enter the private Authoring Workspace Store. It is not
Question Library membership and remains omitted until its usable path exists.

### Starred

Starred will expose the current Account's Question Star relationship. The label
is retained for spatial design, but no route or handler is claimed today.

### Watched

Watched will expose the current Account's Question Watch relationship. It is
truthfully withheld because no complete destination currently exists.

### Overview

Overview is the first task inside an Instructor assignment workspace. The
declared route does not yet provide the complete teaching/data handler required
for a live Ribbon Task.

### Questions

Questions is the assignment workspace task for choosing and organizing
questions. It remains omitted until the route's production capability is whole.

### Policies

Policies is the assignment workspace task for delivery and scoring rules. A
future live control requires a registered teaching/data handler, not page shape.

### Grading Operations

Grading Operations is the assignment workspace task for automated grading
operations. Its Ribbon slot stays withheld until its protected capability works.

### Student View

Student View is the assignment workspace task for inspecting learner-facing
presentation. It is retained in the catalog but omitted without a full handler.

### Grade Settings

Grade Settings configures grade calculation for a Course Instance. The client
method is evidence of intended integration, not proof of a usable server path.

### Appearance

Appearance names Course Appearance configuration. It is a future destination
identity and remains omitted until it has a complete declared route and handler.

### Back to Assignments

Back to Assignments returns an Assignment Attempt reader to the course list of
assignments. It does not render as a live Task until that destination is backed.
