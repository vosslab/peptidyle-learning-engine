//! Tenant-owned course roster, invitation, and learner-identity contract.
//!
//! Account credentials remain behind [`crate::AccountIdentityStore`]. This
//! module owns only the protected operational metadata needed to invite a
//! learner, authorize course activity, and match manual grade exports.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use async_trait::async_trait;
use objects::Sha256Digest;
use question_model::{ActivityTimestamp, CourseId, StudentId, TenantId, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AuthenticationEmail, EmailDomain, Page, PageRequest, SessionTokenHash, StoreError,
    TenantContext,
};

#[path = "course_roster/import.rs"]
mod import;
pub use import::*;

/// Maximum institution-supplied roster identifier length.
pub const MAX_COURSE_ROSTER_ID_CHARS: usize = 64;
/// Maximum invitation idempotency-key bytes.
pub const MAX_ROSTER_IDEMPOTENCY_KEY_BYTES: usize = 128;
/// Maximum configured exact email domains per course.
pub const MAX_ALLOWED_EMAIL_DOMAINS: usize = 32;

macro_rules! roster_id_type {
    ($name:ident, $label:literal) => {
        #[doc = $label]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Result<Self, StoreError> {
                crate::random_uuid::random_uuid_v4(|error| {
                    StoreError::Unavailable(format!("{} randomness unavailable: {error}", $label))
                })
                .map(Self)
            }

            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }
    };
}

roster_id_type!(CourseMemberId, "course member ID");
roster_id_type!(CourseInvitationId, "course invitation ID");

/// One-way invitation token hash. The raw token remains in the delivery URL.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseInvitationSecretHash([u8; 32]);

impl CourseInvitationSecretHash {
    pub fn compute(secret: &[u8]) -> Self {
        Self(*Sha256Digest::compute(secret).as_bytes())
    }

    pub fn from_bytes(value: [u8; 32]) -> Self {
        Self(value)
    }

    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl std::fmt::Debug for CourseInvitationSecretHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CourseInvitationSecretHash([redacted])")
    }
}

/// Strong positive optimistic-concurrency token for one course roster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RosterRevision(u64);

impl RosterRevision {
    pub const INITIAL: Self = Self(1);

    pub fn from_stored(value: i64) -> Result<Self, StoreError> {
        u64::try_from(value)
            .ok()
            .filter(|value| *value > 0)
            .map(Self)
            .ok_or_else(|| StoreError::Unavailable("stored roster revision is invalid".to_string()))
    }

    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Result<Self, StoreError> {
        self.0.checked_add(1).map(Self).ok_or(StoreError::Conflict)
    }
}

/// Instructor-supplied identifier used only inside one course and its exports.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseRosterId(String);

impl CourseRosterId {
    pub fn parse(value: &str) -> Result<Self, CourseRosterError> {
        let value = value.trim();
        if value.is_empty()
            || value.chars().count() > MAX_COURSE_ROSTER_ID_CHARS
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(CourseRosterError::InvalidRosterId);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for CourseRosterId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CourseRosterId([protected])")
    }
}

/// Exact domain rule; subdomains require an explicit opt-in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllowedEmailDomain {
    pub domain: EmailDomain,
    pub include_subdomains: bool,
}

impl AllowedEmailDomain {
    pub fn allows(&self, email: &AuthenticationEmail) -> bool {
        email.domain() == &self.domain
            || (self.include_subdomains
                && email
                    .domain()
                    .as_str()
                    .strip_suffix(self.domain.as_str())
                    .is_some_and(|prefix| prefix.ends_with('.') && prefix.len() > 1))
    }
}

/// Closed signup posture. Empty domains are valid only for invitation-only use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseSignupPosture {
    InvitationOnly,
    PermittedDomains,
}

/// Revisioned enrollment policy for one course.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseEnrollmentPolicy {
    pub course: CourseId,
    pub allowed_domains: BTreeSet<AllowedEmailDomain>,
    pub signup_posture: CourseSignupPosture,
    pub revision: RosterRevision,
}

