-- Privileges from question_images.sql.

SET LOCAL ROLE ple_data_owner;

REVOKE ALL ON ple_data.question_image_delivery FROM PUBLIC;

SET LOCAL ROLE ple_private_owner;


-- Object Delivery relations are owned by ple_data_owner.  The validation
-- routine remains private-owner code, while the table owner installs its
-- two cross-table deferred triggers.
GRANT EXECUTE ON FUNCTION ple_private.require_complete_question_image_publication_delivery() TO ple_data_owner;

REVOKE ALL ON ple_private.question_image_publication FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.reject_question_image_publication_change(),
    ple_private.validate_question_image_publication(), ple_private.require_complete_question_image_publication_delivery() FROM PUBLIC;

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, INSERT, UPDATE ON ple_data.object_delivery, ple_data.question_image_delivery TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.select_ready_question_image_renditions(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.select_ready_question_image_renditions(text, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.select_ready_question_image_renditions(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.select_ready_question_image_renditions(text, integer) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.load_question_fork_asset(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.load_question_fork_asset(text, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.load_question_fork_asset(text, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.load_question_fork_asset(text, integer) TO ple_app;

