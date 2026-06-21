# ultrasteller-contract

**Ultrasteller** — live satellite fleet tracking and ground-station pass
prediction, built on real orbital data, with on-chain prediction markets on
Stellar/Soroban in progress.

🌐 Live: https://ultrasteller.vercel.app
Frontend: [ultrasteller-frontend](https://github.com/itodo001/ultrasteller-frontend)
Backend: [ultrasteller-backend](https://github.com/itodo001/ultrasteller-backend)

Soroban smart contracts for Ultrasteller. A Cargo workspace with two
contracts:

- **`pass-anchor`** — anchors satellite ground-station pass events on-chain
  as immutable, timestamped proofs.
- **`prediction-market`** — binary prediction markets self-resolved from real
  orbital data ("will this pass exceed N° max elevation?").

## pass-anchor

`anchor_pass(submitter, pass_hash, satellite)` writes a record the first
time a given pass hash is submitted (rejects duplicates) and emits a
`PassAnchored` event. `get_pass(pass_hash)` reads it back. `pass_hash` is
computed off-chain (e.g. sha256 over satellite name, rise/set/max-elevation
times, and ground station) — the contract never needs to know the shape of
a pass record, only that one happened and when it was attested.

## prediction-market

One market per (satellite, ground station) pass: a YES/NO bet on whether the
pass's max elevation will reach a threshold. Pure pari-mutuel, zero house
fee — the losing side's pool splits to the winning side pro-rata. Bets are
staked and paid out in the network's native asset (XLM) via the standard
token interface.

Market creation and resolution are **admin-only** — markets are derived from
real pass predictions and resolved from the recomputed actual outcome (the
backend's orbital-mechanics engine is the oracle), not user-created or
arbitrarily resolved.

- `initialize(admin, native_token)` — one-time setup.
- `create_market(market_id, satellite, station, threshold_centideg, close_time)`
- `place_bet(bettor, market_id, side, amount)` — `side = true` is YES.
- `resolve_market(market_id, actual_max_elevation_centideg)`
- `claim(bettor, market_id)` — pays stake back + pro-rata share of the
  losing pool if you won; nothing if you lost.
- `get_market(market_id)`, `get_position(market_id, bettor)` — read-only.

Known v1 limitation: if a market resolves with zero stake on the winning
side, the losing pool has no claimant and stays locked in the contract
(rare; recoverable later via an admin sweep, not built yet).

## Build & test

```bash
cargo test --workspace
stellar contract build          # builds all crates in the workspace
stellar contract build -p prediction-market   # or just one
```

## Deploy (testnet)

```bash
stellar keys generate ultrasteller-admin --network testnet --fund
stellar contract deploy \
  --wasm target/wasm32v1-none/release/prediction_market.wasm \
  --source ultrasteller-admin \
  --network testnet \
  --alias prediction-market

# native asset's Stellar Asset Contract address, needed for initialize:
stellar contract id asset --asset native --network testnet
```

## Currently deployed (Stellar testnet)

- `pass-anchor`: `CAT2NFV7UGKNTCMYBUG7EQKKEDO3NOOINUSLIZ35LBRFSBRO7TFPUEXK`
- `prediction-market`: `CDNRCSYFX22JB6PD7MCOAAL2QMVITBUHRTXQPIKYYVC6EHDO3O445VED`
  (initialized; full bet → resolve → claim lifecycle manually verified
  end-to-end with real testnet XLM before this was committed)
