# Question export specification

## Purpose

Instructor export sends selected Questions to another LMS. It is not a PLE backup or PLE-to-PLE
transfer workflow. Export does not change the original Question, owner, Revision, license, or
Student Work.

## Selection and output

An Instructor can select Published Questions or a Question Pool. Selecting a Pool includes its
member Questions in one LMS-importable package. Each member is an individual Question in the
package. Preserve question-bank grouping where the target supports it.

Export uses the exact selected Question Revisions and preserves attribution and license
information. It carries supported content, metadata, and assets into the target format and
reports unsupported content or information that cannot be preserved. It must not silently omit
a selected Question or claim that an unsupported interaction was converted correctly.

Student Work and credentials are not Question export content. The server checks the Instructor's
access to the selected content before exporting it.

## Conversion ownership

PLE uses `qti-package-maker-rs` as an external library for all conversion and packaging. PLE
supplies the selected content; the library handles the supported target formats. PLE does not
build a second converter or its own competing package format.

[QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md) documents the distinction between interchange
formats and PLE's internal Question source. Exact reader, writer, and Question Type support must
be checked in the external library. Existing Blueprint JSON comparison and exchange rules are a
separate concern and do not expand this Instructor Question export workflow.
