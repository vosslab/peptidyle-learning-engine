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
            Self::Production(ProductionLoginProfile::Worker) | Self::BaseCourseApplication => {
                &[ExpectedMembership {
                    role_name: "ple_app",
                    set_option: true,
                }]
            }
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
