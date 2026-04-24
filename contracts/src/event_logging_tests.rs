#![cfg(test)]

use soroban_sdk::{Env, Address, BytesN};
use crate::SecurityScannerContract;

#[test]
fn test_vulnerability_reported_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Report vulnerability should emit VULNERABILITY_REPORTED event
    let report_id = client.report_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "access_control"),
        &String::from_str(&env, "high"),
        &String::from_str(&env, "Missing access control check"),
        &String::from_str(&env, "contract.rs:100"),
        &token_id
    ).unwrap();
    
    // Verify event was emitted
    let events = env.events().all();
    let vulnerability_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("VULNERABILITY_REPORTED"))
        .collect();
    
    assert!(!vulnerability_events.is_empty());
    
    // Verify event data contains expected information
    let event_data = vulnerability_events[0].clone();
    let reported_event = event_data.try_into_val::<crate::VulnerabilityReportedEvent>().unwrap();
    
    assert_eq!(reported_event.report_id, report_id);
    assert_eq!(reported_event.reporter, reporter);
    assert_eq!(reported_event.contract_id, contract_address);
    assert_eq!(reported_event.vulnerability_type, String::from_str(&env, "access_control"));
    assert_eq!(reported_event.severity, String::from_str(&env, "high"));
}

#[test]
fn test_vulnerability_verified_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // First report vulnerability
    let report_id = client.report_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "access_control"),
        &String::from_str(&env, "high"),
        &String::from_str(&env, "Missing access control"),
        &String::from_str(&env, "contract.rs:100"),
        &token_id
    ).unwrap();
    
    // Fund bounty pool
    let funder = Address::generate(&env);
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &10000);
    client.fund_bounty_pool(&funder, &token_id, &5000);
    
    // Verify vulnerability should emit VULNERABILITY_VERIFIED and FUND_TRANSFER events
    client.verify_vulnerability(&admin, report_id, &1000i128, &token_id).unwrap();
    
    // Verify events were emitted
    let events = env.events().all();
    
    let verified_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("VULNERABILITY_VERIFIED"))
        .collect();
    
    let transfer_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("FUND_TRANSFER"))
        .collect();
    
    assert!(!verified_events.is_empty());
    assert!(!transfer_events.is_empty());
    
    // Verify verified event data
    let verified_event_data = verified_events[0].clone();
    let verified_event = verified_event_data.try_into_val::<crate::VulnerabilityVerifiedEvent>().unwrap();
    
    assert_eq!(verified_event.report_id, report_id);
    assert_eq!(verified_event.verifier, admin);
    assert_eq!(verified_event.reporter, reporter);
    assert_eq!(verified_event.bounty_amount, 1000i128);
    assert_eq!(verified_event.token, token_id);
    
    // Verify transfer event data
    let transfer_event_data = transfer_events[0].clone();
    let transfer_event = transfer_event_data.try_into_val::<crate::FundTransferEvent>().unwrap();
    
    assert_eq!(transfer_event.purpose, String::from_str(&env, "bounty"));
    assert_eq!(transfer_event.amount, 1000i128);
    assert_eq!(transfer_event.to, reporter);
}

#[test]
fn test_batch_escrow_release_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let recipients = vec![
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    let amounts = vec![1000i128, 1500i128, 2000i128];
    let tokens = vec![token_id.clone(), token_id.clone(), token_id.clone()];
    
    // Fund bounty pool for transfers
    let funder = Address::generate(&env);
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &10000);
    client.fund_bounty_pool(&funder, &token_id, &10000);
    
    // Batch escrow release should emit BATCH_ESCROW_RELEASE and multiple FUND_TRANSFER events
    let batch_id = client.batch_escrow_release(
        &admin,
        recipients.clone(),
        amounts.clone(),
        tokens.clone(),
    ).unwrap();
    
    // Verify events were emitted
    let events = env.events().all();
    
    let batch_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("BATCH_ESCROW_RELEASE"))
        .collect();
    
    let transfer_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("FUND_TRANSFER"))
        .collect();
    
    assert!(!batch_events.is_empty());
    assert_eq!(transfer_events.len(), 3); // One for each recipient
    
    // Verify batch event data
    let batch_event_data = batch_events[0].clone();
    let batch_event = batch_event_data.try_into_val::<crate::BatchEscrowReleaseEvent>().unwrap();
    
    assert_eq!(batch_event.batch_id, batch_id);
    assert_eq!(batch_event.recipient_count, 3);
    assert_eq!(batch_event.total_amount, 4500i128);
    assert_eq!(batch_event.initiated_by, admin);
    assert_eq!(batch_event.token, token_id);
}

#[test]
fn test_emergency_reward_distributed_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Report emergency vulnerability
    let alert_id = client.report_emergency_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "critical"),
        &String::from_str(&env, "Critical vulnerability"),
        &String::from_str(&env, "contract.rs:200"),
        &token_id
    ).unwrap();
    
    // Fund bounty pool for emergency rewards
    let funder = Address::generate(&env);
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &10000);
    client.fund_bounty_pool(&funder, &token_id, &10000);
    
    // Emergency reward distribution should emit EMERGENCY_REWARD_DISTRIBUTED and FUND_TRANSFER events
    let batch_id = client.emergency_reward_distribution(
        &admin,
        vec![alert_id],
        &token_id
    ).unwrap();
    
    // Verify events were emitted
    let events = env.events().all();
    
    let emergency_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("EMERGENCY_REWARD_DISTRIBUTED"))
        .collect();
    
    let transfer_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("FUND_TRANSFER"))
        .collect();
    
    assert!(!emergency_events.is_empty());
    assert!(!transfer_events.is_empty());
    
    // Verify emergency event data
    let emergency_event_data = emergency_events[0].clone();
    let emergency_event = emergency_event_data.try_into_val::<crate::EmergencyRewardDistributedEvent>().unwrap();
    
    assert_eq!(emergency_event.batch_id, batch_id);
    assert_eq!(emergency_event.alert_count, 1);
    assert_eq!(emergency_event.distributed_by, admin);
    assert_eq!(emergency_event.token, token_id);
    assert!(emergency_event.total_reward > 0);
}

