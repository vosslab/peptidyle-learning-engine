# Authorization and FERPA changes

Temporary working report for the corpus-wide Human Guidance compliance pass.

## Resolved authorization model

- Every Account has one immutable Product Role: Student, Instructor, or Sysadmin.
- A person needing multiple Product Roles uses separate Accounts.
- Every current Course co-Instructor has equal teaching and FERPA authority for that Course. The
  creator or first Instructor has no extra privilege, and every Course Instance has at least one
  assigned Instructor.
- A Student Account is global. Course membership and the Student Record scope access to one Course;
  removing or deactivating Course access preserves the Account and Student Work.
- Instructors may bulk add Students through roster import. They remove Students individually; PLE
  has no bulk Student-removal or roster-replacement workflow.
- Students and Instructors use passkeys or email codes. Student institutional email is immutable,
  and an Instructor may reset Course login access and issue a new signup code without creating a new
  Student identity.
- Instructor Account deactivation preserves authorship, Course relationships, and history;
  reactivation restores the same Account and Product Role. No permanent Account-closure workflow is
  currently defined.
- Sysadmin has platform-administration capability but no ambient Course membership or FERPA access.
  Support access is deliberate, scoped to the task, and recorded.
- Future Course Observer, Student Observer, and Grader are Course relationships rather than Product
  Roles. Grader is not currently needed because Assessment grading is automatic.

## Blueprint authorization changes

- Only the Blueprint owner saves content or changes lifecycle state.
- Every vetted Instructor may read Public Blueprints and explicitly included Archived Blueprints;
  only Public Blueprints are adoptable.
- Daughter Course co-Instructors approve offered changes to existing Assessments. Automatic copying
  of a newly added Blueprint Assessment does not grant the Blueprint owner daughter-Course access.
- Any vetted Instructor may submit a Blueprint Course Change Proposal; only the receiving owner
  accepts changes into a new Revision.
- Stars are visible to vetted Instructors; Watch membership is private.

## FERPA and deletion

Student Work includes Attempts, saved-response finalization evidence, immutable credit outcomes,
and the minimum evidence needed to interpret them. Assessment Unrelease deletes the
Assessment-owned Student Work only after equal co-Instructor authorization, Released state,
current concurrency precondition, and typed Assessment-title confirmation. A Course Instance becomes
Inactive six months after creation. That limit prevents Course reuse or deadline extensions from
indefinitely delaying FERPA retention, but inactivity does not itself delete Student records. The
latest Assessment deadline starts the FERPA clock; starting it does not itself archive or remove
Student data. The configured policy later determines notice, removal from normal interfaces,
recovery, and permanent deletion while preserving teaching content and privacy-safe aggregate
Question statistics.
