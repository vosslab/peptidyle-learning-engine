# Agent instructions

## Durable authorities

### Human edited primary authority

- docs/HUMAN_GUIDANCE.md
- docs/FALL_2026_PILOT.md

### LLM but human lightly inspected authorities

- docs/DATABASE_STYLE.md
- docs/TERMINOLOGY_CONTRACT.md
- docs/DESIGN_DECISIONS.md
- docs/REPO_STYLE.md
- docs/*_STYLE.md 

### Other files of note

- docs/CONTRACTS.md
- docs/ROADMAP.md
- docs/TODO.md

### external content of note

- OTHER_REPOS/adapt/ # platform that we are building from and making better
- OTHER_REPOS/biology-problems-website/ # source of pilot content

## Workflow

- Follow the main authoritative docs/HUMAN_GUIDANCE.md and the language, naming, repository, 
  Markdown, and test style guides in docs/.
- Use `source ./source_me.sh && ./launchers/run_fast_checks.sh` for a quicker compliance check
- Use `source ./source_me.sh && ./launchers/all_test.sh` for a complete compliance check
- Use `source ./source_me.sh && ./devel/capture_screenshots.sh` for UI work to capture fresh screenshots
- Complete one bounded task, pass its narrow gate, then update docs/CHANGELOG.md.
- Run Python commands through `source source_me.sh && python3`.

