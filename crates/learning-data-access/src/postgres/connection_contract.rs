//! Closed PostgreSQL login and capability-role authority contracts.

use serde::Deserialize;

use super::ProductionLoginProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoginAuthority {
    pub(super) current_user: String,
    pub(super) session_user: String,
    pub(super) superuser: bool,
    pub(super) create_database: bool,
    pub(super) create_role: bool,
    pub(super) inherit: bool,
    pub(super) replication: bool,
    pub(super) bypass_rls: bool,
    pub(super) can_login: bool,
    pub(super) direct_memberships: Vec<DirectMembership>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CapabilityAuthority {
    pub(super) role_name: String,
    pub(super) superuser: bool,
    pub(super) create_database: bool,
    pub(super) create_role: bool,
    pub(super) inherit: bool,
    pub(super) replication: bool,
    pub(super) bypass_rls: bool,
    pub(super) can_login: bool,
    pub(super) direct_memberships: Vec<DirectMembership>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct DirectMembership {
    pub(super) role_name: String,
    pub(super) admin_option: bool,
    pub(super) inherit_option: bool,
    pub(super) set_option: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExpectedMembership {
    pub(super) role_name: &'static str,
    pub(super) set_option: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LoginContract {
    Production(ProductionLoginProfile),
}

impl LoginContract {
    pub(super) fn expected_login(self) -> &'static str {
        match self {
            Self::Production(ProductionLoginProfile::Api) => "ple_api_login",
            Self::Production(ProductionLoginProfile::AssessmentAttemptExpiryWorker) => {
                "ple_worker_login"
            }
            Self::Production(ProductionLoginProfile::CourseRetentionExecutor) => {
                "ple_course_retention_login"
            }
            Self::Production(ProductionLoginProfile::CourseRetentionNotifier) => {
                "ple_course_retention_notifier_login"
            }
            Self::Production(ProductionLoginProfile::PublicAssetPublisher) => "ple_publisher_login",
        }
    }

    pub(super) fn expected_memberships(self) -> &'static [ExpectedMembership] {
        match self {
            Self::Production(ProductionLoginProfile::Api) => &[
                ExpectedMembership {
                    role_name: "ple_app",
                    set_option: true,
                },
                ExpectedMembership {
                    role_name: "ple_auth",
                    set_option: true,
                },
            ],
            Self::Production(ProductionLoginProfile::AssessmentAttemptExpiryWorker) => {
                &[ExpectedMembership {
                    role_name: "ple_assessment_attempt_expiry_worker",
                    set_option: true,
                }]
            }
            Self::Production(ProductionLoginProfile::CourseRetentionExecutor) => {
                &[ExpectedMembership {
                    role_name: "ple_course_retention_executor",
                    set_option: true,
                }]
            }
            Self::Production(ProductionLoginProfile::CourseRetentionNotifier) => &[],
            Self::Production(ProductionLoginProfile::PublicAssetPublisher) => {
                &[ExpectedMembership {
                    role_name: "ple_public_asset_publisher",
                    set_option: true,
                }]
            }
        }
    }

    pub(super) fn expected_capabilities(self) -> &'static [ExpectedMembership] {
        self.expected_memberships()
    }

    pub(super) fn expected_effective_functions(self) -> &'static [&'static str] {
        match self {
            Self::Production(ProductionLoginProfile::CourseRetentionExecutor) => &[
                "ple_api.archive_course_student_records(uuid,timestamp with time zone)",
                "ple_api.delete_course_student_records(uuid,timestamp with time zone)",
                "ple_api.read_archived_course_student_work_for_retention(uuid)",
                "ple_data.course_retention_due_actions(timestamp with time zone)",
            ],
            Self::Production(ProductionLoginProfile::CourseRetentionNotifier) => &[
                "ple_api.claim_course_retention_notification(timestamp with time zone,integer)",
                "ple_api.record_course_retention_notification_provider_acceptance(uuid,uuid,uuid,timestamp with time zone)",
                "ple_api.record_course_retention_notification_delivered(uuid,uuid,timestamp with time zone)",
                "ple_api.fail_course_retention_notification_before_acceptance(uuid,uuid,timestamp with time zone,text)",
            ],
            _ => &[],
        }
    }

    pub(super) fn set_function_inventory_role_sql(self) -> Option<&'static str> {
        match self {
            Self::Production(ProductionLoginProfile::CourseRetentionExecutor) => {
                Some("SET ROLE ple_course_retention_executor")
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpectedMembership, LoginContract};
    use crate::postgres::ProductionLoginProfile;

    #[test]
    fn api_login_has_the_closed_session_capabilities() {
        let api = LoginContract::Production(ProductionLoginProfile::Api);

        assert_eq!(
            api.expected_memberships(),
            [
                ExpectedMembership {
                    role_name: "ple_app",
                    set_option: true,
                },
                ExpectedMembership {
                    role_name: "ple_auth",
                    set_option: true,
                },
            ]
        );
    }

    #[test]
    fn assessment_attempt_expiry_worker_login_has_one_exact_capability() {
        let worker =
            LoginContract::Production(ProductionLoginProfile::AssessmentAttemptExpiryWorker);

        assert_eq!(
            worker.expected_memberships(),
            [ExpectedMembership {
                role_name: "ple_assessment_attempt_expiry_worker",
                set_option: true,
            }]
        );
    }

    #[test]
    fn public_asset_publisher_login_has_one_exact_capability() {
        let publisher = LoginContract::Production(ProductionLoginProfile::PublicAssetPublisher);

        assert_eq!(
            publisher.expected_memberships(),
            [ExpectedMembership {
                role_name: "ple_public_asset_publisher",
                set_option: true,
            }]
        );
    }
}
