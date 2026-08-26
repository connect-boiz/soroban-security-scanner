use chrono::{DateTime, Duration, TimeZone, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,              // Subject (user ID)
    pub email: String,            // User email
    pub role: String,             // User role
    pub permissions: Vec<String>, // User permissions
    pub session_id: String,       // Session identifier
    pub iat: i64,                 // Issued at
    pub exp: i64,                 // Expiration time
    pub iss: String,              // Issuer
    pub aud: String,              // Audience
    pub jti: String,              // JWT ID
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
    #[error("Token expired")]
    Expired,
    #[error("Token revoked")]
    Revoked,
    #[error("Invalid claims")]
    InvalidClaims,
    #[error("Encoding error: {0}")]
    Encoding(String),
    #[error("Decoding error: {0}")]
    Decoding(String),
}

use crate::auth::token_revocation::TokenRevocationList;
use std::sync::Arc;

/// A freshly minted access + refresh token pair.
///
/// Returned by [`JwtService::refresh_access_token`], which rotates the refresh
/// token on every use. Callers must persist **both** tokens and discard the
/// refresh token they passed in: the old one has been consumed (revoked) and is
/// rejected if presented again, which is what defeats refresh-token replay
/// (Issue #485).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Newly issued short-lived access token.
    pub access_token: String,
    /// Newly issued refresh token that replaces the one just used.
    pub refresh_token: String,
}

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    algorithm: Algorithm,
    /// Token revocation list for force-invalidation (Issue #428)
    revocation_list: Arc<TokenRevocationList>,
}

