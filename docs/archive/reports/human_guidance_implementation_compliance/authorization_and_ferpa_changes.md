# Authorization and FERPA changes

Implementation findings for Accounts, roles, and Student-record access. Open
readings are recorded once in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md).

## Shipped boundaries

- An Account has one User Role: Student, Instructor, or Sysadmin.
- Course membership decides which private Course records an Instructor may use.
- A Sysadmin does not automatically receive FERPA Course records. Support access is
  a scoped read.
- Support repair is an Instructor-issued capability. It lasts one hour, is audited,
  and reads one Student roster entry, one Course, or one Course Assessment. The read
  returns identity, type, title, and status. It does not grant membership and does
  not mutate Course content.
- Students reach Question content through Coursework. Answers and grading stay on
  the server.
- FERPA retention follows the Course Instance dates and the six-month Active
  lifetime. Retention intervals are operational configuration.
- Removing or deactivating a Student revokes future Course access and keeps the
  Account and Student Work until retention says otherwise.

## Open readings in this area

- Position 475 asks whether Instructor approval is the completed vetting decision,
  with later access changed only by deactivate and reactivate, or a separate
  post-creation Account status.
- Positions 469, 479, and 481 ask where system configuration, installation-wide
  settings, and rare configuration tasks belong. No empty settings page was added.
- Position 484 leaves only the complete Sysadmin Ribbon task layout unlocked.

Accounts and roles: 44 verified, 0 open, 8 not applicable.
