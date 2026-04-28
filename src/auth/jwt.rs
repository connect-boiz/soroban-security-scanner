use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,        // Subject (user ID)
    pub email: String,      // User email
    pub role: String,       // User role
    pub permissions: Vec<String>, // User permissions
    pub session_id: String, // Session identifier
    pub iat: i64,          // Issued at
    pub exp: i64,          // Expiration time
    pub iss: String,       // Issuer
    pub aud: String,       // Audience
    pub jti: String,       // JWT ID
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
    #[error("Token expired")]
    Expired,
    #[error("Invalid claims")]
    InvalidClaims,
    #[error("Encoding error: {0}")]
    Encoding(String),
    #[error("Decoding error: {0}")]
    Decoding(String),
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    algorithm: Algorithm,
}

impl JwtService {
    pub fn new(secret: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            issuer,
            audience,
            algorithm: Algorithm::HS256,
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

        Ok(token)
    }

    pub fn validate_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                _ => JwtError::InvalidToken(e),
            })?;

        Ok(token_data.claims)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<JwtClaims, JwtError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&format!("{}-refresh", self.audience)]);

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                _ => JwtError::InvalidToken(e),
            })?;

        if token_data.claims.role != "refresh" {
            return Err(JwtError::InvalidClaims);
        }

        Ok(token_data.claims)
    }

    pub fn extract_token_from_header(&self, auth_header: &str) -> Option<String> {
        if auth_header.starts_with("Bearer ") {
            Some(auth_header[7..].to_string())
        } else {
            None
        }
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
    ) -> Result<String, JwtError> {
        // Validate refresh token
        let claims = self.validate_refresh_token(refresh_token)?;
        
        // Ensure the refresh token belongs to the same user
        if claims.sub != user_id {
            return Err(JwtError::InvalidClaims);
        }

        // Generate new access token
        self.generate_token(user_id, email, role, permissions, expires_in_hours)
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
        let jwt_service = JwtService::new(TEST_SECRET, TEST_ISSUER.to_string(), TEST_AUDIENCE.to_string());
        
        let token = jwt_service.generate_token(
            "user123",
            "test@example.com",
            "admin",
            vec!["read".to_string(), "write".to_string()],
            1,
        ).unwrap();

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
        let jwt_service = JwtService::new(TEST_SECRET, TEST_ISSUER.to_string(), TEST_AUDIENCE.to_string());
        
        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();
        let claims = jwt_service.validate_refresh_token(&refresh_token).unwrap();
        
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.role, "refresh");
        assert_eq!(claims.aud, format!("{}-refresh", TEST_AUDIENCE));
    }

    #[test]
    fn test_token_expiration() {
        let jwt_service = JwtService::new(TEST_SECRET, TEST_ISSUER.to_string(), TEST_AUDIENCE.to_string());
        
        // Generate token with very short expiration
        let token = jwt_service.generate_token(
            "user123",
            "test@example.com",
            "admin",
            vec![],
            0, // 0 hours = immediate expiration
        ).unwrap();

        // Token should be expired
        assert!(jwt_service.is_token_expired(&token));
    }

    #[test]
    fn test_extract_token_from_header() {
        let jwt_service = JwtService::new(TEST_SECRET, TEST_ISSUER.to_string(), TEST_AUDIENCE.to_string());
        
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
        let jwt_service = JwtService::new(TEST_SECRET, TEST_ISSUER.to_string(), TEST_AUDIENCE.to_string());
        
        let refresh_token = jwt_service.generate_refresh_token("user123", 7).unwrap();
        
        let new_access_token = jwt_service.refresh_access_token(
            &refresh_token,
            "user123",
            "test@example.com",
            "admin",
            vec!["read".to_string()],
            1,
        ).unwrap();

        let claims = jwt_service.validate_token(&new_access_token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
    }
}
