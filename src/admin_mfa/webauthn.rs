//! FIDO2 / WebAuthn second-factor support (hardware security keys, e.g. YubiKey).
//!
//! This implements the WebAuthn *ceremony* state machine — challenge issuance,
//! credential registration, assertion verification, and the cloned-authenticator
//! check via the signature counter (per the WebAuthn spec §6.1.1). The
//! cryptographic signature check itself is delegated to a pluggable
//! [`AssertionVerifier`] so a production deployment can wire in a full COSE/CBOR
//! verifier without this module pulling in a heavy crypto stack.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A registered WebAuthn credential (one hardware key per record).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredCredential {
    /// Opaque credential id from the authenticator.
    pub credential_id: Vec<u8>,
    /// COSE-encoded public key bytes.
    pub public_key: Vec<u8>,
    /// Last seen signature counter (monotonic; detects cloned keys).
    pub sign_count: u32,
    /// Friendly label ("YubiKey 5C", "Work laptop", …).
    pub label: String,
}

/// A pending registration or authentication challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebAuthnChallenge {
    /// Unique challenge id.
    pub id: Uuid,
    /// Random challenge bytes the authenticator must sign.
    pub challenge: Vec<u8>,
    /// Unix expiry timestamp (seconds).
    pub expires_at: i64,
}

/// Data returned by the client after an authentication ceremony.
#[derive(Debug, Clone)]
pub struct AssertionResponse {
    /// Which credential was used.
    pub credential_id: Vec<u8>,
    /// Authenticator signature counter from this assertion.
    pub sign_count: u32,
    /// The raw signature over (authenticatorData || clientDataHash).
    pub signature: Vec<u8>,
    /// The challenge bytes echoed by the client.
    pub challenge: Vec<u8>,
}

/// Errors surfaced by the WebAuthn factor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebAuthnError {
    /// No challenge matched (unknown, already consumed, or wrong value).
    UnknownChallenge,
    /// The challenge has expired.
    ChallengeExpired,
    /// The referenced credential is not registered.
    UnknownCredential,
    /// The signature failed cryptographic verification.
    BadSignature,
    /// The signature counter did not increase — possible cloned authenticator.
    CounterRollback,
}

/// Delegated signature verification. Implement against a real COSE verifier in
/// production; the default test double accepts any non-empty signature.
pub trait AssertionVerifier: Send + Sync {
    /// Verifies `signature` for `credential` over `challenge`.
    fn verify(&self, credential: &RegisteredCredential, challenge: &[u8], signature: &[u8]) -> bool;
}

/// A permissive verifier used in tests and as a wiring placeholder. It checks
/// only that a signature is present — **never** use it in production.
#[derive(Debug, Default, Clone, Copy)]
pub struct AcceptingVerifier;

impl AssertionVerifier for AcceptingVerifier {
    fn verify(&self, _credential: &RegisteredCredential, _challenge: &[u8], signature: &[u8]) -> bool {
        !signature.is_empty()
    }
}

/// Per-user store of credentials and outstanding challenges.
#[derive(Default)]
pub struct WebAuthnRegistry {
    credentials: HashMap<Uuid, Vec<RegisteredCredential>>,
    challenges: HashMap<Uuid, WebAuthnChallenge>,
}

