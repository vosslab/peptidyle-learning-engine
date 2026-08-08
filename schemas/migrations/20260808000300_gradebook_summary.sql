-- MOD-UI-GRADEBOOK: support bounded summary-only course pages.
--
-- The gradebook page joins assignment -> enrollment -> maintained summary.
-- This index follows that access path and does not involve historical runs or
-- attempts, whose volume grows with continued practice.
CREATE INDEX enrollment_gradebook_summary_page_idx
    ON enrollment (tenant_id, assignment_id, enrollment_id);
