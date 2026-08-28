use chrono::{Duration, Utc};
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
use chrono::TimeZone;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RefreshedTokens {
    pub access_token: String,
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
        let now = Utc::now();
        let exp = now + Duration::days(expires_in_days);

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

    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
        user_id: &str,
        email: &str,
        role: &str,
        permissions: Vec<String>,
        expires_in_hours: i64,
        refresh_expires_in_days: i64,
    ) -> Result<RefreshedTokens, JwtError> {
        let claims = self.validate_refresh_token(refresh_token)?;

        if claims.sub != user_id {
            return Err(JwtError::InvalidClaims);
        }

        let expiry = Utc
            .timestamp_opt(claims.exp, 0)
            .single()
            .ok_or(JwtError::InvalidClaims)?;
        self.revocation_list
            .revoke_token(&claims.jti, user_id, expiry);

        let access_token =
            self.generate_token(user_id, email, role, permissions, expires_in_hours)?;
        let new_refresh_token = self.generate_refresh_token(user_id, refresh_expires_in_days)?;

        Ok(RefreshedTokens {
            access_token,
            refresh_token: new_refresh_token,
        })
    }

    /// Revoke a single JWT by its jti (Issue #428)
    pub fn revoke_token(&self, jti: &str, user_id: &str, exp: i64) {
        let expiry = chrono::Utc.timestamp_opt(exp, 0).unwrap();
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

        let refreshed = jwt_service
            .refresh_access_token(
                &refresh_token,
                "user123",
                "test@example.com",
                "admin",
                vec!["read".to_string()],
                1,
                7,
            )
            .unwrap();

        let claims = jwt_service.validate_token(&refreshed.access_token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");

        assert!(jwt_service
            .validate_refresh_token(&refreshed.refresh_token)
            .is_ok());
        assert!(jwt_service.validate_refresh_token(&refresh_token).is_err());
    }

    #[test]
    fn test_refresh_token_replay_is_rejected() {
        let jwt_service = JwtService::new(
            TEST_SECRET,
            TEST_ISSUER.to_string(),
            TEST_AUDIENCE.to_string(),
        );

        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();

        let _first = jwt_service
            .refresh_access_token(
                &refresh_token,
                "user123",
                "test@example.com",
                "admin",
                vec!["read".to_string()],
                1,
                7,
            )
            .unwrap();

        assert!(matches!(
            jwt_service.validate_refresh_token(&refresh_token),
            Err(JwtError::Revoked)
        ));
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
}
