# DataSheetBlock and SearchBlock Specification

## Design Principles

- Follow KISS: prefer the smallest coherent design that satisfies actual requirements.
- Use shared components and behavior across PLE.
- Prefer browser-side processing to reduce server requests and costs.
- Prioritize useful content over surrounding interface elements.
- Keep presentation separate from collection state and page-specific behavior.
- Preserve flexibility where interaction details require practical evaluation.

## Component Structure

    SearchBlock
    +-- DataSheetControls
    +-- DataSheetBlock
        +-- HeaderRow (Compact and List only)
        +-- RecordItem x N

### SearchBlock

SearchBlock composes DataSheetControls and DataSheetBlock into a shared
collection interface.

- Supports controls above or beside the data.
- Accommodates page-specific search criteria, filters, and actions.
- Reuses shared collection functionality across workflows.
- Keeps surrounding controls separate from record presentation.

Typical arrangements:

- Question Library Search: Controls above the data.
- Question Library Browse: Controls beside the data.
- Compact search: Minimal controls, relying on HeaderRow for sorting
  and filtering.

### DataSheetControls

DataSheetControls provides the surrounding collection controls.

- Selects Compact, List, Cards, or Visual Boxes.
- Provides sorting and filtering controls when column headings are unavailable.
- Provides column configuration controls when applicable.
- Provides collection-level actions and bulk operations when applicable.
- Reflects current sorting, filtering, and selection state.
- Adapts its layout and available controls to the workflow.

### DataSheetBlock

DataSheetBlock is the reusable component for presenting collections.

- Displays a collection of RecordItem objects.
- Supports all four presentation modes.
- Renders HeaderRow in Compact and List modes.
- Arranges records into rows or a responsive grid.
- Handles direct interactions with records and column headings.
- Adapts presentation to available screen space.
- Excludes surrounding menus, display-mode controls, and collection-level
  action controls.

### HeaderRow

HeaderRow provides spreadsheet-style column functionality in Compact
and List modes.

- Displays aligned column headings.
- Supports sorting and filtering through column headings.
- Supports column resizing, reordering, and visibility.
- Reflects current sorting and filtering state.

Cards and Visual Boxes do not use HeaderRow.

### RecordItem

RecordItem represents one record in the collection.

- Displays record data according to the selected presentation mode.
- Supports applicable record-specific actions and interactions.
- Preserves record identity across display modes.

## Four Display Modes

| Property | Compact | List | Cards | Visual Boxes |
| --- | --- | --- | --- | --- |
| Layout | Spreadsheet | Table | Stacked panels | Tile grid |
| Items per row | One | One | One | Multiple |
| Column headings | Yes | Yes | No | No |
| Column sorting/filtering | Yes | Yes | No | No |
| Fixed columns | Yes | Yes | No | No |
| Text wrapping | No | Yes | Yes | Yes |
| Item height | Tight, consistent | Variable | Variable | Content-dependent |
| Primary emphasis | Data density | Readability | Text and metadata | Images and previews |

### Compact

A dense spreadsheet optimized for displaying and comparing many records.

- One RecordItem per row.
- Aligned columns with sortable and filterable headings.
- Tight, consistent row heights.
- Single-line text with truncation.
- Resizable, reorderable, and configurable columns.
- Cells may contain text, numbers, images, icons, and interactive elements.
- Horizontal scrolling when necessary.
- Prioritizes information density.

### List

A readable table with aligned columns and flexible-height rows.

- One RecordItem per row.
- Aligned columns with sortable and filterable headings.
- Text wraps onto multiple lines.
- Row heights expand to fit content.
- Supports descriptive text, metadata, images, and record actions.
- Prioritizes readability while retaining table structure.

Compact and List have independent column configurations:

- Compact displays more columns with less information per cell.
- List displays fewer columns with more information per cell.
- Each mode independently defines column order, widths, and visibility.
- Column configurations are transient for the current page instance.

