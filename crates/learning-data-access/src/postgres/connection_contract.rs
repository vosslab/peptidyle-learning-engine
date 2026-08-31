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
}
