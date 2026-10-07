# M04 producer and source fixture check

The Pilot producer stores title, description, language, tags, license, and citation as Question
record metadata. The four Native JSON sources now contain only display and grading content; their
tags and null citation values moved to the Pilot manifest, and the manifest retains the existing
project license and supplied language. The parameterized Genetics producer carries its available
title, description, and license into Draft metadata and leaves unknown language and citation absent.
Its existing author source URL remains provenance.

Publication reads the locked Draft metadata row as its record authority. Existing tag and license
arguments are checked against that row; title, description, language, citation, and remaining metadata
are inserted from the Draft.

The two PLE adapter source fixtures also contain content only. The browser response-format corpus is
response-only and did not need a change. The published Question fixture continues to contain record
metadata because it represents a persisted Question record.

## Focused checks

Commands ran from the repository root. `source ./source_me.sh` initializes the repository shell
environment.

| Command | Result |
| --- | --- |
| `source ./source_me.sh && cargo test -p project-tools pilot_content` | Passed: 4 matching producer tests; 27 other binary tests filtered. |
| `source ./source_me.sh && cargo test -p adapter_qti supported_qti_item_converts_to_ple_json_without_vendor_points_or_policy` | Passed: 1 focused QTI conversion test; 94 other adapter tests filtered. |
| `source ./source_me.sh && cargo tools pilot-content` | Passed: 2 chapters, 8 reviewed Questions, 4 adapted PGML sources. |
| `source ./source_me.sh && PLE_CURRICULUM_CONTENT_ROOT=content/genetics cargo tools curriculum-content validate manifest.yaml` | Passed: 9 topics, 41 canonical sources. |
| `rustfmt --check --edition 2024 crates/project-tools/src/pilot_content.rs crates/project-tools/src/pilot_content/publication.rs crates/project-tools/src/curriculum_content/parameterized_publication.rs crates/adapters/qti/src/profiles/server_parts.rs` | Passed. |
| `git diff --check` | Passed after formatting. |

The exact content-only source audit was:

```sh
source ./source_me.sh && python3 - <<'PY'
import json
from pathlib import Path
metadata = {'questionTitle', 'questionDescription', 'tags', 'questionLicense', 'questionCitation', 'language'}
paths = list(Path('content/pilot/ple_question_json').glob('*.json')) + list(Path('crates/adapters/ple/tests/fixtures').glob('*.json'))
for path in paths:
    value = json.loads(path.read_text())
    leaked = metadata.intersection(value)
    if leaked:
        raise SystemExit(f'{path}: Native metadata fields remain: {sorted(leaked)}')
print(f'{len(paths)} Native JSON sources parse and contain no record metadata fields')
PY
```

Result: `6 Native JSON sources parse and contain no record metadata fields`.

The first producer test attempt exposed the in-repository QTI adapter's old call to the previous
five-argument Native import constructor. The caller now passes only prompt, choices, and correct
choice; its public mapping retains the source title separately. Both focused producer and QTI
conversion tests then passed.
