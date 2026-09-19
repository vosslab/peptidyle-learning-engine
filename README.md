# Peptidyle Learning Engine

An open-source teaching platform for instructors to build reusable courses and automatically graded practice, combining native and WeBWorK Questions with exact Revision evidence and server-owned grading.

## Preparing for launch

PLE is preparing for its first production launch. The local Live Demo provides connected
Instructor, Student, and Sysadmin workflows using the ordinary application, database, and
Course relationships. Authoring, delivery, and interface work remain under active refinement;
individual implementation and screenshot receipts establish only their stated scope.
[docs/ROADMAP.md](docs/ROADMAP.md) records release gates, and
[docs/TEST_EVIDENCE_MODEL.md](docs/TEST_EVIDENCE_MODEL.md) explains what each check proves.
The quick start below is a disposable local demonstration, not a production deployment procedure.

Email authentication is not yet configured for the Live Demo. Students and Instructors enter
through its visible fictional-account selector; Sysadmin entry additionally requires genuine
TOTP authentication. See [docs/LIVE_DEMO_SPEC.md](docs/LIVE_DEMO_SPEC.md).

## Reuse Questions, preserve the work

PLE separates reusable teaching content from its delivery in a particular Course:

- Find, author, publish, and reuse Questions through one global Question Library.
- Combine native static Questions and algorithmic WeBWorK PG or PGML source. A generated
  variant belongs to its algorithmic Question; a Question Pool selects among distinct Questions.
- Design a Blueprint Course, then create a teaching Course Instance with independent Assessments.
- Set Assessment content and Properties separately: Question order and points, Attempts, timing,
  availability, and permitted feedback.
- Let Students save responses, resume an open Attempt, and submit the whole Attempt, while
  retained work identifies the exact Question Revision and, when the Question came from a Pool, the Pool ID and Pool Edit Number delivered.

Regular Assignments default to repeated practice with unlimited Attempts. Practice Question
Assignments provide focused review and show correct answers after whole-Attempt submission.
Quizzes and Exams allow one Attempt. These are pedagogical Types within the same Assessment
model, with independently configurable settings; see
[docs/MASTERY_ASSIGNMENT_DESIGN.md](docs/MASTERY_ASSIGNMENT_DESIGN.md).

Question Backends own interaction and grading. PLE keeps grading authority, protected answer
content, and provider credentials on the server, and scopes Student Work through Course
relationships and Student ownership. Reusable published content remains distinct from
FERPA-sensitive Attempts, responses, and grades. Current product intent is defined by
[docs/HUMAN_GUIDANCE.md](docs/HUMAN_GUIDANCE.md), with vocabulary in
[docs/TERMINOLOGY_CONTRACT.md](docs/TERMINOLOGY_CONTRACT.md).

## See the Live Demo

These repository captures show the fictional-account entry, Instructor Assessment Properties,
a Student's saved response, and Sysadmin administration. They are rendered evidence from the
published capture corpus, not a fresh verification of every current workflow.

<!-- screenshots:begin (managed by screenshot-docs) -->

![Live Demo sign-in with fictional Instructor, Student, and Sysadmin Accounts](docs/screenshots/public/sign_in_laptop.png)
![Instructor Assessment Properties Editor with a saved, released Assessment](docs/screenshots/instructor/assignment_release_released.png)
![Student Chapter 1 Pilot Practice Attempt with a saved response and Question navigation](docs/screenshots/student/assignment_attempt_saved_laptop.png)
![Sysadmin administration home with Instructor Account and scoped roster-support actions](docs/screenshots/sysadmin/system_administration_home_laptop.png)
<!-- screenshots:end -->

Browse [docs/SCREENSHOT_ATLAS.md](docs/SCREENSHOT_ATLAS.md) for the full grouped corpus and
its coverage gaps. Capture and replay instructions are in
[docs/SCREENSHOT_CONTRACT.md](docs/SCREENSHOT_CONTRACT.md).
The demo runs locally at the HTTPS address printed by the launcher.

## Quick start

Start from a checkout with Bash, Git, Python, Rust, Node.js/npm, Podman, and a usable Compose
provider. [docs/INSTALL.md](docs/INSTALL.md) gives the complete setup path;
[docs/MACOS_PODMAN.md](docs/MACOS_PODMAN.md) covers macOS Podman setup.
Install the declared Python runtime dependencies, then start the demo:

```bash
source source_me.sh && python3 -m pip install --requirement pip_requirements.txt
./launchers/run_live_demo.sh
```

The launcher installs missing TypeScript dependencies, builds the production browser bundle,
provisions the fixed disposable `ple-live-demo-browser` stack, and prints a ready HTTPS entry URL.
Open that URL, or open the running demo with:

```bash
./launchers/run_live_demo.sh open
```

To start a fresh demo and open it automatically, use `./launchers/run_live_demo.sh start --open`.
The standalone `--open` option also starts a fresh demo; `open` opens the existing one.

### Try a teaching workflow

1. Choose **Elena Rivera (Instructor)**. Open **BCHM 301** and inspect **Chapter 1 Pilot Practice**,
   its Questions, Assessment Properties, answer-free Student View, and Gradebook.
2. Sign Out through Profile and choose **Jack Nguyen (Student)**. Resume the open practice
   Attempt to inspect its saved response and Question navigation.
3. Choose **Mary Okafor (Student)** to inspect completed work, or **Avery Thompson (Student)**
   to see the same released practice before starting a new Attempt.

These personas use ordinary Course membership and Student Work. The reusable demo Blueprint is
**Biochemistry 301: Proteins and Peptides**. The installation also includes the free, open-source
BiologyProblems.org Genetics example Blueprint. Its canonical PG/PGML Questions demonstrate
backend-native algorithmic variation. See [docs/LIVE_DEMO_SPEC.md](docs/LIVE_DEMO_SPEC.md)
for the teaching graph and identity boundaries.

Stop the demo when finished:

```bash
./launchers/run_live_demo.sh stop
```

Starting again replaces this demo's containers, volumes, networks, and local records. Unrelated
Podman projects are outside that lifecycle. For startup problems, use
[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md); for diagnostics and controller ownership,
use [docs/LOCAL_STACK_OPERATIONS.md](docs/LOCAL_STACK_OPERATIONS.md).

## Find the right documentation

- [docs/INSTALL.md](docs/INSTALL.md): prerequisites, checkout setup, and installation boundaries.
- [docs/USAGE.md](docs/USAGE.md): Live Demo commands, Blueprint adoption, and stack diagnostics.
- [docs/LIVE_DEMO_SPEC.md](docs/LIVE_DEMO_SPEC.md): fictional Accounts, teaching data, and demo access.
- [docs/HUMAN_GUIDANCE.md](docs/HUMAN_GUIDANCE.md): controlling product intent and teaching rules.
- [docs/TERMINOLOGY_CONTRACT.md](docs/TERMINOLOGY_CONTRACT.md): Courses, Assessments, Questions,
  Revisions, Student Work, and retention vocabulary.
- [docs/CODE_ARCHITECTURE.md](docs/CODE_ARCHITECTURE.md): component ownership and security boundaries.
- [docs/ROADMAP.md](docs/ROADMAP.md): release direction and production gates.
- [docs/RELATED_PROJECTS.md](docs/RELATED_PROJECTS.md): related assessment systems and standards.

For common design questions, see [docs/FAQ.md](docs/FAQ.md). Developers can continue with
[docs/FILE_STRUCTURE.md](docs/FILE_STRUCTURE.md), [docs/CONTRACTS.md](docs/CONTRACTS.md),
and [docs/API_CONTRACTS.md](docs/API_CONTRACTS.md) for layout, module boundaries, and HTTP details.

## For contributors

The implementation uses a Rust workspace, PostgreSQL, S3-compatible local MinIO storage, and a
Solid/TypeScript browser with a WebAssembly bridge. Follow
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for dependency setup and bounded verification.
The build and offline check front doors are:

```bash
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
```

The complete aggregate adds disposable service acceptance:

```bash
source source_me.sh && ./launchers/all_test.sh
```

Browser and screenshot evidence have separate execution lanes. Passing offline checks or service
acceptance alone does not establish a visible teaching journey; see
[docs/TEST_EVIDENCE_MODEL.md](docs/TEST_EVIDENCE_MODEL.md).

## License and authorship

Code is licensed under the [GNU Affero General Public License v3](LICENSE.AGPL-3.0). Documentation
and figures are licensed under [Creative Commons Attribution 4.0](LICENSE.CC-BY-4.0). See
[docs/AUTHORS.md](docs/AUTHORS.md) for project authorship and acknowledgments.

The bundled Ribbon sprite redistributes only Font Awesome Free SVG icon artwork under
[Creative Commons Attribution 4.0](LICENSE.CC-BY-4.0). That attribution does not claim to
redistribute Font Awesome package tooling, code, or fonts.

The bundled Atkinson Hyperlegible Next browser fonts are distributed under the
[SIL Open Font License 1.1](src/assets/fonts/atkinson_hyperlegible_next/ofl_1_1.txt); their pinned
source and retained-file record are in
[the local provenance record](src/assets/fonts/atkinson_hyperlegible_next/provenance.txt).
