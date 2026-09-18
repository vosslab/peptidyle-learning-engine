-- Privileges from library_discussions.sql.

SET LOCAL ROLE ple_private_owner;

GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON TABLE ple_data.library_improvement_thread,
    ple_data.library_improvement_post, ple_data.library_impact_notice FROM PUBLIC;