#[test]
fn test_reputation_updated_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let researcher = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Report vulnerability should emit REPUTATION_UPDATED event
    let report_id = client.report_vulnerability(
        &researcher,
        &contract_address,
        &String::from_str(&env, "access_control"),
        &String::from_str(&env, "low"),
        &String::from_str(&env, "Minor issue"),
        &String::from_str(&env, "contract.rs:50"),
        &token_id
    ).unwrap();
    
    // Verify events were emitted
    let events = env.events().all();
    
    let reputation_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("REPUTATION_UPDATED"))
        .collect();
    
    assert!(!reputation_events.is_empty());
    
    // Verify reputation event data
    let reputation_event_data = reputation_events[0].clone();
    let reputation_event = reputation_event_data.try_into_val::<crate::ReputationUpdatedEvent>().unwrap();
    
    assert_eq!(reputation_event.researcher, researcher);
    assert_eq!(reputation_event.successful_reports, 0);
    assert_eq!(reputation_event.total_earnings, 0i128);
}

#[test]
fn test_gas_config_updated_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    // Update gas config should emit GAS_CONFIG_UPDATED event
    client.update_gas_config(
        &admin,
        Some(150000),
        Some(750000),
        Some(300000),
        Some(100),
        Some(150)
    ).unwrap();
    
    // Verify event was emitted
    let events = env.events().all();
    
    let gas_config_events: Vec<_> = events
        .iter()
        .filter(|(topic, _data)| topic == &soroban_sdk::Symbol::short("GAS_CONFIG_UPDATED"))
        .collect();
    
    assert!(!gas_config_events.is_empty());
    
    // Verify gas config event data
    let gas_config_event_data = gas_config_events[0].clone();
    let gas_config_event = gas_config_event_data.try_into_val::<crate::GasConfigUpdatedEvent>().unwrap();
    
    assert_eq!(gas_config_event.updated_by, admin);
    assert_eq!(gas_config_event.single_transfer_limit, 150000);
    assert_eq!(gas_config_event.batch_transfer_limit, 750000);
    assert_eq!(gas_config_event.emergency_limit, 300000);
    assert_eq!(gas_config_event.max_batch_size, 100);
    assert_eq!(gas_config_event.gas_price_multiplier, 150);
}

#[test]
fn test_event_logging_comprehensive() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token_id, _token_admin) = setup_test(&env);
    
    let reporter = Address::generate(&env);
    let contract_address = BytesN::from_array(&env, &[1u8; 32]);
    
    // Perform multiple operations that should emit various events
    let report_id = client.report_vulnerability(
        &reporter,
        &contract_address,
        &String::from_str(&env, "access_control"),
        &String::from_str(&env, "critical"),
        &String::from_str(&env, "Critical vulnerability"),
        &String::from_str(&env, "contract.rs:100"),
        &token_id
    ).unwrap();
    
    // Fund and verify vulnerability
    let funder = Address::generate(&env);
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&funder, &10000);
    client.fund_bounty_pool(&funder, &token_id, &5000);
    
    client.verify_vulnerability(&admin, report_id, &1000i128, &token_id).unwrap();
    
    // Update gas config
    client.update_gas_config(
        &admin,
        Some(200000),
        Some(1000000),
        Some(400000),
        Some(75),
        Some(120)
    ).unwrap();
    
    // Verify all expected events were emitted
    let events = env.events().all();
    let event_topics: Vec<_> = events.iter().map(|(topic, _data)| topic.clone()).collect();
    
    let expected_topics = vec![
        soroban_sdk::Symbol::short("VULNERABILITY_REPORTED"),
        soroban_sdk::Symbol::short("VULNERABILITY_VERIFIED"),
        soroban_sdk::Symbol::short("FUND_TRANSFER"), // Should appear twice (bounty pool funding + bounty payment)
        soroban_sdk::Symbol::short("REPUTATION_UPDATED"),
        soroban_sdk::Symbol::short("GAS_CONFIG_UPDATED"),
    ];
    
    for expected_topic in expected_topics {
        assert!(event_topics.contains(&expected_topic), 
            "Expected event topic {:?} not found in emitted events", expected_topic);
    }
    
    // Verify we have the right number of events
    assert!(event_topics.len() >= 5, "Should have at least 5 events emitted");
}

fn setup_test(env: &Env) -> (SecurityScannerContract, Address, BytesN<32>, Address) {
    let contract_id = env.register_contract(None, SecurityScannerContract);
    let client = SecurityScannerContract::new(env, &contract_id);
    
    let admin = Address::generate(env);
    client.initialize(&admin);
    
    let token_id = env.register_contract(None, soroban_sdk::token::StellarAssetContract);
    let token_admin = Address::generate(env);
    
    soroban_sdk::token::StellarAssetClient::new(env, &token_id)
        .initialize(&token_admin, &7, &"Test Token", &"TEST", &1000000000);
    
    client.add_supported_token(&admin, &token_id, 7, &1000000i128);
    
    (client, admin, token_id, token_admin)
}
