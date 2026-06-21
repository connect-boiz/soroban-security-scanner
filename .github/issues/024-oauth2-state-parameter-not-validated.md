# Issue 24: [Authentication] OAuth2 State Parameter Not Validated, Allowing CSRF on Social Login

## Description

The OAuth2 implementation in `src/auth/oauth.rs` handles social login via the `oauth2` crate. The `OAuth2Config` stores client credentials, authorization URL, token URL, and redirect URL. During the OAuth2 authorization code flow, the backend generates a `state` parameter and sends it in the initial redirect to the provider. However, the callback endpoint (`/auth/oauth/callback`) does not validate that the `state` parameter returned by the provider matches the one originally sent. According to the OAuth 2.0 spec (RFC 6749, Section 10.12), this validation is mandatory to prevent CSRF attacks on the redirect flow. Without `state` validation, an attacker can initiate an OAuth2 flow with their own provider session and trick the victim into completing it, linking the attacker's social account to the victim's local account and gaining unauthorized access.

## Acceptance Criteria

- [ ] Store the `state` parameter in the user's HTTP session when initiating the OAuth2 redirect
- [ ] Validate the returned `state` parameter against the stored value in the callback handler
- [ ] If `state` does not match, reject the authentication and return a 403 error with a clear message
- [ ] Implement state parameter expiry: state tokens should expire after 10 minutes
- [ ] Use cryptographically secure random generation for the `state` parameter (using `ring::rand`)
- [ ] Write integration tests that simulate a CSRF attack and verify the callback is rejected

## Additional Context

Key files: `src/auth/oauth.rs`, `src/auth/mod.rs`, `src/auth/jwt.rs`, `tests/auth_integration_tests.rs`.
