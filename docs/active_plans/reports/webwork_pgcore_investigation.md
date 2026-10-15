# WeBWorK PGcore failure investigation

2026-10-14. Investigation only; no renderer, BPQ, BPW, or importer source was changed.

## Finding

The current `genetics_terminology-matching.pgml` has an escaping edge case in its generated
PG setup code. The original source fails under both the deployed renderer image and a fresh,
unmodified renderer HEAD build. All required macros load in both environments. The failure is
not caused by the uncommitted renderer changes or a missing dependency in either tested image.

The owning repository is **biology-problems (BPQ)**. Its matching YAML generator selects a
plain-Perl escaping helper where a complete PG-compatible string literal is needed.

This does not imply that the BPQ pipeline is generally invalid. Five unchanged Biochemistry
samples, an unchanged Genetics matching question, two minimal PGML examples, and two complete
handbook examples rendered successfully. The user's successful complete Biochemistry import
into ADAPT last spring is consistent with an escaping edge case. ADAPT's exact historical
runtime and source artifacts were not available in this investigation, so no claim is made
about reproducing that environment.

## Image and source provenance

| Property | Failing image | Clean comparison |
| --- | --- | --- |
| Image tag | `localhost/pg-renderer:reviewed` | `localhost/ple-pgcore-clean-head:investigation` |
| Image ID | `eada5de385c19296a06f1753659ac8dbb43e309c11bfd07d8cdf9d4bcb1b5660` | `94a75051ac5547b8644df841f8cfa4c80b214babcaa91afffe9e60ab8992cfc0` |
| Renderer source | Embedded working-tree snapshot matching the checkout's modified files | Exact `git archive HEAD` of `a06111e1a4b0e2cf5149d7a586fd165e4b189cd8` |
| PG revision | `3ca5687eaa28bebde231043a1ea5609c04edd670` | `a29b85f259fac1924688a2a6f8b80b9715c4bc55` (`PG-2.17+`) |
| Ubuntu | 26.04, from modified `ubuntu:latest` | 24.04, from HEAD Dockerfile |

The failing image was created on 2026-10-13 at 13:54:09 UTC. It has no renderer source-revision
label and excludes the application's `.git` directory. Therefore its precise historical build
invocation cannot be certified from a commit label. Instead, embedded file hashes were compared:
177 of 188 tracked paths match HEAD, ten differences match the current working tree (including
the deleted license), and the remaining difference is the intentionally excluded `.gitignore`.
Embedded `PG.pm`, `RenderProblem.pm`, `FormatRenderedProblem.pm`, and Dockerfiles exactly match
the modified checkout. Embedded `PLEAnswerReview.pm` also matches the untracked file.

The clean build used the archived HEAD Dockerfile unchanged, including its older PG release
and checkbox macro backport. It did not reuse the modified renderer application layer. The
runtime-init script exists in the failing image but its Dockerfile starts Hypnotoad directly;
that script is not its entrypoint. Existing working-tree changes were preserved: all 194
snapshotted renderer files and Git status were unchanged after the comparison.

## Where PGcore generation fails

The YAML source is ordinary valid text:

```yaml
genome:
  - an organism's complete set of DNA
```

The generated PGML file contains this at line 58:

```perl
'an organism\'s complete set of DNA',
```

Both installed versions of `WeBWorK::PG::Translator::default_preprocess_code` deliberately
double backslashes for TeX and then translate `~~` into a backslash:

```perl
$evalString =~ s/\\/\\\\/g;
$evalString =~ s/~~/\\/g;
```

Consequently Perl compilation receives a doubled backslash before the apostrophe. The quote
ends the string instead of being escaped. The original source produces:

```text
Unknown regexp modifier "/t"
syntax error ... near "'an organism\\'s complete set of DNA', ..."
Execution ... aborted due to compilation errors
```

The failing image reports generated line 59 because its translator prepends a setup line;
the clean build reports line 58. These are the same source location, not different failures.