impl CourseEnrollmentPolicy {
    pub fn validates(&self, email: &AuthenticationEmail) -> bool {
        self.allowed_domains.is_empty()
            || self.allowed_domains.iter().any(|rule| rule.allows(email))
    }

    pub fn validate_shape(&self) -> Result<(), CourseRosterError> {
        if self.allowed_domains.len() > MAX_ALLOWED_EMAIL_DOMAINS
            || (self.signup_posture == CourseSignupPosture::PermittedDomains
                && self.allowed_domains.is_empty())
        {
            return Err(CourseRosterError::InvalidEnrollmentPolicy);
        }
        Ok(())
    }
}

/// Current access state retained beside the protected roster record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseMemberStatus {
    Active,
    Revoked,
}

/// Protected course-local projection for one claimed learner.
#[derive(Clone, PartialEq, Eq)]
pub struct CourseRosterMember {
    pub id: CourseMemberId,
    pub tenant: TenantId,
    pub course: CourseId,
    pub user: UserId,
    pub student: StudentId,
    pub display_name: String,
    /// Course-local invitation/export email, when the roster record has one.
    pub roster_email: Option<AuthenticationEmail>,
    /// Course-local institutional export key, when the roster record has one.
    pub roster_id: Option<CourseRosterId>,
    pub status: CourseMemberStatus,
    pub joined_at: ActivityTimestamp,
    pub revoked_at: Option<ActivityTimestamp>,
}

impl std::fmt::Debug for CourseRosterMember {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseRosterMember")
            .field("id", &self.id)
            .field("tenant", &self.tenant)
            .field("course", &self.course)
            .field("user", &self.user)
            .field("student", &self.student)
            .field("display_name", &self.display_name)
            .field("roster_email", &"[protected]")
            .field("roster_id", &"[protected]")
            .field("status", &self.status)
            .field("joined_at", &self.joined_at)
            .field("revoked_at", &self.revoked_at)
            .finish()
    }
}

/// Current state of a course invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInvitationStatus {
    Pending,
    Claimed,
    Revoked,
    Expired,
}

/// Protected pending/consumed invitation metadata; raw secrets are absent.
#[derive(Clone, PartialEq, Eq)]
pub struct CourseInvitation {
    pub id: CourseInvitationId,
    pub tenant: TenantId,
    pub course: CourseId,
    pub email: AuthenticationEmail,
    pub roster_id: CourseRosterId,
    pub invited_by: UserId,
    pub status: CourseInvitationStatus,
    pub created_at: ActivityTimestamp,
    pub expires_at: ActivityTimestamp,
    pub claimed_by: Option<UserId>,
}

impl std::fmt::Debug for CourseInvitation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CourseInvitation")
            .field("id", &self.id)
            .field("tenant", &self.tenant)
            .field("course", &self.course)
            .field("email", &"[protected]")
            .field("roster_id", &"[protected]")
            .field("invited_by", &self.invited_by)
            .field("status", &self.status)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .field("claimed_by", &self.claimed_by)
            .finish()
    }
}

/// One stable item in the combined roster page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseRosterEntry {
    Member(CourseRosterMember),
    Invitation(CourseInvitation),
}

/// Roster page and policy read from one backend snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRosterPage {
    pub entries: Page<CourseRosterEntry>,
    pub policy: CourseEnrollmentPolicy,
}

/// Bounded invitation lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseInvitationLifetime(NonZeroU32);

impl CourseInvitationLifetime {
    pub const MAX_SECONDS: u32 = 30 * 24 * 60 * 60;

    pub fn from_seconds(seconds: u32) -> Option<Self> {
        NonZeroU32::new(seconds)
            .filter(|seconds| seconds.get() <= Self::MAX_SECONDS)
            .map(Self)
    }

    pub fn as_seconds(self) -> u32 {
        self.0.get()
    }
}

