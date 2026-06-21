# ultrasteller-contract

Soroban smart contract that anchors satellite ground-station pass events
on-chain as immutable, timestamped proofs.

`anchor_pass(submitter, pass_hash, satellite)` writes a record the first
time a given pass hash is submitted (rejects duplicates) and emits a
`PassAnchored` event. `get_pass(pass_hash)` reads it back. `pass_hash` is
computed off-chain (e.g. sha256 over satellite name, rise/set/max-elevation
times, and ground station) — the contract never needs to know the shape of
a pass record, only that one happened and when it was attested.

## Build & test

```bash
cargo test
stellar contract build
```

## Deploy (testnet)

```bash
stellar keys generate ultrasteller-admin --network testnet --fund
stellar contract deploy \
  --wasm target/wasm32v1-none/release/pass_anchor.wasm \
  --source ultrasteller-admin \
  --network testnet \
  --alias pass-anchor
```

## Currently deployed

- Network: Stellar testnet
- Contract address: `CAT2NFV7UGKNTCMYBUG7EQKKEDO3NOOINUSLIZ35LBRFSBRO7TFPUEXK`
