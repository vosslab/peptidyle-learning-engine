//! Connected proof that persistence keeps a canonical public ID unchanged.

#![cfg(feature = "postgres")]

use sqlx::{Connection, Row};

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance runtime"]
async fn persistence_does_not_reformat_a_canonical_public_id() {
    let runtime = acceptance_runtime::AcceptanceRuntime::load().expect("acceptance runtime");
    let mut connection = sqlx::PgConnection::connect(runtime.migration_url().expose())
        .await
        .expect("migration connection");
    sqlx::query("SET ROLE ple_private_owner")
        .execute(&mut connection)
        .await
        .expect("private owner");

    let stored_account: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('UABCDEFGM', 'student', pg_catalog.transaction_timestamp()) \
         RETURNING account_id::text",
    )
    .fetch_one(&mut connection)
    .await
    .expect("supplied Account ID");
    assert_eq!(stored_account, "UABCDEFGM");

    let reread_account: String = sqlx::query_scalar(
        "SELECT account_id::text FROM ple_private.account WHERE account_id = 'UABCDEFGM'",
    )
    .fetch_one(&mut connection)
    .await
    .expect("reread Account ID");
    assert_eq!(reread_account, "UABCDEFGM");

    let minted_account: String = sqlx::query_scalar(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('U00000009', 'student', pg_catalog.transaction_timestamp()) \
         RETURNING account_id::text",
    )
    .fetch_one(&mut connection)
    .await
    .expect("minted Account ID");
    assert_ne!(minted_account, "U00000009");
    assert_eq!(minted_account.len(), 9);
    assert!(minted_account.starts_with('U'));
    assert!(
        minted_account[1..]
            .chars()
            .all(|character| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(character)),
        "minted Account ID {minted_account}"
    );

    let lowercase_account = sqlx::query(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('uabcdefgm', 'student', pg_catalog.transaction_timestamp())",
    )
    .execute(&mut connection)
    .await
    .expect_err("lowercase Account ID");
    assert_eq!(
        lowercase_account
            .as_database_error()
            .and_then(|database| database.code().map(|code| code.to_string())),
        Some("23514".to_string())
    );

    let uuid_account = sqlx::query(
        "INSERT INTO ple_private.account (account_id, user_role, created_at) \
         VALUES ('00000000-0000-0000-0000-000000000001', 'student', \
                 pg_catalog.transaction_timestamp())",
    )
    .execute(&mut connection)
    .await
    .expect_err("UUID Account ID");
    assert_eq!(
        uuid_account
            .as_database_error()
            .and_then(|database| database.code().map(|code| code.to_string())),
        Some("23514".to_string())
    );

    sqlx::query("SET ROLE ple_data_owner")
        .execute(&mut connection)
        .await
        .expect("data owner");
    let row = sqlx::query(
        "SELECT 'ABCD-XEFG'::ple_data.question_family_id::text, \
                'CIABCDEFGS'::ple_data.course_instance_id::text, \
                'AABCDEFG8'::ple_data.assessment_id::text, \
                'BPABCDEFGJ'::ple_data.blueprint_course_id::text",
    )
    .fetch_one(&mut connection)
    .await
    .expect("canonical public ID domains");
    assert_eq!(row.get::<String, _>(0), "ABCD-XEFG");
    assert_eq!(row.get::<String, _>(1), "CIABCDEFGS");
    assert_eq!(row.get::<String, _>(2), "AABCDEFG8");
    assert_eq!(row.get::<String, _>(3), "BPABCDEFGJ");

    let lowercase_question = sqlx::query("SELECT 'abcd-xefg'::ple_data.question_family_id")
        .execute(&mut connection)
        .await
        .expect_err("lowercase Question ID");
    assert_eq!(
        lowercase_question
            .as_database_error()
            .and_then(|database| database.code().map(|code| code.to_string())),
        Some("23514".to_string())
    );
}
