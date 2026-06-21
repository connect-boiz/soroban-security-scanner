# Issue 19: [Multi-Sig Wizard] Proposal Creation Form Lacks Real-Time Validation for Stellar Addresses

## Description

The `MultiSigWizard.tsx` component in `frontend/components/MultiSigWizard.tsx` provides a multi-step form for creating multi-signature proposals, including fields for signer addresses, signature thresholds, and execution delays. The signer address fields accept freeform text input but do not validate that the entered address is a valid Stellar public key (G... or X... format, 56 characters, base32-encoded with a version byte and checksum). Users can submit a proposal with an invalid address, which only fails at the backend API call (after form submission) with a cryptic error. This wastes user time and API resources. The `utils/validation.ts` file already contains a `validateStellarAddress()` function, but it is not integrated into the wizard form. Additionally, the form does not prevent users from adding duplicate signer addresses, which would inflate the required signature threshold without adding meaningful security.

## Acceptance Criteria

- [ ] Integrate `validateStellarAddress()` from `utils/validation.ts` into all signer address input fields in the wizard
- [ ] Show real-time validation feedback (green checkmark for valid, red error message for invalid) as the user types
- [ ] Prevent duplicate signer addresses — show a warning "Address {{address}} is already a signer" and disable the "Add" button
- [ ] Add input masking/formatting to make Stellar addresses more readable (group characters every 4 characters)
- [ ] Validate that the signature threshold is between 1 and the total number of unique signers
- [ ] Write unit tests for the validation functions covering edge cases like empty input, invalid characters, and checksum mismatch

## Additional Context

Key files: `frontend/components/MultiSigWizard.tsx`, `frontend/utils/validation.ts`.