`PG.pm` calls `$translator->translate`. Translation preprocesses the entire source and passes
it to `Safe::reval`, expecting the returned tuple to include `$PGcore`. Compilation fails before
the problem executes, so the returned PGcore is undefined. The translator retains `$@` in
`errors` and sets `error_flag`. `RenderProblem.pm` subsequently detects the missing object and
adds `Problem failed during render - no PGcore received.` That is a secondary diagnostic, not
the underlying exception. The actual syntax error is in the debug HTML's Translator errors
section; `debug.pg_warn` is null because the PGcore warning collector was never available.

## Controlled comparison

Both containers received the same source, seed 1, JSON `/render-api?_format=json` request, and
`outputFormat=debug`. They used separate loopback ports and disposable writable directories.
Their image-defined runtime users were retained. No generated source file was edited.

| Case | Failing image | Clean HEAD |
| --- | --- | --- |
| Minimal PGML with numeric answer | Pass | Pass |
| All seven macros required by the matching question | Pass | Pass |
| Original Genetics terminology matching source | Syntax error | Same syntax error |
| Same source with only the apostrophe literal changed to `q{...}` in memory | Pass, five dropdowns | Pass, five dropdowns |
| Same source with only PG `~~'` escaping in memory | Pass, five dropdowns | Pass, five dropdowns |
| Unchanged `monohybrid_cross_genotype-matching.pgml` | Pass, six dropdowns | Pass, six dropdowns |
| Unchanged `levels_of_protein_structure-matching.pgml` | Pass, four dropdowns | Pass, four dropdowns |

Additional unchanged Biochemistry samples passed in the failing image:
`levels_of_protein_structure-which_one.pgml`, `which_amino_acid.pgml`,
`ionic_bond_amino_acids.pgml`, and `ph_h_concentration_ratio.pgml`.
Complete examples extracted from the handbook's sections 2.1 and 5.2 also passed.
This was a focused diagnosis, not a full-catalog or all-seed acceptance run.

## Generator ownership and correction direction

The source chain is:

1. BPW `task_files/genetics_tasks1.csv:36` selects BPQ's
   `problems/matching_sets/inheritance/genetics_terminology.yml`.
2. BPW `bbq_settings.yml` maps the matching family to BPQ's `yaml_match_to_pgml.py`.
3. BPQ `yaml_match_to_pgml.py:237`, `render_match_data`, calls
   `webwork_lib.escape_perl_string` and surrounds the result with single quotes.
4. BPQ `webwork_lib.py:92` escapes apostrophes as `\'`, which is valid plain Perl but does
   not survive PG preprocessing in this context.

BPQ already provides `webwork_lib.perl_string_literal` at line 118, which selects `q{...}` for
apostrophe-containing text. Its documentation explicitly recognizes this PG escaping problem.
The matching generator bypasses that helper. The multiple-choice YAML generator also manually
wraps escaped values, so the same family of call sites merits a bounded follow-up.

To verify ownership, the actual matching generator was imported and run against the original
YAML entirely in memory. Current output failed under both images. Replacing only its
`render_match_data` function in memory to use the existing complete-literal helper for keys
and values produced output that passed under both images with five dropdowns and no warnings.

The appropriate follow-up is a BPQ generator correction, followed by ordinary regeneration of
affected BPW artifacts. Do not repair individual generated PGML files or add an importer
rewrite. The observed apostrophe case is demonstrated; this investigation does not certify
the helper's handling of every other quoting, brace, backslash, or newline combination.
Existing helper tests assert plain-Perl escape bytes and do not establish acceptance through
the PG preprocessing boundary.

## Evidence

Temporary forensic receipts are retained under
`/Users/vosslab/.cache/ple_pgcore_investigation/`: image and working-tree hash manifests,
`comparison.json`, `generator_comparison.json`, `pgml_samples.json`,
`biochemistry_samples.json`, and `handbook_samples.json`. The clean build log is
`/tmp/ple_pgcore_clean_build.log`. These are investigation evidence, not permanent tests or
new import gates. Renderer, generator, generated content, and importer repairs remain unmade.
