//! Recipient-specific read presence for one named question collection.
//!
//! This value models only a share relation and its closed lifecycle. Protected
//! services later resolve actors, collection ownership, and approval before
//! persistently applying these idempotent transitions.

use question_model::UserId;

use super::NamedQuestionCollectionId;

/// One recipient-specific, read-only relation for a named question collection.
///
/// The relation is deliberately non-authorizing and non-serializable. A later
/// Store and protected service own approval checks, ownership resolution,
/// persistence, audit, and read authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedQuestionCollectionShare {
    collection_id: NamedQuestionCollectionId,
    owner: UserId,
    recipient: UserId,
    state: NamedQuestionCollectionShareState,
}

/// Closed lifecycle state for one named collection share relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamedQuestionCollectionShareState {
    /// The recipient has an active read relation.
    Active,
    /// The recipient's former read relation is inactive.
    Revoked,
}

/// Why a named collection share relation cannot be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedQuestionCollectionShareError {
    /// The collection owner cannot be the share recipient.
    SelfRecipient,
}

impl std::fmt::Display for NamedQuestionCollectionShareError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SelfRecipient => "named collection share recipient must differ from its owner",
        })
    }
}

impl std::error::Error for NamedQuestionCollectionShareError {}

/// Result of granting or reactivating a named collection share relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedQuestionCollectionShareGrantOutcome {
    /// The revoked relation became active.
    Activated,
    /// The relation was already active.
    Unchanged,
}

/// Result of revoking a named collection share relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedQuestionCollectionShareRevokeOutcome {
    /// The active relation became revoked.
    Revoked,
    /// The relation was already revoked.
    Unchanged,
}

impl NamedQuestionCollectionShare {
    /// Creates an active recipient-specific read relation.
    pub fn new(
        collection_id: NamedQuestionCollectionId,
        owner: UserId,
        recipient: UserId,
    ) -> Result<Self, NamedQuestionCollectionShareError> {
        if owner == recipient {
            return Err(NamedQuestionCollectionShareError::SelfRecipient);
        }
        Ok(Self {
            collection_id,
            owner,
            recipient,
            state: NamedQuestionCollectionShareState::Active,
        })
    }

    /// Returns the opaque collection identity named by this relation.
    pub fn collection_id(&self) -> NamedQuestionCollectionId {
        self.collection_id
    }

    /// Returns the immutable collection owner named by this relation.
    pub fn owner(&self) -> UserId {
        self.owner
    }

    /// Returns the immutable read-only recipient named by this relation.
    pub fn recipient(&self) -> UserId {
        self.recipient
    }

    /// Returns this relation's closed lifecycle state.
    pub fn state(&self) -> NamedQuestionCollectionShareState {
        self.state
    }

    /// Activates a revoked relation without changing its identity.
    pub fn grant(&mut self) -> NamedQuestionCollectionShareGrantOutcome {
        if self.state == NamedQuestionCollectionShareState::Active {
            return NamedQuestionCollectionShareGrantOutcome::Unchanged;
        }
        self.state = NamedQuestionCollectionShareState::Active;
        NamedQuestionCollectionShareGrantOutcome::Activated
    }

    /// Revokes an active relation without changing its identity.
    pub fn revoke(&mut self) -> NamedQuestionCollectionShareRevokeOutcome {
        if self.state == NamedQuestionCollectionShareState::Revoked {
            return NamedQuestionCollectionShareRevokeOutcome::Unchanged;
        }
        self.state = NamedQuestionCollectionShareState::Revoked;
        NamedQuestionCollectionShareRevokeOutcome::Revoked
    }
}

#[cfg(test)]
mod tests {
    use question_model::UserId;
    use uuid::Uuid;

    use super::*;
    use crate::NamedQuestionCollection;

    fn user(value: u128) -> UserId {
        UserId::from_uuid(Uuid::from_u128(value))
    }

    fn collection(owner: UserId) -> NamedQuestionCollection {
        NamedQuestionCollection::new(owner, "Exam review".to_string(), vec![])
            .expect("valid collection")
    }

    #[test]
    fn creation_retains_identity_and_starts_active() {
        let owner = user(1);
        let collection = collection(owner);
        let share = NamedQuestionCollectionShare::new(collection.id(), owner, user(2))
            .expect("distinct recipient");

        assert_eq!(share.collection_id(), collection.id());
        assert_eq!(share.state(), NamedQuestionCollectionShareState::Active);
    }

    #[test]
    fn creation_refuses_self_recipient() {
        let owner = user(1);

        assert_eq!(
            NamedQuestionCollectionShare::new(collection(owner).id(), owner, owner),
            Err(NamedQuestionCollectionShareError::SelfRecipient)
        );
    }

    #[test]
    fn revoke_changes_active_relation_then_is_unchanged() {
        let owner = user(1);
        let mut share = NamedQuestionCollectionShare::new(collection(owner).id(), owner, user(2))
            .expect("distinct recipient");

        assert_eq!(
            share.revoke(),
            NamedQuestionCollectionShareRevokeOutcome::Revoked
        );
        assert_eq!(
            share.revoke(),
            NamedQuestionCollectionShareRevokeOutcome::Unchanged
        );
    }

    #[test]
    fn grant_reactivates_revoked_relation_then_is_unchanged() {
        let owner = user(1);
        let mut share = NamedQuestionCollectionShare::new(collection(owner).id(), owner, user(2))
            .expect("distinct recipient");
        share.revoke();

        assert_eq!(
            share.grant(),
            NamedQuestionCollectionShareGrantOutcome::Activated
        );
        assert_eq!(
            share.grant(),
            NamedQuestionCollectionShareGrantOutcome::Unchanged
        );
    }

    #[test]
    fn identity_accessors_leave_recipient_read_only() {
        let owner = user(1);
        let recipient = user(2);
        let collection = collection(owner);
        let share = NamedQuestionCollectionShare::new(collection.id(), owner, recipient)
            .expect("distinct recipient");

        assert_eq!(
            (share.collection_id(), share.owner(), share.recipient()),
            (collection.id(), owner, recipient)
        );
    }
}
