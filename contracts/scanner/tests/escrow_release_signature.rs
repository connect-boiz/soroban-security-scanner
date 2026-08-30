//! Regression tests for Issue #481: `release_escrow` must actually verify the
//! release signature instead of storing it unchecked.
//!
//! These run against the real contract through the generated client, and the
//! signatures are produced with `ed25519-dalek` so we exercise the same
//! ed25519 scheme the Soroban host verifies with.

use ed25519_dalek::{Signer, SigningKey};
use security_scanner::{ContractError, SecurityScannerContract, SecurityScannerContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Bytes, BytesN, Env, String};

struct TestSetup<'a> {
    env: Env,
    client: SecurityScannerContractClient<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

/// Deterministic keypair from a seed byte, returning the dalek signing key and
/// its public key as the `BytesN<32>` the contract expects as a release signer.
fn make_signer(env: &Env, seed: u8) -> (SigningKey, BytesN<32>) {
    let signing_key = SigningKey::from_bytes(&[seed; 32]);
    let public_key = BytesN::from_array(env, &signing_key.verifying_key().to_bytes());
    (signing_key, public_key)
}

/// Sign a canonical release message and hand back a `BytesN<64>` signature.
fn sign_message(env: &Env, signing_key: &SigningKey, message: &Bytes) -> BytesN<64> {
    let bytes = message.to_alloc_vec();
    let signature = signing_key.sign(bytes.as_slice());
    BytesN::from_array(env, &signature.to_bytes())
}

fn setup() -> TestSetup<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(issuer);
    let token_id = token_contract.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(SecurityScannerContract, ());
    let client = SecurityScannerContractClient::new(&env, &contract_id);
    client.initialize(&admin, &token_id);

    TestSetup {
        env,
        client,
        token_admin,
    }
}

fn fund_depositor(setup: &TestSetup, depositor: &Address, amount: i128) {
    setup.token_admin.mint(depositor, &amount);
}

#[test]
fn release_with_valid_signature_succeeds() {
    let setup = setup();
    let env = &setup.env;
    let client = &setup.client;
    let depositor = Address::generate(env);
    let beneficiary = Address::generate(env);
    fund_depositor(&setup, &depositor, 10_000);
    let (signing_key, public_key) = make_signer(env, 7);

    let escrow_id = client.create_escrow(
        &depositor,
        &beneficiary,
        &1_000i128,
        &String::from_str(env, "bounty"),
        &0u64,
        &Some(public_key),
    );

    let message = client.escrow_release_message(&escrow_id);
    let signature = sign_message(env, &signing_key, &message);

    client.release_escrow(&escrow_id, &depositor, &Some(signature.clone()));

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, String::from_str(env, "released"));
    // Only a signature that actually verified should ever be persisted.
    assert_eq!(escrow.release_signature, Some(signature));
}

#[test]
fn release_without_signature_is_rejected_when_signer_required() {
    let setup = setup();
    let env = &setup.env;
    let client = &setup.client;
    let depositor = Address::generate(env);
    let beneficiary = Address::generate(env);
    fund_depositor(&setup, &depositor, 10_000);
    let (_signing_key, public_key) = make_signer(env, 9);

    let escrow_id = client.create_escrow(
        &depositor,
        &beneficiary,
        &1_000i128,
        &String::from_str(env, "bounty"),
        &0u64,
        &Some(public_key),
    );

    // A registered signer but no signature: the release must be refused.
    let result = client.try_release_escrow(&escrow_id, &depositor, &None);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));

    // ...and the funds stay put.
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, String::from_str(env, "pending"));
}

#[test]
fn release_with_forged_signature_is_rejected() {
    let setup = setup();
    let env = &setup.env;
    let client = &setup.client;
    let depositor = Address::generate(env);
    let beneficiary = Address::generate(env);
    fund_depositor(&setup, &depositor, 10_000);
    let (_registered_key, registered_pk) = make_signer(env, 11);
    let (attacker_key, _attacker_pk) = make_signer(env, 12);

    let escrow_id = client.create_escrow(
        &depositor,
        &beneficiary,
        &1_000i128,
        &String::from_str(env, "bounty"),
        &0u64,
        &Some(registered_pk),
    );

    // Correct message, but signed by the wrong key. ed25519_verify rejects it,
    // aborting the release.
    let message = client.escrow_release_message(&escrow_id);
    let forged = sign_message(env, &attacker_key, &message);

    let result = client.try_release_escrow(&escrow_id, &depositor, &Some(forged));
    assert!(
        result.is_err(),
        "forged signature must not release the escrow"
    );

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, String::from_str(env, "pending"));
}

#[test]
fn release_without_signer_keeps_legacy_behavior() {
    let setup = setup();
    let env = &setup.env;
    let client = &setup.client;
    let depositor = Address::generate(env);
    let beneficiary = Address::generate(env);
    fund_depositor(&setup, &depositor, 10_000);

    // No release signer registered — this is the path the emergency-reward flow
    // relies on, authorized purely by depositor auth.
    let escrow_id = client.create_escrow(
        &depositor,
        &beneficiary,
        &1_000i128,
        &String::from_str(env, "bounty"),
        &0u64,
        &None,
    );

    client.release_escrow(&escrow_id, &depositor, &None);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, String::from_str(env, "released"));
    assert_eq!(escrow.release_signature, None);
}

#[test]
fn signature_is_ignored_when_no_signer_registered() {
    let setup = setup();
    let env = &setup.env;
    let client = &setup.client;
    let depositor = Address::generate(env);
    let beneficiary = Address::generate(env);
    fund_depositor(&setup, &depositor, 10_000);

    let escrow_id = client.create_escrow(
        &depositor,
        &beneficiary,
        &1_000i128,
        &String::from_str(env, "bounty"),
        &0u64,
        &None,
    );

    // A well-formed signature over unrelated bytes. With no signer registered it
    // is neither required nor trusted, so it must not be stored on the escrow.
    let (stray_key, _pk) = make_signer(env, 3);
    let stray = sign_message(env, &stray_key, &Bytes::from_array(env, &[0u8; 8]));

    client.release_escrow(&escrow_id, &depositor, &Some(stray));

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, String::from_str(env, "released"));
    assert_eq!(escrow.release_signature, None);
}
