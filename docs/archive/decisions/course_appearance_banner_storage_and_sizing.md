# Course Appearance banner geometry

## Status

Current product decision. [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) is the
authority. This decision replaces the former 6:1 course-entry hero, 5:2 card
crop, and dual-rendition design recorded during M6/M8 planning.

## Product contract

- Course banners use a 5:1 aspect ratio.
- 1280 by 256 pixels is the recommended authoring size.
- Higher-resolution 5:1 images are supported.
- PLE responsively scales Course banners while preserving their aspect ratio.
- Course banners appear as small centered banners rather than full-width page
  heroes.

The same banner geometry applies within every supported interface viewport.
Viewport support describes the interface containing the banner; it does not
create another banner crop or geometry.

PLE has no separate course-entry and course-card banner geometries. The product
contract does not require multiple banner renditions, crop previews, focal-point
selection, or a banner-specific processing lifecycle.

## Compatible behavior

An Instructor uploads one Course banner independently of the Course's
three-color theme. PLE keeps the Course identifier outside the artwork and does
not overlay Course text, a scrim, or a gradient on Instructor artwork.

The image remains either decorative or informative. Informative images have
saved alternative text; decorative images do not repeat the adjacent Course
identity to assistive technology.

Authorization, image validation, bounded upload size, storage integrity, and
replacement cleanup remain implementation responsibilities. They do not change
the 5:1 product geometry or justify an alternate crop family.

## Superseded implementation evidence

Current source, routes, tables, or generated artifacts may still name `hero`,
`card`, or banner renditions and may still encode 6:1 or 5:2 output. Those are
implementation gaps, not compatibility requirements. Future implementation
work should converge on the single responsive 5:1 banner contract.

The retained
[historical ratio specimen](assets/course_appearance_banner_ratio_specimen.svg)
records the former page-width comparison. It does not select current product
geometry.
