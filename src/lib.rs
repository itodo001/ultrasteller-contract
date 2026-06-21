#![no_std]
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, BytesN, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PassAnchorRecord {
    pub anchored_at: u64,
    pub submitter: Address,
    pub satellite: Symbol,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct PassAnchored {
    #[topic]
    pub satellite: Symbol,
    pub pass_hash: BytesN<32>,
}

#[contract]
pub struct PassAnchorContract;

#[contractimpl]
impl PassAnchorContract {
    /// Anchors a pass-event hash on-chain as an immutable, timestamped proof.
    /// `pass_hash` is computed off-chain (e.g. sha256 over satellite name,
    /// rise/set/max-elevation times, and ground station) so the contract
    /// never needs to know the shape of a pass record, only that one happened.
    pub fn anchor_pass(env: Env, submitter: Address, pass_hash: BytesN<32>, satellite: Symbol) {
        submitter.require_auth();

        if env.storage().persistent().has(&pass_hash) {
            panic!("pass already anchored");
        }

        let record = PassAnchorRecord {
            anchored_at: env.ledger().timestamp(),
            submitter: submitter.clone(),
            satellite: satellite.clone(),
        };
        env.storage().persistent().set(&pass_hash, &record);
        PassAnchored {
            satellite,
            pass_hash,
        }
        .publish(&env);
    }

    /// Looks up a previously anchored pass by its hash.
    pub fn get_pass(env: Env, pass_hash: BytesN<32>) -> Option<PassAnchorRecord> {
        env.storage().persistent().get(&pass_hash)
    }
}

mod test;
