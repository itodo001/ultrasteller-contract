#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

#[test]
fn anchors_a_new_pass_and_reads_it_back() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PassAnchorContract, ());
    let client = PassAnchorContractClient::new(&env, &contract_id);

    let submitter = Address::generate(&env);
    let satellite = Symbol::new(&env, "ISS_ZARYA");
    let pass_hash = BytesN::from_array(&env, &[7u8; 32]);

    client.anchor_pass(&submitter, &pass_hash, &satellite);

    let record = client.get_pass(&pass_hash).unwrap();
    assert_eq!(record.submitter, submitter);
    assert_eq!(record.satellite, satellite);
}

#[test]
#[should_panic(expected = "pass already anchored")]
fn rejects_anchoring_the_same_pass_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PassAnchorContract, ());
    let client = PassAnchorContractClient::new(&env, &contract_id);

    let submitter = Address::generate(&env);
    let satellite = Symbol::new(&env, "ISS_ZARYA");
    let pass_hash = BytesN::from_array(&env, &[7u8; 32]);

    client.anchor_pass(&submitter, &pass_hash, &satellite);
    client.anchor_pass(&submitter, &pass_hash, &satellite);
}

#[test]
fn unknown_pass_returns_none() {
    let env = Env::default();
    let contract_id = env.register(PassAnchorContract, ());
    let client = PassAnchorContractClient::new(&env, &contract_id);

    let pass_hash = BytesN::from_array(&env, &[1u8; 32]);
    assert_eq!(client.get_pass(&pass_hash), None);
}
