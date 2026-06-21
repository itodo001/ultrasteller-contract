#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::Env;

struct Setup {
    env: Env,
    client: PredictionMarketContractClient<'static>,
    token: token::TokenClient<'static>,
    token_address: Address,
}

fn setup() -> Setup {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = asset.address();
    let token = token::TokenClient::new(&env, &token_address);

    let contract_id = env.register(PredictionMarketContract, ());
    let client = PredictionMarketContractClient::new(&env, &contract_id);
    client.initialize(&admin, &token_address);

    Setup { env, client, token, token_address }
}

fn fund(setup: &Setup, who: &Address, amount: i128) {
    let asset_admin = token::StellarAssetClient::new(&setup.env, &setup.token_address);
    asset_admin.mint(who, &amount);
}

const ONE_HOUR: u64 = 3600;

fn create_test_market(setup: &Setup, market_id: &BytesN<32>, close_in_secs: u64) {
    let close_time = setup.env.ledger().timestamp() + close_in_secs;
    setup.client.create_market(
        market_id,
        &Symbol::new(&setup.env, "ISS_ZARYA"),
        &Symbol::new(&setup.env, "CAPE_CANAVERAL"),
        &4500,
        &close_time,
    );
}

fn advance_past_close(setup: &Setup, close_in_secs: u64) {
    setup.env.ledger().set_timestamp(setup.env.ledger().timestamp() + close_in_secs + 1);
}

#[test]
fn happy_path_both_sides_bet_and_claim() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[1u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);

    let alice = Address::generate(&s.env); // bets YES
    let bob = Address::generate(&s.env); // bets NO
    fund(&s, &alice, 100);
    fund(&s, &bob, 200);

    s.client.place_bet(&alice, &market_id, &true, &100);
    s.client.place_bet(&bob, &market_id, &false, &200);

    advance_past_close(&s, ONE_HOUR);
    s.client.resolve_market(&market_id, &5000); // 50.00deg >= 45.00deg threshold -> YES wins

    s.client.claim(&alice, &market_id);
    // Alice staked 100 of a 100 YES pool, wins the full 200 NO pool pro-rata -> 100 + 200 = 300
    assert_eq!(s.token.balance(&alice), 300);

    s.client.claim(&bob, &market_id);
    // Bob lost, no payout, balance stays at whatever remained after his bet (0)
    assert_eq!(s.token.balance(&bob), 0);
}

#[test]
fn proportional_payout_across_multiple_winners() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[2u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);

    let alice = Address::generate(&s.env); // YES, 100
    let carol = Address::generate(&s.env); // YES, 300
    let bob = Address::generate(&s.env); // NO, 400
    fund(&s, &alice, 100);
    fund(&s, &carol, 300);
    fund(&s, &bob, 400);

    s.client.place_bet(&alice, &market_id, &true, &100);
    s.client.place_bet(&carol, &market_id, &true, &300);
    s.client.place_bet(&bob, &market_id, &false, &400);

    advance_past_close(&s, ONE_HOUR);
    s.client.resolve_market(&market_id, &5000); // YES wins

    s.client.claim(&alice, &market_id);
    s.client.claim(&carol, &market_id);

    // YES pool = 400, NO pool (losing) = 400, split pro-rata by stake share.
    assert_eq!(s.token.balance(&alice), 100 + (100 * 400) / 400); // 200
    assert_eq!(s.token.balance(&carol), 300 + (300 * 400) / 400); // 600
}

#[test]
fn losing_side_with_zero_winning_pool_keeps_funds_locked() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[3u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);

    let bob = Address::generate(&s.env);
    fund(&s, &bob, 400);
    s.client.place_bet(&bob, &market_id, &false, &400);

    advance_past_close(&s, ONE_HOUR);
    s.client.resolve_market(&market_id, &5000); // YES wins, but nobody bet YES

    // Bob lost (he bet NO), no payout, and since nobody won there's nothing to claim.
    s.client.claim(&bob, &market_id);
    assert_eq!(s.token.balance(&bob), 0);
}

#[test]
#[should_panic(expected = "market already resolved")]
fn rejects_double_resolve() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[4u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);
    advance_past_close(&s, ONE_HOUR);
    s.client.resolve_market(&market_id, &5000);
    s.client.resolve_market(&market_id, &5000);
}

#[test]
#[should_panic(expected = "market closed")]
fn rejects_bet_after_close() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[5u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);

    let alice = Address::generate(&s.env);
    fund(&s, &alice, 100);
    advance_past_close(&s, ONE_HOUR);
    s.client.place_bet(&alice, &market_id, &true, &100);
}

#[test]
#[should_panic(expected = "market not resolved yet")]
fn rejects_claim_before_resolve() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[6u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);

    let alice = Address::generate(&s.env);
    fund(&s, &alice, 100);
    s.client.place_bet(&alice, &market_id, &true, &100);
    s.client.claim(&alice, &market_id);
}

#[test]
#[should_panic(expected = "already claimed")]
fn rejects_double_claim() {
    let s = setup();
    let market_id = BytesN::from_array(&s.env, &[7u8; 32]);
    create_test_market(&s, &market_id, ONE_HOUR);

    let alice = Address::generate(&s.env);
    fund(&s, &alice, 100);
    s.client.place_bet(&alice, &market_id, &true, &100);

    advance_past_close(&s, ONE_HOUR);
    s.client.resolve_market(&market_id, &5000);
    s.client.claim(&alice, &market_id);
    s.client.claim(&alice, &market_id);
}
