#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for passwordless identity and invitation RLS.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AccountIdentityStore, AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash,
    AuthenticationEmail, AuthenticationRateLimitDecision, AuthenticationRateLimitKey,
    AuthenticationRateLimitPolicy, AuthenticationRateLimitScope, BeginEmailAuthentication,
    BeginWebauthnCeremony, BrowserBindingHash, ClaimCourseInvitation, CommitCourseRosterImport,
    CompleteEmailAuthentication, CompleteEmailChangeAndRevokeUserSessions,
    CompletePasskeyAuthenticationAndCreateSession, ConsumeAuthenticationRateLimit,
    CourseInvitationDeliveryState, CourseInvitationDeliveryStore, CourseInvitationLifetime,
    CourseInvitationSecretHash, CourseRosterId, CourseRosterImportLifetime,
    CourseRosterImportRowInput, CourseRosterStore, CourseSignupPosture, CreateCourseInvitation,
    CredentialIdHash, EmailAuthenticationPurpose, EmailChallengeId, EmailChallengeLifetime,
    EmailChallengeSecretHash, PageRequest, PageSize, PasskeyId, PasskeyRecord, RegisterPasskey,
    ReplaceCourseEnrollmentPolicy, RevokeCourseInvitation, RevokeCourseMember,
    RosterIdempotencyKey, RosterImportInvitation, SessionLifetime, SessionStore, SessionSubject,
    SessionTokenHash, StageCourseRosterImport, StoreError, TenantContext, UpsertCourseMember,
    WebauthnCeremonyId, WebauthnCeremonyKind, WebauthnCeremonyLifetime, WebauthnState,
};
use objects::Sha256Digest;
use question_model::{CourseId, CourseMembershipRole, TenantId, UserId, UserRole};
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn database_error_code(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.into_owned())
}

include!("postgres_enrollment_live/course_member_upsert.rs");
include!("postgres_enrollment_live/account_course_context.rs");
include!("postgres_enrollment_live/email_change_rollback.rs");
include!("postgres_enrollment_live/enrollment_capability.rs");
include!("postgres_enrollment_live/expired_invitation_replay.rs");
include!("postgres_enrollment_live/invitation_claim_broker.rs");
include!("postgres_enrollment_live/passwordless_challenge.rs");
