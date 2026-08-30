# Baseline schema dependency graph

## Evidence and construction rule

This one-time authoring record derives edges from cross-schema foreign keys,
functions, grants, policies, and projections in the direct baseline sources.
The PostgreSQL 17 clean-volume check applied the listed order from an empty
database successfully. The baseline preserves these edges while placing each
live definition in one domain-owned file.

## Ordered source list

1. `2026082901_principal_baseline.sql`
2. `2026082902_global_account_primary_session.sql`
3. `2026082903_email_challenges_and_rate_limits.sql`
4. `2026082904_webauthn_ceremonies_and_passkeys.sql`
5. `2026082905_instructor_approval.sql`
6. `2026082906_actor_resolution_broker.sql`
7. `2026082907_shared_catalog_roots.sql`
8. `2026082908_catalog_lifecycle_events.sql`
9. `2026082909_catalog_stewardship.sql`
10. `2026082910_private_authoring_roots.sql`
11. `2026082911_blueprint_course_roots.sql`
12. `2026082912_private_collections.sql`
13. `2026082913_course_instance_roots.sql`
14. `2026082914_course_memberships.sql`
15. `2026082915_student_enrollment_roots.sql`
16. `2026082916_course_delivery_schedule.sql`
17. `2026082917_assignment_runs_and_attempts.sql`
18. `2026082918_submissions_and_feedback.sql`
19. `2026082919_course_object_metadata.sql`
20. `2026082920_delivery_indexes.sql`
21. `2026082921_automated_grading_receipts.sql`
22. `2026082922_gradebook_evidence.sql`
23. `2026082923_item_course_analysis.sql`
24. `2026082924_correction_manifests.sql`
25. `2026082925_typed_jobs_and_leases.sql`
26. `2026082926_exports_retention_audit.sql`
27. `2026082927_external_tool_provider_state.sql`
28. `2026082928_object_delivery_reconciliation.sql`
29. `2026082929_capability_brokers.sql`
30. `2026082930_forced_rls_policies.sql`
31. `2026082931_acl_closure.sql`
32. `2026082932_baseline_acceptance_witness.sql`

## Actual dependency edges

| Source | Earlier prerequisites | Live owner it establishes |
| --- | --- | --- |
| 2901 | none | roles, schemas, default-deny ACLs, migration witness |
| 2902 | 2901 | global account and opaque primary session |
| 2903–2905 | 2902 | email challenges, passkeys, Instructor approval |
| 2906 | 2902 | transaction-local actor resolver |
| 2907 | 2901 | published-question roots and immutable versions |
| 2908 | 2907 | catalog lifecycle evidence |
| 2909 | 2902, 2907 | catalog proposals and stewardship evidence |
| 2910 | 2902 | private workspaces and drafts |
| 2911 | 2902 | reusable BlueprintCourse and revision roots |
| 2912 | 2902, 2907, 2910 | Instructor collections and saved searches |
| 2913 | 2911 | CourseInstance and immutable Blueprint adoption |
| 2914 | 2902, 2913 | course membership and invitation roots |
| 2915 | 2913, 2914 | Student and assignment enrollment roots |
| 2916 | 2911, 2913 | delivery schedule and release state |
| 2917 | 2907, 2915 | private runs and attempts |
| 2918 | 2917 | submissions and feedback release |
| 2919 | 2913, 2915 | private course-object metadata |
| 2920 | 2907, 2914–2916 | delivery indexes and answer-free projection |
| 2921 | 2918 | grading operation and immutable receipt |
| 2922 | 2915 | Gradebook snapshot and control evidence |
| 2923 | 2913 | course/item analysis and thresholded evidence |
| 2924 | 2902, 2907, 2913 | correction manifest and recalculation evidence |
| 2925 | 2907, 2910, 2913, 2916, 2917 | typed worker target, generation fence, and lease |
| 2926 | 2916, 2919, 2925 | export request, retention plan, and lifecycle evidence |
| 2927 | 2907, 2916, 2917 | private launch, provider cache, exchange, and passback state |
| 2928 | 2907, 2919, 2925 | delivery registry, reconciliation, cleanup, and access evidence |
| 2929 | 2902, 2910, 2913, 2915, 2925 | observer/support grants and authorization predicates |
| 2930 | 2929 | app and worker forced-RLS policies |
| 2931 | 2906, 2929, 2930 | final table, sequence, schema, and function ACLs |
| 2932 | 2930, 2931 | forced-RLS and default-deny catalog witness |

## Independent authoring groups

After 2901, account/session and catalog-root work can proceed independently.
After 2902, email, passkey, approval, actor-resolution, private-authoring,
and reusable-course roots can proceed independently. Catalog lifecycle and
stewardship follow the catalog root; collection work waits for both the catalog
and private-authoring roots. Course delivery then follows the Blueprint root;
its membership, schedule, analysis, correction, object, run, grading, and
Gradebook branches follow the edges above. Consolidation applies the ordered
list because SQL application itself remains deterministic.
