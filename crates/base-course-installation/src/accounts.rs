//! Ordinary account records required by the Base Course recipe.

use learning_data_access::AuthenticationEmail;
use learning_data_access::postgres::{
    BaseCourseAccountPlatformRoles, BaseCourseAccountRecipe, BaseCourseInstallLock,
};

use crate::{BaseCourseInstallError, BaseCourseParticipants};

pub(crate) const PRIMARY_INSTRUCTOR_NAME: &str = "Dr. Elena Rivera";
pub(crate) const MARY_NAME: &str = "Mary Okafor";
pub(crate) const JACK_NAME: &str = "Jack Chen";
pub(crate) const APPROVAL_CANDIDATE_NAME: &str = "Avery Singh";
pub(crate) const SYSADMIN_NAME: &str = "Morgan Reyes";

const PRIMARY_INSTRUCTOR_EMAIL: &str = "elena.rivera@live-demo.ple.example";
const MARY_EMAIL: &str = "mary.okafor@live-demo.ple.example";
const JACK_EMAIL: &str = "jack.chen@live-demo.ple.example";
const APPROVAL_CANDIDATE_EMAIL: &str = "avery.singh@live-demo.ple.example";
const SYSADMIN_EMAIL: &str = "morgan.reyes@live-demo.ple.example";

pub(crate) async fn ensure_accounts(
    lock: &mut BaseCourseInstallLock,
    participants: BaseCourseParticipants,
) -> Result<(), BaseCourseInstallError> {
    let accounts = account_recipes(participants)?;
    lock.provision_accounts(&accounts).await.map_err(|source| {
        BaseCourseInstallError::persistence("provisioning the Base Course accounts", source)
    })
}

fn account_recipes(
    participants: BaseCourseParticipants,
) -> Result<[BaseCourseAccountRecipe; 5], BaseCourseInstallError> {
    Ok([
        account_recipe(
            participants.primary_instructor(),
            PRIMARY_INSTRUCTOR_EMAIL,
            PRIMARY_INSTRUCTOR_NAME,
            BaseCourseAccountPlatformRoles::None,
        )?,
        account_recipe(
            participants.mary(),
            MARY_EMAIL,
            MARY_NAME,
            BaseCourseAccountPlatformRoles::None,
        )?,
        account_recipe(
            participants.jack(),
            JACK_EMAIL,
            JACK_NAME,
            BaseCourseAccountPlatformRoles::None,
        )?,
        account_recipe(
            participants.approval_candidate(),
            APPROVAL_CANDIDATE_EMAIL,
            APPROVAL_CANDIDATE_NAME,
            BaseCourseAccountPlatformRoles::None,
        )?,
        account_recipe(
            participants.sysadmin(),
            SYSADMIN_EMAIL,
            SYSADMIN_NAME,
            BaseCourseAccountPlatformRoles::Sysadmin,
        )?,
    ])
}

fn account_recipe(
    user: question_model::UserId,
    email: &str,
    display_name: &str,
    platform_roles: BaseCourseAccountPlatformRoles,
) -> Result<BaseCourseAccountRecipe, BaseCourseInstallError> {
    let email = AuthenticationEmail::parse(email).map_err(|error| {
        BaseCourseInstallError::baseline(format!(
            "the versioned account email for {display_name} is invalid: {error}"
        ))
    })?;
    BaseCourseAccountRecipe::new(user, email, display_name, platform_roles).map_err(|error| {
        BaseCourseInstallError::baseline(format!(
            "the versioned account recipe for {display_name} is invalid: {error}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use question_model::{TenantId, UserId, UserRole};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn recipe_defines_five_named_accounts_and_only_one_sysadmin() {
        let participants = BaseCourseParticipants::try_new(
            TenantId::from_uuid(Uuid::from_u128(1)),
            UserId::from_uuid(Uuid::from_u128(2)),
            UserId::from_uuid(Uuid::from_u128(3)),
            UserId::from_uuid(Uuid::from_u128(4)),
            UserId::from_uuid(Uuid::from_u128(5)),
            UserId::from_uuid(Uuid::from_u128(6)),
        )
        .unwrap();
        let accounts = account_recipes(participants).unwrap();

        assert_eq!(
            accounts
                .iter()
                .map(BaseCourseAccountRecipe::display_name)
                .collect::<Vec<_>>(),
            [
                PRIMARY_INSTRUCTOR_NAME,
                MARY_NAME,
                JACK_NAME,
                APPROVAL_CANDIDATE_NAME,
                SYSADMIN_NAME,
            ]
        );
        assert_eq!(
            accounts
                .iter()
                .map(|account| account.email().delivery())
                .collect::<Vec<_>>(),
            [
                PRIMARY_INSTRUCTOR_EMAIL,
                MARY_EMAIL,
                JACK_EMAIL,
                APPROVAL_CANDIDATE_EMAIL,
                SYSADMIN_EMAIL,
            ]
        );
        assert_eq!(
            accounts
                .iter()
                .map(BaseCourseAccountRecipe::user)
                .collect::<Vec<_>>(),
            [
                participants.primary_instructor(),
                participants.mary(),
                participants.jack(),
                participants.approval_candidate(),
                participants.sysadmin(),
            ]
        );
        assert_eq!(accounts[4].platform_roles(), [UserRole::Sysadmin]);
        assert!(
            accounts[..4]
                .iter()
                .all(|account| account.platform_roles().is_empty())
        );
    }
}