### Cards

Independent, text-oriented content panels without spreadsheet columns.

- One RecordItem per row.
- No shared column headings or fixed columns.
- Text and metadata receive more space than images.
- Content flows naturally within each card.
- Card heights adapt to their contents.
- Supports images, descriptions, metadata, and record actions.
- Sorting and filtering use DataSheetControls.

### Visual Boxes

Image-oriented tiles arranged in a responsive grid.

- Multiple RecordItem objects per row.
- No shared column headings or fixed columns.
- Images, thumbnails, or previews dominate the presentation.
- Text and metadata are secondary.
- Supports portrait, landscape, and square visual content.
- Grid adapts the number of columns to available width.
- Supports record-specific actions and indicators.
- Sorting and filtering use DataSheetControls.
- Records without images display meaningful record content.

## Shared Collection Behavior

All four modes are alternative presentations of the same collection.

- Every DataSheetBlock supports all four modes.
- Every page provides access to all four modes.
- Each page defines its own default display mode.
- Sorting and filtering apply to the complete collection before pagination.
- Switching modes preserves record order, sorting, filtering, pagination,
  and selection.
- Switching modes changes presentation only, without triggering a new
  search or reordering records.
- Compact and List provide sorting and filtering through HeaderRow.
- Cards and Visual Boxes use DataSheetControls for sorting and filtering.
- Collection behavior is shared rather than implemented separately
  for each mode.

### Display-Mode Preferences

Display-mode preferences are transient.

- Each page defines a default appropriate to its workflow.
- Question Library defaults to Compact.
- Blueprint Course Search is expected to default to Cards.
- Users may switch freely among all four modes.
- Changes last only for the current page instance.
- Reopening the page restores its default mode.
- Preferences are not saved to local storage or the server.

## Collection State and Browser-First Processing

The page owns collection state and supplies records to SearchBlock.

Shared collection logic provides consistent sorting, filtering,
pagination, and selection behavior.

- Prefer browser-side processing for collections available in memory.
- Sorting and filtering operate on the complete loaded collection.
- Pagination determines which records are visible.
- DataSheetBlock renders the resulting records.
- Sorting, filtering, pagination, and display-mode changes operate
  locally when the collection is available in memory.
- Switching display modes does not request data from the server.
- Collections too large to load efficiently may use server-side
  queries while preserving the same interface and behavior.
- Search results remain unchanged until explicitly refreshed.

SearchBlock composes the interface, DataSheetControls exposes collection
operations, and DataSheetBlock renders records.

## Sorting and Column Filtering

Compact and List provide spreadsheet-style column interactions.

- Column headings support sorting and filtering.
- Column menu icons open sorting and filtering options.
- Filtering supports selecting available column values.
- Applicable column configuration options are available through
  column controls.
- Direct sorting interactions remain available without opening the
  full column menu.
- Sorting and filtering state persists when switching display modes.

Cards and Visual Boxes use DataSheetControls for sorting and filtering.

## Pagination

Use PLE's existing numbered pagination interface.

- Previous and Next navigation.
- Numbered page buttons.
- Ellipses for skipped page ranges.
- Current page clearly distinguished.
- Total page count displayed.
- Navigation adapts near the beginning, middle, and end.
- Sorting and filtering apply before pagination.
- Avoid infinite scrolling.

Do not impose a universal scroll-position policy.

- Loading a new page naturally starts at the beginning.
- Updating results within the current page may preserve scroll position.
- Opening a record in a new tab preserves the original search position.

## Selection

Selection is optional for each collection.

- Collections without meaningful selection operations may omit selection.
- When enabled, selection works across all four display modes.
- Selection belongs to record identity rather than presentation.
- Selection persists across display modes and pagination.
- Selected records receive whole-record highlighting.
- DataSheetBlock provides selection interactions and indicators.
- DataSheetControls provides selection counts and applicable bulk actions.
- The page determines which bulk actions are available.
- Opening a record and selecting a record are separate interactions.

