#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, BytesN, Env, MuxedAddress, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    Market(BytesN<32>),
    Position(BytesN<32>, Address),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Market {
    pub satellite: Symbol,
    pub station: Symbol,
    pub threshold_centideg: u32,
    pub close_time: u64,
    pub yes_pool: i128,
    pub no_pool: i128,
    pub resolved: bool,
    pub outcome: Option<bool>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Position {
    pub yes_amount: i128,
    pub no_amount: i128,
    pub claimed: bool,
}

#[contract]
pub struct PredictionMarketContract;

#[contractimpl]
impl PredictionMarketContract {
    /// One-time setup. `native_token` is the network's native-asset Stellar
    /// Asset Contract address, used to escrow and pay out bets.
    pub fn initialize(env: Env, admin: Address, native_token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &native_token);
    }

    /// Creates a binary market: "will the satellite's max elevation over the
    /// station reach threshold_centideg by close_time?". Admin-only —
    /// markets are derived from real pass data by the backend, not
    /// user-created, so v1 has no permissionless market creation.
    pub fn create_market(
        env: Env,
        market_id: BytesN<32>,
        satellite: Symbol,
        station: Symbol,
        threshold_centideg: u32,
        close_time: u64,
    ) {
        Self::admin(&env).require_auth();

        if close_time <= env.ledger().timestamp() {
            panic!("close_time must be in the future");
        }

        let key = DataKey::Market(market_id);
        if env.storage().persistent().has(&key) {
            panic!("market already exists");
        }
        env.storage().persistent().set(
            &key,
            &Market {
                satellite,
                station,
                threshold_centideg,
                close_time,
                yes_pool: 0,
                no_pool: 0,
                resolved: false,
                outcome: None,
            },
        );
    }

    /// Places a bet. `side = true` is YES. Funds move immediately from
    /// `bettor` into the contract via the standard token interface.
    pub fn place_bet(env: Env, bettor: Address, market_id: BytesN<32>, side: bool, amount: i128) {
        bettor.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let key = DataKey::Market(market_id.clone());
        let mut market: Market = env.storage().persistent().get(&key).expect("market not found");
        if market.resolved {
            panic!("market already resolved");
        }
        if env.ledger().timestamp() >= market.close_time {
            panic!("market closed");
        }

        let token_client = token::TokenClient::new(&env, &Self::token(&env));
        let contract_addr: MuxedAddress = env.current_contract_address().into();
        token_client.transfer(&bettor, &contract_addr, &amount);

        if side {
            market.yes_pool += amount;
        } else {
            market.no_pool += amount;
        }
        env.storage().persistent().set(&key, &market);

        let pos_key = DataKey::Position(market_id, bettor);
        let mut position: Position = env.storage().persistent().get(&pos_key).unwrap_or_default();
        if side {
            position.yes_amount += amount;
        } else {
            position.no_amount += amount;
        }
        env.storage().persistent().set(&pos_key, &position);
    }

    /// Resolves a market from the actual recomputed max elevation.
    /// Admin-only (the backend's orbital-mechanics engine is the oracle).
    pub fn resolve_market(env: Env, market_id: BytesN<32>, actual_max_elevation_centideg: u32) {
        Self::admin(&env).require_auth();

        let key = DataKey::Market(market_id);
        let mut market: Market = env.storage().persistent().get(&key).expect("market not found");
        if market.resolved {
            panic!("market already resolved");
        }
        if env.ledger().timestamp() < market.close_time {
            panic!("market not closed yet");
        }
        market.resolved = true;
        market.outcome = Some(actual_max_elevation_centideg >= market.threshold_centideg);
        env.storage().persistent().set(&key, &market);
    }

    /// Pays out a winning position (stake back plus a pro-rata share of the
    /// losing pool). Losers get nothing; their stake funds the payout.
    pub fn claim(env: Env, bettor: Address, market_id: BytesN<32>) {
        bettor.require_auth();

        let market: Market = env
            .storage()
            .persistent()
            .get(&DataKey::Market(market_id.clone()))
            .expect("market not found");
        if !market.resolved {
            panic!("market not resolved yet");
        }
        let outcome = market.outcome.expect("resolved market missing outcome");

        let pos_key = DataKey::Position(market_id, bettor.clone());
        let mut position: Position = env.storage().persistent().get(&pos_key).expect("no position");
        if position.claimed {
            panic!("already claimed");
        }
        position.claimed = true;
        env.storage().persistent().set(&pos_key, &position);

        let (winning_amount, winning_pool, losing_pool) = if outcome {
            (position.yes_amount, market.yes_pool, market.no_pool)
        } else {
            (position.no_amount, market.no_pool, market.yes_pool)
        };

        if winning_amount == 0 || winning_pool == 0 {
            return;
        }

        let payout = winning_amount + (winning_amount * losing_pool) / winning_pool;

        let token_client = token::TokenClient::new(&env, &Self::token(&env));
        let to: MuxedAddress = bettor.into();
        token_client.transfer(&env.current_contract_address(), &to, &payout);
    }

    pub fn get_market(env: Env, market_id: BytesN<32>) -> Market {
        env.storage().persistent().get(&DataKey::Market(market_id)).expect("market not found")
    }

    pub fn get_position(env: Env, market_id: BytesN<32>, bettor: Address) -> Position {
        env.storage()
            .persistent()
            .get(&DataKey::Position(market_id, bettor))
            .unwrap_or_default()
    }

    fn admin(env: &Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).expect("not initialized")
    }

    fn token(env: &Env) -> Address {
        env.storage().instance().get(&DataKey::Token).expect("not initialized")
    }
}

mod test;
