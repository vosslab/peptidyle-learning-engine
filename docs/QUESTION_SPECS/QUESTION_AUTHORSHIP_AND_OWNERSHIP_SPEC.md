# Question owners and authors specification

Owner identifies responsibility and authority to change owned content. Author identifies who
created content. These are different facts. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#published-question-revisions-edits-and-forks) and
[AUTHORIZATION_CONTRACTS.md](../AUTHORIZATION_CONTRACTS.md).

## Published Questions

A Published Question has an owner and authors. Only its owning
Instructor or a Sysadmin may edit it. Every Instructor can read any Published Question, add it to
an Assessment, or fork it. Content changes create a new Question Revision; search-metadata edits
preserve the current Revision. Another Instructor can fork the Question into a private
Draft and publish a separate Question with a new ID. The fork starts with the source Question's
license, authors, metadata, and Question content, and records the source Question Revision Tuple.

Question authors may have PLE Accounts, but source authors need not. Each author has a display
name and may have an Account reference. A new Revision carries forward the Question record's
fields except for the changes being published. Use the exact Revision's authors and citation
when interpreting or exporting it. Citation is optional text; see
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md#citation).

## Question Pools

A Pool has an owner and, if forked, a source-Pool reference. It has no separate Author field.
Each Published Question in a Pool retains its own owner and authors; those may differ from each other and from the
Pool owner. Selecting Questions into a Pool does not transfer their ownership.

A Pool fork is a new Question Pool with its own Pool ID and owner, and records its source Pool.
It can be used in any number of Assessments. See [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).

## Actions and authority

| Action | Authority |
| --- | --- |
| Discover Library Objects | Every Instructor and Sysadmin |
| Edit a private Draft | Its authorized authoring owner |
| Publish a new Question Revision | Owning Instructor or Sysadmin |
| Read a Published Question or add it to an Assessment | Every Instructor |
| Fork a Library Object | Instructor with access to the source and permitted license |
| Change which Published Questions a Pool contains | Pool owner or Sysadmin |
| Correct Bloom | Owning Instructor or Sysadmin |
| Edit a Question's Title, Description, Discipline, Subject, Topic, Subtopic, and Tags | Owning Instructor or Sysadmin |
| View content as Student | Authorized Coursework access, not direct Library authoring access |

Search-metadata editing does not grant permission to replace another Instructor's Question
source. Neil expects Instructors to do the bare minimum of metadata writing, so PLE should assign
metadata automatically where possible. Initial AI Bloom assignment remains deferred. That work
does not establish an additional editor role. Instructor bulk editing remains deferred; its
existing API does not establish editing rights for other Instructors.

Instructors are vetted before account creation. Once in PLE, all Instructors are equal; there is
no Verified Instructor tier. Sysadmins have full administrative authority across PLE, including
Questions and Pools; the Instructor ownership rules do not limit that authority. Much of the
concrete Sysadmin tooling remains deferred under HG. This authority does not make new tools a
current implementation requirement.

## Attribution in discovery

Question results may show their authors separately from their owner. Pool results show Owner,
not an invented Pool Author or an aggregation of member authors. Author filters match Question
authors; Owner filters can match either object kind's own owner.

Instructor Profiles are visible to every PLE Account. Show the Instructor Profile image when
viewing Question authors or Question Pool owners, as HG requires. Images have no separate
permissions mechanism. Library access still controls Library
identity lists such as who Starred. See [QUESTION_LIBRARY_SPEC.md](QUESTION_LIBRARY_SPEC.md).
Profile visibility does not give Students access to Question source or Library searches.

For example, Elena may own a Pool containing Questions owned by Amir and Lin. Elena is the Pool
owner. Amir and Lin remain their Questions' owners, and every Question keeps its authors.
Forking the Pool changes the new Pool's owner and ID, not any member's owner or author.
