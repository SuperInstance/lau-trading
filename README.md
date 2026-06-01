# lau-trading

A player-to-player trading marketplace for the Lau game world. Create trade offers for materials, agents, knowledge, recipes, and pets — accept, reject, gift, and track everything with full history and statistics.

## What This Does

`lau-trading` is the economic backbone of the Lau multiplayer game. It provides a central `TradeMarket` where players create offers ("I'll give you 5 iron for your builder bot"), accept or reject them, send one-way gifts, and query the marketplace to find deals. Every completed trade is recorded, and you can derive statistics — who trades the most, what item types are popular.

All state is serializable via `serde`, so the marketplace can be persisted, transmitted over a network, or snapshotted for undo.

## Key Idea

Trading is modeled as **offers with lifecycle states** (Pending → Accepted / Rejected / Expired). The market is the single source of truth: offers live in `TradeMarket::offers` while pending, and move to `TradeMarket::history` once accepted. This two-list design makes queries fast — you only search active offers, not completed trades.

Gifts are zero-negotiation transfers that skip the offer queue entirely and land directly in history.

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
lau-trading = { git = "https://github.com/SuperInstance/lau-trading" }
```

Requires Rust **2024 edition**.

## Quick Start

```rust
use lau_trading::{TradeMarket, Tradeable, Gift, TradeStats};

let mut market = TradeMarket::new();

// Alice offers 5 iron for Bob's builder bot
let id = market.create_offer(
    "alice",
    "bob",
    vec![Tradeable::Material("iron".into(), 5)],
    vec![Tradeable::Agent("builder_bot".into(), 0.95)],
);

// Bob accepts
let completed = market.accept(&id).unwrap();
assert_eq!(completed.offer.status, lau_trading::TradeStatus::Accepted);

// Eve sends a gift — no negotiation
market.send_gift(Gift {
    from: "eve".into(),
    to: "frank".into(),
    item: Tradeable::Knowledge("alchemy".into(), 0.99),
    message: "For your research".into(),
});

// Query: what offers are waiting for Bob?
let bob_offers = market.find_offers_for("bob");

// Statistics from completed trades
let stats = TradeStats::from_history(&market.history);
println!("Total trades: {}", stats.total_trades);
println!("Most active: {:?}", stats.most_active);
```

## API Reference

### Types

| Type | Description |
|---|---|
| `Tradeable` | Enum of tradeable items: `Material(name, qty)`, `Agent(name, skill)`, `Knowledge(name, level)`, `Recipe(name)`, `Pet(name, level)` |
| `TradeStatus` | Offer lifecycle: `Pending`, `Accepted`, `Rejected`, `Expired` |
| `TradeOffer` | An offer with `id`, `from`, `to`, `give` (items offered), `want` (items requested), `status` |
| `CompletedTrade` | A finished trade: the original offer + `completed_tick` |
| `TradeStats` | Aggregated stats: `total_trades`, `by_type` (counts per item category), `most_active` (player) |
| `Gift` | One-way transfer: `from`, `to`, `item`, `message` |
| `TradeMarket` | The central marketplace |

### TradeMarket Methods

| Method | Returns | Description |
|---|---|---|
| `new()` | `TradeMarket` | Empty marketplace |
| `create_offer(from, to, give, want)` | `String` | Creates a Pending offer, returns its ID |
| `accept(offer_id)` | `Result<CompletedTrade, String>` | Moves offer to history |
| `reject(offer_id)` | `Result<(), String>` | Marks offer as Rejected (stays in offers list) |
| `find_offers_for(player)` | `Vec<&TradeOffer>` | All Pending offers addressed to `player` |
| `find_offers_wanting(tradeable)` | `Vec<&TradeOffer>` | All Pending offers requesting a specific item |
| `send_gift(gift)` | `()` | Records a one-way gift directly into history |

### TradeOffer Methods

| Method | Returns | Description |
|---|---|---|
| `is_fair()` | `bool` | Simple fairness check: both sides offer the same number of items |

### TradeStats Methods

| Method | Returns | Description |
|---|---|---|
| `from_history(&[CompletedTrade])` | `TradeStats` | Derives statistics from trade history |

## How It Works

### Offer Lifecycle

```
create_offer() → [Pending] ──accept()──→ [Accepted] → history[]
                  │
                  └──reject()──→ [Rejected] (stays in offers[])
```

- **`create_offer`** generates an ID (`offer-N`) and pushes to `offers`.
- **`accept`** removes from `offers`, sets status to `Accepted`, pushes a `CompletedTrade` to `history`.
- **`reject`** sets status to `Rejected` in-place. A rejected offer can't be re-accepted.
- **`send_gift`** creates a synthetic `CompletedTrade` with an empty `want` list and ID `gift-N`.

### ID Generation

IDs are sequential: `offer-N` for trades, `gift-N` for gifts, where N = `offers.len() + history.len() + 1`. This is simple and collision-free within a single session.

### Serialization

All types derive `Serialize` and `Deserialize`. The entire `TradeMarket` can be serialized to JSON (or any serde format) and restored:

```rust
let json = serde_json::to_string(&market).unwrap();
let restored: TradeMarket = serde_json::from_str(&json).unwrap();
```

### Fairness Check

`is_fair()` uses a simple count-based heuristic: both `give` and `want` must have the same number of items. This is intentionally basic — real fairness would need value estimation, but for a kids' game, equal item count is a good starting point.

## The Math

### Statistics Aggregation

`TradeStats::from_history` iterates all completed trades:

- **`total_trades`**: `history.len()`
- **`by_type`**: For each item in `give`, increment the count for its category key (`"material"`, `"agent"`, `"knowledge"`, `"recipe"`, `"pet"`). Only the giver's items are counted (not the receiver's `want`).
- **`most_active`**: Count how many times each player appears as either `from` or `to`. The player with the highest count wins. Ties are broken by insertion order (first seen).

### Conservation Note

The marketplace does **not** enforce that items actually change hands — it only tracks offers and their states. The calling code is responsible for transferring items in the game's inventory system. This keeps the trading module decoupled from specific inventory implementations.

## Tests

**24 tests** covering:

- Offer creation, acceptance, rejection
- Double-reject and accept-after-reject error cases
- `find_offers_for` and `find_offers_wanting` queries
- Filtering out non-pending offers
- Gift sending and message preservation
- Statistics from empty, single, and multi-trade histories
- `most_active` player detection
- Full serde roundtrips for all types

Run with `cargo test`.

## License

MIT
