#!/usr/bin/env python3
"""Validate the answer-free Chapter 1 release-seed manifest."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from uuid import UUID


EXPECTED_CHAPTERS = (
    (
        "genetics-chapter-1",
        (
            "genetics-disorders-webwork-mc",
            "genetics-disorders-webwork-matching",
            "genetics-disorders-flat-mc",
            "genetics-disorders-flat-matching",
        ),
    ),
    (
        "biochemistry-chapter-1",
        (
            "biochemistry-functional-groups-webwork-mc",
            "biochemistry-functional-groups-webwork-matching",
            "biochemistry-functional-groups-flat-mc",
            "biochemistry-functional-groups-flat-matching",
        ),
    ),
)
QUESTION_ID_PATTERN = re.compile(r"[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}\Z")


def fail(message: str) -> None:
    raise SystemExit(f"Chapter 1 manifest E2E: {message}")


def load_manifest(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"could not read {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path} is not a JSON object")
    return value


def require_uuid(value: object, label: str) -> None:
    if not isinstance(value, str):
        fail(f"{label} is not a UUID string")
    try:
        UUID(value)
    except ValueError as error:
        fail(f"{label} is not a UUID: {error}")


def validate(manifest: dict[str, object]) -> None:
    if set(manifest) != {"chapters"}:
        fail("the manifest must expose only the chapter collection")
    chapters = manifest["chapters"]
    if not isinstance(chapters, list) or len(chapters) != len(EXPECTED_CHAPTERS):
        fail("the manifest must contain exactly Genetics Chapter 1 and Biochemistry Chapter 1")

    display_ids: list[str] = []
    for chapter_index, ((expected_slug, expected_questions), chapter) in enumerate(
        zip(EXPECTED_CHAPTERS, chapters, strict=True), start=1
    ):
        if not isinstance(chapter, dict):
            fail(f"chapter {chapter_index} is not an object")
        if set(chapter) != {
            "slug",
            "courseId",
            "assignmentId",
            "enrollmentId",
            "questions",
        }:
            fail(f"chapter {chapter_index} has an unexpected field set")
        if chapter["slug"] != expected_slug:
            fail(f"chapter {chapter_index} is not {expected_slug}")
        for name in ("courseId", "assignmentId", "enrollmentId"):
            require_uuid(chapter[name], f"{expected_slug}.{name}")

        questions = chapter["questions"]
        if not isinstance(questions, list) or len(questions) != 4:
            fail(f"{expected_slug} must contain exactly four questions")
        for question_index, (expected_question, question) in enumerate(
            zip(expected_questions, questions, strict=True), start=1
        ):
            if not isinstance(question, dict):
                fail(f"{expected_slug} question {question_index} is not an object")
            if set(question) != {"slug", "displayId", "problemId", "versionId"}:
                fail(f"{expected_slug} question {question_index} has an unexpected field set")
            if question["slug"] != expected_question:
                fail(f"{expected_slug} question {question_index} is not {expected_question}")
            display_id = question["displayId"]
            if not isinstance(display_id, str) or not QUESTION_ID_PATTERN.fullmatch(display_id):
                fail(f"{expected_question} lacks a canonical human-readable Question ID")
            display_ids.append(display_id)
            require_uuid(question["problemId"], f"{expected_question}.problemId")
            require_uuid(question["versionId"], f"{expected_question}.versionId")

    if len(set(display_ids)) != 8:
        fail("the eight questions must have distinct human-readable identities")


def main(argv: list[str]) -> None:
    if len(argv) != 3:
        fail("usage: e2e_chapter_one_manifest.py FIRST_MANIFEST SECOND_MANIFEST")
    first = load_manifest(Path(argv[1]))
    second = load_manifest(Path(argv[2]))
    if first != second:
        fail("the release seed did not produce the same manifest on rerun")
    validate(first)


if __name__ == "__main__":
    main(sys.argv)