impl WebAuthnRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Issues a challenge valid for `ttl_secs` from `now`.
    pub fn issue_challenge(
        &mut self,
        challenge_bytes: Vec<u8>,
        now: i64,
        ttl_secs: i64,
    ) -> WebAuthnChallenge {
        let challenge = WebAuthnChallenge {
            id: Uuid::new_v4(),
            challenge: challenge_bytes,
            expires_at: now + ttl_secs,
        };
        self.challenges.insert(challenge.id, challenge.clone());
        challenge
    }

    /// Registers a new credential for a user after a successful registration
    /// ceremony.
    pub fn register(&mut self, user: Uuid, credential: RegisteredCredential) {
        self.credentials.entry(user).or_default().push(credential);
    }

    /// Returns true if the user has at least one hardware key registered.
    pub fn has_credential(&self, user: Uuid) -> bool {
        self.credentials.get(&user).is_some_and(|c| !c.is_empty())
    }

    /// Lists a user's registered credentials.
    pub fn credentials(&self, user: Uuid) -> &[RegisteredCredential] {
        self.credentials.get(&user).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Verifies an authentication assertion, consuming the matching challenge,
    /// checking the signature and enforcing counter monotonicity.
    pub fn verify_assertion(
        &mut self,
        user: Uuid,
        challenge_id: Uuid,
        response: &AssertionResponse,
        verifier: &dyn AssertionVerifier,
        now: i64,
    ) -> Result<(), WebAuthnError> {
        // 1. Consume the challenge (single use) and validate it.
        let challenge = self
            .challenges
            .remove(&challenge_id)
            .ok_or(WebAuthnError::UnknownChallenge)?;
        if challenge.expires_at < now {
            return Err(WebAuthnError::ChallengeExpired);
        }
        if challenge.challenge != response.challenge {
            return Err(WebAuthnError::UnknownChallenge);
        }

        // 2. Locate the credential.
        let creds = self
            .credentials
            .get_mut(&user)
            .ok_or(WebAuthnError::UnknownCredential)?;
        let cred = creds
            .iter_mut()
            .find(|c| c.credential_id == response.credential_id)
            .ok_or(WebAuthnError::UnknownCredential)?;

        // 3. Verify the signature.
        if !verifier.verify(cred, &challenge.challenge, &response.signature) {
            return Err(WebAuthnError::BadSignature);
        }

        // 4. Cloned-authenticator detection (counter must strictly increase,
        //    unless the authenticator does not support counters and reports 0).
        if response.sign_count != 0 && response.sign_count <= cred.sign_count {
            return Err(WebAuthnError::CounterRollback);
        }
        cred.sign_count = response.sign_count;
        Ok(())
    }

    /// Drops expired challenges (housekeeping).
    pub fn prune_challenges(&mut self, now: i64) {
        self.challenges.retain(|_, c| c.expires_at >= now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credential() -> RegisteredCredential {
        RegisteredCredential {
            credential_id: vec![1, 2, 3, 4],
            public_key: vec![9, 9, 9],
            sign_count: 5,
            label: "YubiKey 5C".to_string(),
        }
    }

    fn registry_with_user() -> (WebAuthnRegistry, Uuid) {
        let mut reg = WebAuthnRegistry::new();
        let user = Uuid::new_v4();
        reg.register(user, credential());
        (reg, user)
    }

    #[test]
    fn registration_tracks_credentials() {
        let (reg, user) = registry_with_user();
        assert!(reg.has_credential(user));
        assert_eq!(reg.credentials(user).len(), 1);
        assert!(!reg.has_credential(Uuid::new_v4()));
    }

    #[test]
    fn valid_assertion_succeeds_and_advances_counter() {
        let (mut reg, user) = registry_with_user();
        let ch = reg.issue_challenge(vec![7; 32], 1000, 300);
        let resp = AssertionResponse {
            credential_id: vec![1, 2, 3, 4],
            sign_count: 6,
            signature: vec![0xaa],
            challenge: vec![7; 32],
        };
        assert!(reg
            .verify_assertion(user, ch.id, &resp, &AcceptingVerifier, 1100)
            .is_ok());
        assert_eq!(reg.credentials(user)[0].sign_count, 6);
    }

    #[test]
    fn counter_rollback_is_rejected() {
        let (mut reg, user) = registry_with_user(); // counter 5
        let ch = reg.issue_challenge(vec![7; 32], 1000, 300);
        let resp = AssertionResponse {
            credential_id: vec![1, 2, 3, 4],
            sign_count: 5, // not greater than stored 5
            signature: vec![0xaa],
            challenge: vec![7; 32],
        };
        assert_eq!(
            reg.verify_assertion(user, ch.id, &resp, &AcceptingVerifier, 1100),
            Err(WebAuthnError::CounterRollback)
        );
    }

    #[test]
    fn expired_challenge_is_rejected() {
        let (mut reg, user) = registry_with_user();
        let ch = reg.issue_challenge(vec![7; 32], 1000, 100);
        let resp = AssertionResponse {
            credential_id: vec![1, 2, 3, 4],
            sign_count: 6,
            signature: vec![0xaa],
            challenge: vec![7; 32],
        };
        assert_eq!(
            reg.verify_assertion(user, ch.id, &resp, &AcceptingVerifier, 2000),
            Err(WebAuthnError::ChallengeExpired)
        );
    }

    #[test]
    fn challenge_is_single_use() {
        let (mut reg, user) = registry_with_user();
        let ch = reg.issue_challenge(vec![7; 32], 1000, 300);
        let resp = AssertionResponse {
            credential_id: vec![1, 2, 3, 4],
            sign_count: 6,
            signature: vec![0xaa],
            challenge: vec![7; 32],
        };
        assert!(reg.verify_assertion(user, ch.id, &resp, &AcceptingVerifier, 1100).is_ok());
        // Re-use of the same challenge id now fails.
        assert_eq!(
            reg.verify_assertion(user, ch.id, &resp, &AcceptingVerifier, 1100),
            Err(WebAuthnError::UnknownChallenge)
        );
    }

    #[test]
    fn bad_signature_is_rejected() {
        let (mut reg, user) = registry_with_user();
        let ch = reg.issue_challenge(vec![7; 32], 1000, 300);
        let resp = AssertionResponse {
            credential_id: vec![1, 2, 3, 4],
            sign_count: 6,
            signature: vec![], // empty → AcceptingVerifier rejects
            challenge: vec![7; 32],
        };
        assert_eq!(
            reg.verify_assertion(user, ch.id, &resp, &AcceptingVerifier, 1100),
            Err(WebAuthnError::BadSignature)
        );
    }
}
