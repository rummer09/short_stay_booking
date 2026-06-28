# short_stay_booking

## Project Title
short_stay_booking

## Project Description
Short-stay rental platforms today (Airbnb, Vrbo, Booking.com, etc.) are
opaque: hosts and guests have to trust a centralized intermediary with
listings, identity, payments, and dispute resolution. `short_stay_booking`
is a Soroban smart contract that captures the full lifecycle of a
short-stay booking on-chain in a transparent, auditable state machine.
Hosts publish a property listing with a nightly rate, guests reserve
date ranges, hosts confirm, and the contract records the check-in,
check-out, and post-stay review events as on-chain state transitions.
The contract deliberately does not move native XLM — it focuses on the
identity, authorization, and event history that the on-chain layer can
guarantee, while settlement layers can be plugged in on top.

## Project Vision
The long-term goal is to become the trust-minimized backbone for
short-stay rentals on Stellar: a base layer where any listing,
booking, and reputation record is globally verifiable, censorship-
resistant, and composable with other on-chain primitives (stablecoin
payments, decentralized identity, NFT-based property titles, and DAO
governed dispute resolution). By making the booking state machine
itself a public smart contract, we remove the "black box" from the
rental market and let third parties build competing front-ends,
analytics, and financial products on top of the same data.

## Key Features
- **Property listings with nightly rate** — hosts publish a
  `property_hash` (off-chain content reference) and a `nightly_rate`
  via `list_property`, receiving a unique `listing_id` back.
- **Explicit booking lifecycle** — every booking progresses through
  the states `pending -> confirmed -> checked_in -> checked_out ->
  reviewed`, enforced by `require_auth` checks on the correct party
  at every transition.
- **Host-gated confirmation** — `confirm` can only be invoked by the
  address that owns the listing, so a guest cannot self-approve a stay.
- **Window-bound check-in** — `check_in` validates against
  `env.ledger().timestamp()` and the booking's `[check_in, check_out)`
  range, preventing premature or stale check-ins.
- **Tamper-proof review trail** — `leave_review` enforces a 1–5 star
  rating, one-review-per-booking, and a `checked_out` precondition so
  ratings can only be issued for stays that actually happened.
- **Pure state-machine design** — no native asset transfer is required,
  keeping the contract small, easy to audit, and easy to compose with
  a separate payment contract.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** real_estate dApp — see `contracts/short_stay_booking/src/lib.rs` for the full short_stay_booking business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CAT5UUG2CUXZOQNW2T3WKDN3Z72LTAT3JERT3UTW6ORT4QMC7Q7UHWTR`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/424665e8dac873bd57fc0506516d59ee62018d8b448f271ac547110ea6a9fc72`

## Future Scope
- **On-chain settlement** — integrate a payment contract (native XLM
  or a Stellar stablecoin SAC) that escrows the `total_price` on
  `confirm` and releases it to the host on `check_out`.
- **Cancellation & dispute flow** — add `cancel_booking` and
  `open_dispute` paths with time-locked refunds, plus a DAO-driven
  arbitrator contract for resolution.
- **Reputation aggregation** — compute per-host and per-guest
  aggregate ratings off-chain and, where useful, store signed
  attestations on-chain for portable reputation.
- **Calendar & availability proofs** — allow hosts to publish
  availability commitments (e.g. Merkle roots of available date sets)
  so that double-booking can be detected on-chain.
- **Multi-asset nightly rates** — extend `Listing` with a currency
  code and integrate the Soroban token interface to support USDC,
  EURC, and other Stellar-issued assets side by side.
- **Frontend & indexer** — ship a reference web app and a Soroban
  RPC indexer that exposes listings, bookings, and reviews to users
  in a familiar Airbnb-like UI.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `short_stay_booking` (real_estate)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