/// Validated, bounded idempotency key for one invitation request.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RosterIdempotencyKey(String);

impl RosterIdempotencyKey {
    pub fn parse(value: &str) -> Result<Self, CourseRosterError> {
        if value.is_empty()
            || value.len() > MAX_ROSTER_IDEMPOTENCY_KEY_BYTES
            || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        {
            return Err(CourseRosterError::InvalidIdempotencyKey);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for RosterIdempotencyKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RosterIdempotencyKey([redacted])")
    }
}

/// Instructor-authorized request to create an indistinguishable invitation row.
#[derive(Clone)]
pub struct CreateCourseInvitation {
    pub course: CourseId,
    pub email: AuthenticationEmail,
    pub roster_id: CourseRosterId,
    pub token_hash: CourseInvitationSecretHash,
    pub idempotency_key: RosterIdempotencyKey,
    pub lifetime: CourseInvitationLifetime,
}

impl std::fmt::Debug for CreateCourseInvitation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CreateCourseInvitation")
            .field("course", &self.course)
            .field("email", &"[protected]")
            .field("roster_id", &"[protected]")
            .field("lifetime", &self.lifetime)
            .finish_non_exhaustive()
    }
}

/// Revision-checked enrollment-policy replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceCourseEnrollmentPolicy {
    pub course: CourseId,
    pub expected_revision: RosterRevision,
    pub allowed_domains: BTreeSet<AllowedEmailDomain>,
    pub signup_posture: CourseSignupPosture,
}

/// Authenticated learner request to consume one invitation secret.
#[derive(Debug, Clone)]
pub struct ClaimCourseInvitation {
    pub token_hash: CourseInvitationSecretHash,
    pub user: UserId,
    pub verified_email: AuthenticationEmail,
    pub display_name: String,
}

/// Canonical optional contact identity for one roster member.
#[derive(Debug, Clone)]
pub struct CourseRosterContact {
    /// Verified email address used for roster matching and delivery.
    pub email: AuthenticationEmail,
    /// Instructor-facing identifier unique within the course roster.
    pub roster_id: CourseRosterId,
}

/// One canonical roster member activation. It owns the roster profile and
/// `course_member` episode in one Store transaction.
#[derive(Debug, Clone)]
pub struct UpsertCourseMember {
    /// Course receiving the canonical roster member.
    pub course: CourseId,
    /// Authenticated or configured user receiving student access.
    pub user: UserId,
    /// Display name shown on the roster and gradebook.
    pub display_name: String,
    /// Optional roster contact; configured local learners have no email identity.
    pub roster_contact: Option<CourseRosterContact>,
}

/// Atomic claim result; no credential or unrelated course state is included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedCourseMembership {
    pub tenant: TenantId,
    pub course: CourseId,
    pub member: CourseRosterMember,
    pub roster_revision: RosterRevision,
}

/// Revision-checked access revocation. Educational records remain retained.
#[derive(Debug, Clone, Copy)]
pub struct RevokeCourseMember {
    pub course: CourseId,
    pub member: CourseMemberId,
    pub expected_revision: RosterRevision,
}

/// Revision-checked cancellation of one still-pending invitation.
#[derive(Debug, Clone, Copy)]
pub struct RevokeCourseInvitation {
    pub course: CourseId,
    pub invitation: CourseInvitationId,
    pub expected_revision: RosterRevision,
}

/// Roster validation error suitable for a safe `422` response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseRosterError {
    InvalidRosterId,
    InvalidIdempotencyKey,
    InvalidEnrollmentPolicy,
}

/// One closed Sysadmin roster-support operation recorded before protected data
/// is returned or changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CourseRosterSupportAction {
    ListRoster,
    CreateInvitation,
    ReplaceEnrollmentPolicy,
    RevokeMember,
    RevokeInvitation,
    StageImport,
    CommitImport,
}

