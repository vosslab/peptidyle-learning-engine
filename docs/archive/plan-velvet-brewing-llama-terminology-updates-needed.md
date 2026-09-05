# Terminology updates for plan-velvet-brewing-llama

Plan: `plan-velvet-brewing-llama.md`

## Contract coverage

Ribbon terminology is substantially complete. `docs/TERMINOLOGY_CONTRACT.md`
deliberately delegates interface vocabulary to `docs/INTERFACE_TERMINOLOGY.md`, which
defines:

- Application Shell and Ribbon;
- Ribbon Schema and Ribbon Scope;
- Product, Course Instance, and Assignment Attempt Ribbon Scopes;
- Ribbon Context, Tab, and Task Rows;
- Ribbon Context Controls, Tabs, Tasks, Task Areas, and Slots;
- Ribbon Availability: Available, Checking, and Unavailable;
- Selected, Loading, and No Selected Ribbon Tab;
- Page Action; and
- Reading and Full-width Content Layouts.

The companion contract also owns the visible destination names needed by the plan.
Ribbon geometry, slot composition, ordering, and interaction behavior remain UI design
and executable-contract concerns rather than terminology-contract content.

No new domain vocabulary is needed for the planned Ribbon architecture.

## Plan corrections before implementation

- **Instructor Approvals** -> **Instructor Accounts**.
- **Followed** -> **Watched** and **Question Watch**.
- `pendingAuthority` -> `checking`, corresponding to **Checking** Ribbon Availability.
- `productRoles` -> singular `productRole` where one Account's Product Role is meant.
- Bare course role -> **Course Membership Role**; prefer `courseMembershipRole` where
  the complete identifier is practical.
- **Question Version** -> **Question Revision**.
- **Question Catalog Entry** -> **Question Summary** where the answer-free listing View
  is meant.
- Use Product rather than `global` for a new Ribbon scope discriminator.
- Use `ribbon` consistently instead of alternating between `ribbon` and `chrome` for
  the route-contract member.
- Use `RibbonTaskDefinition` consistently instead of `CommandDefinition` for Ribbon
  navigation.
- Use **Ribbon Task Area** rather than command cluster when naming the declared
  presentation grouping.

## Terms that do not need promotion

Keep these as local technical or presentation vocabulary unless implementation gives
them a wider domain meaning:

- Route Scope Identity;
- membership index;
- cached reference resolution;
- module graph;
- identity, geometry, and reachability oracles; and
- course title and theme-loading presentation state.

The session's `courseMemberships` field may remain an exact browser/API View over
current Course Memberships. Its bounded selection rule belongs in the API contract; it
does not require a new PLE domain object.

## Completion boundary

This plan is terminology-ready when its examples, milestones, proposed identifiers,
and visible labels agree with `docs/INTERFACE_TERMINOLOGY.md` and no longer reintroduce
retired Instructor Approval, Followed, Question Version, generic role, or global-scope
wording.
