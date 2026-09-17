The worst confirmed repetitions are:

1. Watch notifications duplicate the entire parent event for every recipient - target, object ID, event kind, revision, activity, and timestamp - even though each row already stores event_id. The notification should retain only recipient + event identity and join the event. This is multiplicative fan-out. See schemas/base_schema/question_watch_notifications.sql:69.

2. Every Assessment Attempt repeats nine policy strings: late-work, variation, ordering, and six feedback-release fields. Snapshotting the policy is correct, but repeating nine strings per attempt is not. Attempts should reference one immutable policy/configuration row whose closed values use enums. See schemas/base_schema/assessment_attempts.sql:50.

3. question_response_grading.grading_state is always exactly graded. It stores no information and should disappear, likely along with the unnecessary one-to-one wrapper table. See schemas/base_schema/grading.sql:7.

4. assessment_submission.finalization_kind is derivable from authorized_by_account_id, while its JSON receipt repeats both that kind and constant submitted state. Those duplicates should be removed and projected when needed. See schemas/base_schema/assessment_attempt_interaction.sql:53.

5. question_attempt_state duplicates facts already represented by Question Response existence, Assessment Submission existence, and timestamps. It should be derived instead of stored for every Question Attempt. See schemas/base_schema/assessment_attempt_interaction.sql:5.

6. issued_question.scoring_rule repeats one of four strings for every issued Question. Its snapshot is legitimate, but its representation should be a shared enum or part of an immutable entry-policy reference. See schemas/base_schema/assessment_attempts.sql:111.

There are also many smaller closed-text fields - roles, states, event kinds, backends, availability, MIME types, and object kinds. Those deserve native enums when genuinely closed. Single-value columns such as fixed roles, job kinds, and rendition kinds should often be removed entirely because they encode no information.

I would prioritize the notification fan-out and Assessment Attempt policy bundle first; those multiply fastest. Free prose, immutable authored snapshots, canonical public IDs, external identifiers, and audit reasons should remain textual.

The broad database term is data redundancy, specifically redundant storage of derivable or functionally dependent data.

Several more precise terms apply to your examples:

* Denormalization: storing values repeatedly that could be represented once and referenced. #1 and #2 are strong examples.
* Transitive redundancy / normalization violation: a row stores attributes that really depend on another referenced entity. #1 fits: notification -> event_id -> event attributes.
* Derived-data redundancy: storing a value that can be computed from other stored facts. #4 and #5.
* Constant-column redundancy: storing the same value in every row, meaning the column carries no information. #3 and your fixed-role/job-kind examples.
* Repeated categorical values: storing a small closed vocabulary as strings repeatedly. #2 and #6. Enums or referenced configuration rows can reduce this, although this is not necessarily a normalization violation by itself.

For the overall problem you are describing, I would probably call it excessive data redundancy from denormalization, with fan-out amplification for the particularly bad notification case.

Your phrase multiplicative fan-out is also quite good for #1. It describes why that particular redundancy becomes expensive, rather than the underlying database-design issue.

Yes. More specifically, I would call that stringly typed schema design.

That term captures the design problem better than "data redundancy":

Stringly typed schema: representing a small, closed domain of valid values as arbitrary text rather than encoding the domain in the schema.

For example, storing "graded" 10 million times is worse than merely repetitive. The database is being asked to store and validate text for something that is actually a finite state/category.

For your audit, I would distinguish:

* Stringly typed closed domains: role, state, event_kind, backend, etc. Use enums or another constrained representation.
* Constant columns: only one legitimate value. Remove the column.
* Derivable columns: value follows from other stored data. Derive it.
* Snapshot configuration: legitimately frozen per event/attempt, but preferably reference one immutable configuration rather than copying a bundle of categorical strings.

"Stringly typed schema design" is probably the phrase you are looking for when criticizing the repeated closed-text fields specifically.
