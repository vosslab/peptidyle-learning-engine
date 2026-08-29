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
    /// Private arm used only by the sealed recovery-execution pool factories.
    AcceptedSubmissionRecovery,
    /// Private arm used only by the sealed exact-execution pool factories.
    AcceptedSubmissionFastPath,
    /// Host-only exact-execution identity used while installing the Base Course.
    BaseCourseAcceptedSubmissionFastPath,
    /// Private arm used exclusively by the dedicated application-pool factories.
    BaseCourseApplication,
    /// Private arm used exclusively by the opaque installer-pool factories.
    BaseCourseInstaller,
    Grader,
}

impl LoginContract {
    pub(super) fn expected_login(self) -> &'static str {
        match self {
            Self::Production(ProductionLoginProfile::Api) => "ple_api_login",
            Self::Production(ProductionLoginProfile::Worker) => "ple_worker_login",
            Self::AcceptedSubmissionRecovery => "ple_accepted_submission_recovery_login",
            Self::AcceptedSubmissionFastPath => "ple_accepted_submission_fast_path_login",
            Self::BaseCourseAcceptedSubmissionFastPath => "ple_base_course_fast_path_login",
            Self::Production(ProductionLoginProfile::InvitationDeliveryWorker) => {
                "ple_invitation_delivery_worker_login"
            }
            Self::Production(ProductionLoginProfile::Publisher) => "ple_publisher_login",
            Self::BaseCourseApplication => "ple_base_course_app_login",
            Self::BaseCourseInstaller => "ple_base_course_installer_login",
            Self::Grader => "ple_grading_reader",
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
            Self::Production(ProductionLoginProfile::Worker) => &[ExpectedMembership {
                role_name: "ple_app",
                set_option: true,
            }],
            Self::AcceptedSubmissionRecovery => &[ExpectedMembership {
                role_name: "ple_accepted_submission_execution",
                set_option: true,
            }],
            Self::AcceptedSubmissionFastPath | Self::BaseCourseAcceptedSubmissionFastPath => {
                &[ExpectedMembership {
                    role_name: "ple_accepted_submission_execution_fast_path",
                    set_option: true,
                }]
            }
            Self::BaseCourseApplication => &[ExpectedMembership {
                role_name: "ple_app",
                set_option: true,
            }],
            Self::Production(ProductionLoginProfile::InvitationDeliveryWorker) => {
                &[ExpectedMembership {
                    role_name: "ple_invitation_delivery_worker",
                    set_option: true,
                }]
            }
            Self::Production(ProductionLoginProfile::Publisher) => &[ExpectedMembership {
                role_name: "ple_public_asset_publisher",
                set_option: true,
            }],
            Self::BaseCourseInstaller => &[ExpectedMembership {
                role_name: "ple_base_course_installer",
                set_option: true,
            }],
            Self::Grader => &[ExpectedMembership {
                role_name: "ple_grader",
                set_option: true,
            }],
        }
    }

    pub(super) fn expected_capabilities(self) -> &'static [ExpectedMembership] {
        self.expected_memberships()
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpectedMembership, LoginContract};
    use crate::postgres::ProductionLoginProfile;

    #[test]
    fn process_logins_have_closed_distinct_capabilities() {
        let api = LoginContract::Production(ProductionLoginProfile::Api);
        let worker = LoginContract::Production(ProductionLoginProfile::Worker);
        let recovery = LoginContract::AcceptedSubmissionRecovery;
        let fast_path = LoginContract::AcceptedSubmissionFastPath;
        let base_course_fast_path = LoginContract::BaseCourseAcceptedSubmissionFastPath;

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
        assert_eq!(
            worker.expected_memberships(),
            [ExpectedMembership {
                role_name: "ple_app",
                set_option: true,
            }]
        );
        assert_eq!(
            recovery.expected_memberships(),
            [ExpectedMembership {
                role_name: "ple_accepted_submission_execution",
                set_option: true,
            }]
        );
        assert_eq!(
            fast_path.expected_memberships(),
            [ExpectedMembership {
                role_name: "ple_accepted_submission_execution_fast_path",
                set_option: true,
            }]
        );
        assert_eq!(
            base_course_fast_path.expected_login(),
            "ple_base_course_fast_path_login"
        );
        assert_eq!(
            base_course_fast_path.expected_memberships(),
            fast_path.expected_memberships()
        );
    }
}