Selection presentation remains adaptable:

- Compact and List may use checkboxes.
- Cards and Visual Boxes may use checkboxes or whole-card selection.
- Final interaction details should be evaluated through actual use.

## Opening Records

Opening search results should preserve the original search workflow.

- Question Library and Blueprint Course search results open in a new
  tab or window.
- The original tab retains search criteria, filters, sorting, selection,
  pagination, and scroll position.
- Opening records does not interrupt browsing large result collections.
- Opening and selecting records remain distinct operations.

## Thumbnails and Visual Previews

Every record supports a meaningful thumbnail or visual preview.

- Visual questions use thumbnails of their images or visual content.
- Text-only questions use a small rendering or preview of the question.
- Other records use an available thumbnail or meaningful content preview.
- When no thumbnail exists, display actual record content rather than
  a generic placeholder.
- Visual Boxes emphasize visual recognition, especially for questions
  containing diagrams, photographs, and other visual material.

### Preview Generation

- Generate question previews once and cache them.
- Reuse cached previews across display modes.
- Prefer existing question rendering capabilities.
- Avoid introducing a separate rendering pipeline unless necessary.

### Preview Layout

- Visual Boxes use a consistent preview area.
- Preserve original image aspect ratios.
- Fit images without distortion or cropping.
- Accommodate portrait, landscape, and square content.
- Display meaningful record content when an image is unavailable.

## Question Pool Editor

Question Pool editing is a dedicated workflow that reuses the shared
collection components.

Use a two-panel interface:

    +------------------------+---------------------------+
    | Current Question Pool  | Question Library          |
    |                        |                           |
    | Questions in pool      | Search / Browse           |
    |                        |                           |
    | Remove ->              | <- Add                    |
    |                        |                           |
    +------------------------+---------------------------+

- Current Question Pool appears on the left.
- Question Library search and browsing appear on the right.
- The pool remains visible while searching and browsing.
- Support adding and removing questions through buttons and drag-and-drop.
- Multi-selection may support efficient batch operations.
- Persistent selection across search pages is available but is not the
  primary mechanism for building pools.
- Pool-editing behavior belongs to the Question Pool Editor rather than
  DataSheetBlock.

## Empty Collections

Follow the existing requirements in HUMAN_GUIDANCE.md.

- Empty collection pages explain their purpose and provide an obvious
  creation action when applicable.
- Question Library Search initially displays an empty spreadsheet with
  column headings and a concise search prompt.
- Searching requires at least one keyword or filter.
- Clearing all search criteria restores the initial empty spreadsheet.
- Empty states remain compact and avoid unnecessary fixed-height regions.

## Content-First Layout

SearchBlock and its components follow PLE's content-first design principles.

- Primary content takes priority over surrounding interface elements.
- Controls justify their visibility, placement, and space consumption.
- Frequently used controls remain readily accessible.
- Related controls are consolidated where practical.
- Use available horizontal and vertical space efficiently.
- Responsive behavior preserves functionality.
- Evaluate content visibility, interface overhead, and interaction cost
  together.

## Implementation Philosophy

Apply KISS aggressively.

- Prefer the smallest coherent design satisfying actual requirements.
- Reuse existing collection logic and UI components where practical.
- Keep domain-specific behavior outside generic presentation components.
- Prefer browser-side operations when the data is already available.
- Avoid speculative mechanisms, abstractions, policies, and tests.
- Preserve adaptability where practical experience may change UI decisions.
- Resolve implementation details through existing code and demonstrated
  requirements rather than inventing additional configuration.

## Core Principle

DataSheetBlock provides four presentations of one collection.

DataSheetControls provides flexible surrounding controls.

SearchBlock composes them without duplicating collection behavior.

RecordItems may change appearance between modes, but their identity,
ordering, filtering, pagination, and selection state remain consistent.
