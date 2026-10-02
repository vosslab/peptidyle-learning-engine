-- Privileges from content_classification_operations.sql.

SET LOCAL ROLE ple_data_owner;

GRANT SELECT, INSERT ON ple_data.content_discipline, ple_data.content_subject,
    ple_data.content_subject_discipline, ple_data.content_topic, ple_data.content_subtopic,
    ple_data.content_discipline_request
    TO ple_private_owner;

-- ASVS 8.2.3: resolving a request changes only its open timestamp.
GRANT UPDATE (resolved_at) ON ple_data.content_discipline_request TO ple_private_owner;

GRANT DELETE ON ple_data.content_subject_discipline TO ple_private_owner;


-- SELECT FOR UPDATE needs UPDATE privilege; no private command edits Subject rows.
GRANT UPDATE (name, is_retired) ON ple_data.content_discipline TO ple_private_owner;

GRANT UPDATE ON ple_data.content_subject TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.find_content_subject(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.find_content_subject(text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.require_content_classification_actor(boolean),
    ple_private.require_content_classification_reader(),
    ple_private.normalize_content_classification_name(text, integer) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.add_content_subject_discipline(uuid, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.add_content_subject_discipline(uuid, uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.create_content_discipline(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.create_content_discipline(text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.rename_content_discipline(uuid, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.rename_content_discipline(uuid, text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.retire_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.retire_content_discipline(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.restore_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.restore_content_discipline(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.require_active_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.require_active_content_discipline(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.create_content_subject(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.create_content_subject(text, uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.create_content_topic(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.create_content_topic(text, uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.create_content_subtopic(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.create_content_subtopic(text, uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.replace_content_subject_disciplines(uuid, uuid[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.replace_content_subject_disciplines(uuid, uuid[]) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.list_content_disciplines() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_content_disciplines() TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.list_content_disciplines_including_retired() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_content_disciplines_including_retired() TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.get_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.get_content_discipline(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.list_content_subjects(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_content_subjects(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.list_content_topics(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_content_topics(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.list_content_subtopics(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_content_subtopics(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.request_content_discipline(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.request_content_discipline(text) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.list_open_content_discipline_requests() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.list_open_content_discipline_requests() TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.resolve_content_discipline_request(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.resolve_content_discipline_request(uuid) TO ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.fulfill_content_discipline_request(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.fulfill_content_discipline_request(uuid) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.find_content_subject(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.find_content_subject(text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.add_content_subject_discipline(uuid, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.add_content_subject_discipline(uuid, uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.create_content_discipline(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_content_discipline(text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.rename_content_discipline(uuid, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.rename_content_discipline(uuid, text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.retire_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.retire_content_discipline(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.restore_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.restore_content_discipline(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.require_active_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.require_active_content_discipline(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.create_content_subject(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_content_subject(text, uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.create_content_topic(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_content_topic(text, uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.create_content_subtopic(text, uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.create_content_subtopic(text, uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.replace_content_subject_disciplines(uuid, uuid[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.replace_content_subject_disciplines(uuid, uuid[]) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_content_disciplines() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_content_disciplines() TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_content_disciplines_including_retired() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_content_disciplines_including_retired() TO ple_app;

REVOKE ALL ON FUNCTION ple_api.get_content_discipline(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.get_content_discipline(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_content_subjects(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_content_subjects(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_content_topics(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_content_topics(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_content_subtopics(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_content_subtopics(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.request_content_discipline(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.request_content_discipline(text) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.list_open_content_discipline_requests() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.list_open_content_discipline_requests() TO ple_app;

REVOKE ALL ON FUNCTION ple_api.resolve_content_discipline_request(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.resolve_content_discipline_request(uuid) TO ple_app;

REVOKE ALL ON FUNCTION ple_api.fulfill_content_discipline_request(uuid) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.fulfill_content_discipline_request(uuid) TO ple_app;
