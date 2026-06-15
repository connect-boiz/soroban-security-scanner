# Issue 3: [Authentication] Missing Account Lockout Notification — Users Not Informed When Account Is Locked

## Description

The authentication system in `src/auth/` implements account lockout after a configurable number of failed login attempts (handled in `account_lockout.rs`). However, when a user's account becomes locked, the system silently rejects further login attempts with a generic "invalid credentials" error message rather than explicitly informing the user that their account has been temporarily locked due to excessive failed attempts. This creates confusion: legitimate users who may have forgotten their password or mistyped it repeatedly receive no clear indication that the lockout policy has been triggered, making them think the system is broken. Furthermore, there is no mechanism to notify the user via email or in-app notification that their account has been locked, nor is there a "forgot password" flow that automatically unlocks the account after successful password reset. The frontend login form also lacks the ability to display a lockout-specific error message with estimated remaining lockout time.

## Acceptance Criteria

- [ ] Return a distinct `AccountLocked` error response from the login endpoint when account is locked, including `locked_until` timestamp
- [ ] Update the frontend `LoginForm` component to display a clear lockout message: "Your account has been temporarily locked due to too many failed attempts. Please try again after [time]."
- [ ] Add email notification via `NotificationService` when an account becomes locked, including the lockout duration and instructions for password reset
- [ ] Implement automatic unlock on successful password reset via `PasswordResetForm`
- [ ] Add a `remaining_attempts` field to the login error response so the frontend can warn users before their account gets locked
- [ ] Write integration tests in `tests/auth_integration_tests.rs` covering the lockout notification flow

## Additional Context

Key files: `src/auth/account_lockout.rs`, `src/auth/middleware.rs`, `src/auth/mod.rs`, `frontend/components/auth/LoginForm.tsx`, `src/notification_service/service.rs`, `tests/auth_integration_tests.rs`.