impl JwtService {
    pub fn new(secret: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            issuer,
            audience,
            algorithm: Algorithm::HS256,
            revocation_list: Arc::new(TokenRevocationList::new()),
        }
    }

    pub fn with_rsa(private_key: &str, public_key: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key.as_ref())
                .expect("Invalid RSA private key"),
            decoding_key: DecodingKey::from_rsa_pem(public_key.as_ref())
                .expect("Invalid RSA public key"),
            issuer,
            audience,
            algorithm: Algorithm::RS256,
            revocation_list: Arc::new(TokenRevocationList::new()),
        }
    }

    pub fn generate_token(
        &self,
        user_id: &str,
        email: &str,
        role: &str,
        permissions: Vec<String>,
        expires_in_hours: i64,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let exp = now + Duration::hours(expires_in_hours);

        let claims = JwtClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            permissions,
            session_id: Uuid::new_v4().to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            jti: Uuid::new_v4().to_string(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::Encoding(e.to_string()))?;

        // Track the issued token so it can be force-invalidated later via
        // revoke_all_user_tokens (e.g. on password change / compromise).
        self.revocation_list
            .track_issued_token(&claims.jti, user_id, exp);

        Ok(token)
    }

    pub fn generate_refresh_token(
        &self,
        user_id: &str,
        expires_in_days: i64,
    ) -> Result<String, JwtError> {
        let exp = Utc::now() + Duration::days(expires_in_days);
        self.generate_refresh_token_until(user_id, exp)
    }

    /// Issue a refresh token that expires at an explicit instant.
    ///
    /// Token rotation ([`Self::refresh_access_token`]) uses this to preserve the
    /// *original* refresh token's expiry, so repeatedly refreshing can never
    /// extend a session's absolute lifetime — every rotated token in a chain
    /// dies exactly when the first one would have.
    fn generate_refresh_token_until(
        &self,
        user_id: &str,
        exp: DateTime<Utc>,
    ) -> Result<String, JwtError> {
        let now = Utc::now();

        let claims = JwtClaims {
            sub: user_id.to_string(),
            email: "".to_string(), // Refresh tokens don't need email
            role: "refresh".to_string(),
            permissions: vec![],
            session_id: Uuid::new_v4().to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.issuer.clone(),
            aud: format!("{}-refresh", self.audience),
            jti: Uuid::new_v4().to_string(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::Encoding(e.to_string()))?;

        // Refresh tokens are long-lived, so tracking them is what makes a
        // password-change revocation able to reach them too.
        self.revocation_list
            .track_issued_token(&claims.jti, user_id, exp);

        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        let token_data =
            decode::<JwtClaims>(token, &self.decoding_key, &validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                    _ => JwtError::InvalidToken(e),
                }
            })?;

        // Check if token has been revoked (Issue #428)
        // This allows force-invalidation of JWTs on password change or account compromise
        if self.revocation_list.is_revoked(&token_data.claims.jti) {
            return Err(JwtError::Revoked);
        }

        Ok(token_data.claims)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&format!("{}-refresh", self.audience)]);

        let token_data =
            decode::<JwtClaims>(token, &self.decoding_key, &validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                    _ => JwtError::InvalidToken(e),
                }
            })?;

        if token_data.claims.role != "refresh" {
            return Err(JwtError::InvalidClaims);
        }

        // A refresh token is single-use: once refresh_access_token has rotated
        // (consumed) it, or it has been force-invalidated on a password change,
        // its jti is on the revocation list and must be rejected even though the
        // signature and expiry are still valid. Without this check a leaked or
        // already-used refresh token could be replayed to mint fresh access
        // tokens indefinitely (Issue #485).
        if self.revocation_list.is_revoked(&token_data.claims.jti) {
            return Err(JwtError::Revoked);
        }

        Ok(token_data.claims)
    }

    pub fn extract_token_from_header(&self, auth_header: &str) -> Option<String> {
        auth_header.strip_prefix("Bearer ").map(str::to_string)
    }

    pub fn is_token_expired(&self, token: &str) -> bool {
        match self.validate_token(token) {
            Ok(_) => false,
            Err(JwtError::Expired) => true,
            Err(_) => false,
        }
    }

    pub fn get_token_claims(&self, token: &str) -> Result<JwtClaims, JwtError> {
        self.validate_token(token)
    }

    /// Exchange a refresh token for a fresh access + refresh token pair.
    ///
    /// The refresh token is **rotated**: the presented token is consumed (added
    /// to the revocation list) and a brand-new refresh token is issued alongside
    /// the new access token. Because the presented token is now revoked, using
    /// it again fails [`Self::validate_refresh_token`], so a leaked or replayed
    /// refresh token is only ever usable once (Issue #485). The replacement
    /// refresh token inherits the consumed token's expiry, so rotation never
    /// extends the session's absolute lifetime.
    ///
    /// Callers must persist **both** returned tokens and discard the refresh
    /// token they passed in.
    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
        user_id: &str,
        email: &str,
        role: &str,
        permissions: Vec<String>,
        expires_in_hours: i64,
    ) -> Result<TokenPair, JwtError> {
        // Validate the presented refresh token. This now also rejects a token
        // whose jti has already been revoked/consumed, blocking replay.
        let claims = self.validate_refresh_token(refresh_token)?;

        // Ensure the refresh token belongs to the same user.
        if claims.sub != user_id {
            return Err(JwtError::InvalidClaims);
        }

        // Consume the presented refresh token so it can never be used again.
        // Any later attempt to refresh with it (replay) now fails the revocation
        // check in validate_refresh_token above.
        self.revoke_token(&claims.jti, &claims.sub, claims.exp);

        // Preserve the consumed token's expiry so the rotated token cannot
        // extend the session's absolute lifetime.
        let refresh_expiry = Utc
            .timestamp_opt(claims.exp, 0)
            .single()
            .ok_or(JwtError::InvalidClaims)?;

        // Issue the replacement access + refresh token pair.
        let access_token =
            self.generate_token(user_id, email, role, permissions, expires_in_hours)?;
        let refresh_token = self.generate_refresh_token_until(user_id, refresh_expiry)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    /// Revoke a single JWT by its jti (Issue #428)
    pub fn revoke_token(&self, jti: &str, user_id: &str, exp: i64) {
        let expiry = Utc.timestamp_opt(exp, 0).unwrap();
        self.revocation_list.revoke_token(jti, user_id, expiry);
    }

    /// Revoke ALL tokens for a user (Issue #428)
    /// Called on password change or account compromise.
    pub fn revoke_all_user_tokens(&self, user_id: &str) -> Result<usize, JwtError> {
        self.revocation_list
            .revoke_all_user_tokens(user_id)
            .map_err(|e| JwtError::Decoding(e.to_string()))
    }

    /// Prune expired entries from the revocation list (Issue #428)
    pub fn prune_expired_revocations(&self) -> usize {
        self.revocation_list.prune_expired()
    }

    /// Get a reference to the token revocation list (Issue #428)
    pub fn revocation_list(&self) -> &TokenRevocationList {
        &self.revocation_list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-that-is-long-enough-for-hs256";
    const TEST_ISSUER: &str = "test-issuer";
    const TEST_AUDIENCE: &str = "test-audience";

    #[test]
    fn test_jwt_token_generation_and_validation() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let token = jwt_service
            .generate_token(
                "user123",
                "test@example.com",
                "admin",
                vec!["read".to_string(), "write".to_string()],
                1,
            )
            .unwrap();

        let claims = jwt_service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.permissions, vec!["read", "write"]);
        assert_eq!(claims.iss, TEST_ISSUER);
        assert_eq!(claims.aud, TEST_AUDIENCE);
    }

    #[test]
    fn test_refresh_token_generation_and_validation() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();
        let claims = jwt_service.validate_refresh_token(&refresh_token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.role, "refresh");
        assert_eq!(claims.aud, format!("{}-refresh", TEST_AUDIENCE));
    }

    #[test]
    fn test_token_expiration() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        // Generate token with a past expiration
        let token = jwt_service
            .generate_token(
                "user123",
                "test@example.com",
                "admin",
                vec![],
                -1, // negative hours = already expired
            )
            .unwrap();

        // Token should be expired
        assert!(jwt_service.is_token_expired(&token));
    }

    #[test]
    fn test_extract_token_from_header() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let valid_header = "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9";
        let invalid_header = "Basic dXNlcjpwYXNz";
        let no_header = "";

        assert_eq!(
            jwt_service.extract_token_from_header(valid_header),
            Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string())
        );
        assert_eq!(jwt_service.extract_token_from_header(invalid_header), None);
        assert_eq!(jwt_service.extract_token_from_header(no_header), None);
    }

    #[test]
    fn test_refresh_access_token() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();

        let pair = jwt_service
            .refresh_access_token(
                &refresh_token,
                "user123",
                "test@example.com",
                "admin",
                vec!["read".to_string()],
                1,
            )
            .unwrap();

        let claims = jwt_service.validate_token(&pair.access_token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_token_rejected_after_revoke_all_user_tokens() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        // Issue a token and confirm it validates.
        let token = jwt_service
            .generate_token("user123", "test@example.com", "admin", vec![], 1)
            .unwrap();
        assert!(jwt_service.validate_token(&token).is_ok());

        // Simulate a password change / account compromise.
        let revoked = jwt_service.revoke_all_user_tokens("user123").unwrap();
        assert_eq!(revoked, 1);

        // The previously valid token must now be rejected as revoked, even
        // though its signature and expiry are still fine.
        assert!(matches!(
            jwt_service.validate_token(&token),
            Err(JwtError::Revoked)
        ));

        // A different user's token is unaffected.
        let other = jwt_service
            .generate_token("user999", "other@example.com", "user", vec![], 1)
            .unwrap();
        assert!(jwt_service.validate_token(&other).is_ok());
    }

    #[test]
    fn test_refresh_token_rotation_preserves_expiry_and_consumes_old_token() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();
        let original_exp = jwt_service
            .validate_refresh_token(&refresh_token)
            .unwrap()
            .exp;

        // Rotate: exchange the refresh token for a new access + refresh pair.
        let pair = jwt_service
            .refresh_access_token(
                &refresh_token,
                "user123",
                "test@example.com",
                "user",
                vec!["read".to_string()],
                1,
            )
            .unwrap();

        // A brand-new refresh token is issued (rotation)...
        assert_ne!(pair.refresh_token, refresh_token);
        let rotated = jwt_service
            .validate_refresh_token(&pair.refresh_token)
            .unwrap();
        // ...that inherits the original token's absolute expiry (no session
        // lifetime extension), alongside a working access token.
        assert_eq!(rotated.exp, original_exp);
        assert!(jwt_service.validate_token(&pair.access_token).is_ok());
    }

    #[test]
    fn test_refresh_token_replay_is_rejected_after_rotation() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();

        // Use the refresh token once — this consumes it.
        jwt_service
            .refresh_access_token(
                &refresh_token,
                "user123",
                "test@example.com",
                "user",
                vec![],
                1,
            )
            .unwrap();

        // Replaying the same refresh token must now be rejected as revoked,
        // both at the validation layer...
        assert!(matches!(
            jwt_service.validate_refresh_token(&refresh_token),
            Err(JwtError::Revoked)
        ));
        // ...and through the refresh entry point, so no new tokens are minted.
        assert!(matches!(
            jwt_service.refresh_access_token(
                &refresh_token,
                "user123",
                "test@example.com",
                "user",
                vec![],
                1,
            ),
            Err(JwtError::Revoked)
        ));
    }

    #[test]
    fn test_validate_refresh_token_rejects_revoked_jti() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();
        let claims = jwt_service.validate_refresh_token(&refresh_token).unwrap();

        // Directly revoke the refresh token's jti (e.g. on password change).
        jwt_service.revoke_token(&claims.jti, &claims.sub, claims.exp);

        assert!(matches!(
            jwt_service.validate_refresh_token(&refresh_token),
            Err(JwtError::Revoked)
        ));
    }
}