/// Audit evidence created only when Sysadmin authority, rather than direct
/// Instructor membership, opens the roster-support boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseRosterSupportAudit {
    pub tenant: TenantId,
    pub course: CourseId,
    pub actor: UserId,
    pub action: CourseRosterSupportAction,
    pub occurred_at: ActivityTimestamp,
}

impl std::fmt::Display for CourseRosterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidRosterId => "course roster identifier is invalid",
            Self::InvalidIdempotencyKey => "roster idempotency key is invalid",
            Self::InvalidEnrollmentPolicy => "course enrollment policy is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CourseRosterError {}

/// Focused tenant roster persistence and atomic claim boundary.
///
/// Direct Instructors own these operations. The list/invite/policy/revoke/
/// import methods also accept the closed Sysadmin roster-support authority and
/// must durably audit that exceptional boundary. Invitation claim and the
/// provisioning-only member upsert does not consume Sysadmin support authority;
/// instead, it requires the explicit actor to be a direct active Instructor in
/// the exact course.
#[async_trait]
pub trait CourseRosterStore: Send + Sync {
    async fn list_course_roster(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        course: CourseId,
        page: PageRequest,
    ) -> Result<CourseRosterPage, StoreError>;

    async fn create_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CreateCourseInvitation,
    ) -> Result<CourseInvitation, StoreError>;

    async fn replace_course_enrollment_policy(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: ReplaceCourseEnrollmentPolicy,
    ) -> Result<CourseEnrollmentPolicy, StoreError>;

    /// Resolves tenant/course only from the hashed invitation capability.
    async fn claim_course_invitation(
        &self,
        command: ClaimCourseInvitation,
    ) -> Result<ClaimedCourseMembership, StoreError>;

    async fn upsert_course_member(
        &self,
        context: TenantContext,
        actor: UserId,
        command: UpsertCourseMember,
    ) -> Result<ClaimedCourseMembership, StoreError>;

    async fn revoke_course_member(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseMember,
    ) -> Result<RosterRevision, StoreError>;

    async fn revoke_course_invitation(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: RevokeCourseInvitation,
    ) -> Result<RosterRevision, StoreError>;

    async fn stage_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: StageCourseRosterImport,
    ) -> Result<CourseRosterImportPreview, StoreError>;

    async fn commit_course_roster_import(
        &self,
        context: TenantContext,
        session: SessionTokenHash,
        command: CommitCourseRosterImport,
    ) -> Result<CommittedCourseRosterImport, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_domain_rule_rejects_substrings_and_unapproved_subdomains() {
        let rule = AllowedEmailDomain {
            domain: EmailDomain::parse("mail.roosevelt.edu").expect("valid domain"),
            include_subdomains: false,
        };
        assert!(rule.allows(
            &AuthenticationEmail::parse("student@mail.roosevelt.edu").expect("valid email")
        ));
        assert!(
            !rule.allows(
                &AuthenticationEmail::parse("student@mail.roosevelt.edu.attacker.example")
                    .expect("valid email")
            )
        );
        assert!(!rule.allows(
            &AuthenticationEmail::parse("student@sub.mail.roosevelt.edu").expect("valid email")
        ));
    }

    #[test]
    fn explicit_subdomain_rule_respects_label_boundary() {
        let rule = AllowedEmailDomain {
            domain: EmailDomain::parse("roosevelt.edu").expect("valid domain"),
            include_subdomains: true,
        };
        assert!(rule.allows(
            &AuthenticationEmail::parse("student@mail.roosevelt.edu").expect("valid email")
        ));
        assert!(
            !rule.allows(
                &AuthenticationEmail::parse("student@notroosevelt.edu").expect("valid email")
            )
        );
    }

    #[test]
    fn roster_ids_are_course_values_not_email_or_uuid_types() {
        assert_eq!(
            CourseRosterId::parse("900123456")
                .expect("Roosevelt roster identifier")
                .as_str(),
            "900123456"
        );
        assert_eq!(
            CourseRosterId::parse("contains spaces"),
            Err(CourseRosterError::InvalidRosterId)
        );
    }
}
