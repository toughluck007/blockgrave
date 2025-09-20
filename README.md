# Blockgrave (Working Title)

## Core Concept
A retro-styled incremental simulation game that blends **cryptomining, stock trading, and lore-driven mystery** into a DOS-like interface. The player rebuilds a mysterious digital artifact known only as **The Chain** by mining and trading its fragments. The aesthetic is rooted in **1980s/1990s ANSI TUI graphics**, with colorful glyphs and pane-based navigation.

---

## Lore Framework
- **The Chain**: The universal digital currency, represented as a long, ancient structure of data that is fractured and scattered. The player is working to re-link it piece by piece.
- **Links**: Each mined unit of The Chain. Links are composed of smaller fragments that must be reconstructed.
- **Linklets**: The atomic units of a Link. Each linklet has difficulty and must be solved to restore the whole Link.
- **Hashpower Equivalent (TBD)**:
  - Working theme: "Relinks"
  - Alternative terms (Relinks per second): "Rl/s" "kRl/s" "mRl/s" "gRl/s" "tRl/s" (tying into kilo-, mega-, giga-, tera-relinks per second, etc)
- **Ledger**: A record of every Link mined, showing its ID, ownership history, and sale prices. Serves as both lore device and gameplay mechanic.

---

## UI Layout
- **Mining Pane**: Shows active mining job. Link represented as a grid of linklets. Glyphs fill in as mining progresses, resembling a defrag visualization. Completed linklets marked with ✓ or ✧. Shows job difficulty, est. payout, and completion time.
- **Hashpower Pane**: Displays current compute resources (Relinks/Threads/Cycles). Expandable popup for stats and management. From here, players can purchase upgrades or enter the Hashpower Marketplace.
- **Bank Pane**: Shows balance of Chain (crypto) and Credits (cash equivalent). Opens the Exchange Marketplace.
- **Ledger Pane**: A permanent history of all Links owned and traded, each with a serial ID (e.g. LNK-5F2D). Keeps pricing history and provenance.
- **Ticker Pane**: Bottom-of-screen scrolling stock ticker, showing Chain’s fluctuating market price.

---

## Core Systems

### Mining System
- Select from available Link jobs, each with difficulty, est. time, and payout.
- Links are grids of linklets, each with hidden difficulty. As mining progresses, linklets flip to ✓.
- Completed Link → added to Ledger with unique ID.
- Some Links trigger events that alter the global Chain price (spikes or crashes).

### Hashpower System
- Players purchase computing units to increase Relinks (hashpower):
  - Processor → Server → Rack → Lab → Supercomputer → Datacenter.
- Incremental cost scaling, limited daily purchase availability.
- Hashpower can also be rented in or out via marketplace, with NPCs filling the world (names appear as renters/buyers).

### Banking & Exchange
- **Bank Pane**: Tracks Chain vs. Credits.
- **Exchange Marketplace**: Trade Chain for Credits. Prices fluctuate semi-randomly but are nudged by player/NPC actions (mined Links, large sales).

### Hashpower Marketplace
- **Sell (Rent Out)**: Toggle availability, set min/max rental terms. Earn passive income while idle.
- **Buy (Rent In)**: Browse NPC offers, each with different terms. Adds to total Relinks temporarily.
- Marketplace populated with NPC usernames to keep the system lively.

---

## Gameplay Loop
1. Mine Links (restore fractured data → earn Chain).
2. Upgrade hashpower (buy processors, servers, racks).
3. Trade Chain for Credits, speculate on market prices.
4. Reinvest into hashpower or stockpile Links for later.
5. Events shift economy and add risk/reward.

---

## MVP Roadmap

**MVP 1**
- Mining pane with basic defrag-like progress.
- Hashpower upgrades with scaling.
- Bank + basic Exchange with random walk pricing.

**MVP 2**
- Ledger tracking with unique IDs.
- Renting hashpower in/out with NPC interactions.
- Events tied to Link completions.

**MVP 3**
- Market depth with ticker and stronger lore flavor.
- Advanced upgrade tiers (labs, datacenters).
- Hidden glyphs/linklets with special effects (e.g., market surge).

---

## Tone & Feel
- DOS/TUI aesthetic with ANSI colors.
- A mix of **tech-sim seriousness** and **cryptic lore mystery**.
- Player feels like a lone operator uncovering fragments of a lost protocol while chasing profits.


## Implementation Stack & Crates (Rust)

If you want the fastest path to a crisp retro TUI: **Rust + ratatui** is perfect. Here’s a battle-tested stack:

**Core TUI**

- `ratatui` – UI framework/widgets.
- `crossterm` – cross‑platform terminal backend (input, events, colors).
- `tui-logger` *(optional)* – log panel widget if you want an in‑game console.

**Async & Timing**

- `tokio` – timers (tick the mining loop every N ms), tasks (background price/matchmaking), channels.

**Data & Persistence**

- `serde`, `serde_json` – save files & simple caches.
- `directories` – OS‑correct save locations.
- `rusqlite` *or* `sqlite` crate – structured ledger/exchange DB.
- `sled` – simple embedded key‑value store (great for append‑only logs).

**IDs, Crypto-ish Hashing, RNG**

- `nanoid` or `uuid` – Link/transaction IDs.
- `blake3` – fast hashing (for faux‑block headers / merkle-ish roots).
- `rand`, `rand_distr` – market walks, difficulty rolls.

**Time, Colors, CLI**

- `chrono` or `time` – timestamps in ledger.
- `owo-colors` or `nu-ansi-term` – expressive ANSI colors.
- `clap` – dev toggles/cheats from the command line.

**Nice-to-haves**

- `tracing`, `tracing-subscriber` – structured logs; wire into a TUI log pane.
- `anyhow` – ergonomic error handling.

**Minimal **``** sketch**

```toml
[dependencies]
ratatui = "0.26"
crossterm = "0.27"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rand = "0.8"
nanoid = "0.4"
blake3 = "1"
# Choose one persistence path:
rusqlite = { version = "0.31", features = ["bundled"] } # SQL-ledger
# sled = "0.34"                                      # KV-ledger
chrono = { version = "0.4", features = ["serde"] }
owo-colors = "4"
anyhow = "1"
```

---

## Ledger / “Chain” Design Options

Two clean ways to realize the Chain → Links → Linklets model. Both keep the vibe without real-world crypto complexity.

### Option A — **SQL Ledger (structured, easy querying)**

Use SQLite via `rusqlite`.

- **Tables**
  - `links(id TEXT PK, created_at, difficulty, linklet_count, minted_value, mined_by, previous_hash, header_hash)`
  - `linklets(id TEXT PK, link_id TEXT, idx INT, difficulty, solved_at, solved_by)`
  - `trades(id TEXT PK, link_id TEXT, ts, seller, buyer, price_credits, price_chain)`
  - `market_events(id TEXT PK, ts, kind, magnitude, note)`
- **Faux‑block headers**: `header_hash = blake3(previous_hash || serialized_linklets || timestamp)`
- **Pros**: Great for the **Ledger Pane** (owner history, price charting, provenance). Easy to filter/sort.
- **Cons**: Slightly more ceremony than KV, but still tiny.

### Option B — **Append‑Only KV “Chainlog” (fast, flavor-rich)**

Use `sled` to store an **append‑only log** of events, plus secondary indexes in memory.

- **Keys**: monotonic counter → `000000000123`.
- **Values**: JSON event blobs like `{ kind: "LinkMined", link_id, header_hash, payout, ts }`.
- Reconstruct state at boot (or periodically snapshot to JSON).
- **Pros**: Super simple to write & stream to a TUI log. Feels like a chain of events.
- **Cons**: You’ll hand‑roll some indexes for queries (e.g., per‑link history).

**Recommendation**: Start with **Option A (SQLite)** for quality-of-life querying in the TUI (Ledger filters, price history). You can still compute a `header_hash` so every Link feels “chain‑y.”

---

## Market & Difficulty Model (quick recipes)

- **Price**: random walk with nudges.
  - `price_t = price_{t-1} + Normal(0, σ) + α*(recent_mined_value) + β*(net_trades)`
- **Job Generation**: draw Link difficulty from a distribution (e.g., `LogNormal`), derive est. time = `difficulty / relinks_per_sec`.
- **Events**: Poisson process for rares (crash/spike/corruption), emit `market_events` rows.

---

## Game Loop Wiring (Tokio)

- `tokio::time::interval(Duration::from_millis(100))` → tick mining progress.
- Spawn tasks:
  - **Mining** (consumes Hashpower budget → fills linklets).
  - **Market** (updates price, posts events).
  - **Matchmaker** (hashpower rentals begin/end).
- UI task draws ratatui frames at \~16–30 FPS or on input.

**Q: Where do Link IDs come from?**\
A: From `nanoid`, but with a **loreful, decodable prefix** that encodes difficulty (and optionally size) so players can “read” an ID at a glance.

### ID Scheme (nanoid + difficulty prefix)

**Format:** `L{D}{S}-{BODY}-{C}`

- `L` – constant to mark a Link.
- `{D}` – **difficulty bucket** (single hex nibble `0–F`, 16 buckets from easiest → hardest).
- `{S}` – **size bucket** for linklets (single hex nibble `0–F`, e.g., 0=very small, F=very large). *(Optional but recommended.)*
- `{BODY}` – a compact `nanoid` (e.g., length 6–8) using uppercase alphanumerics `0-9A-Z` for a DOS vibe.
- `{C}` – **checksum** (single base16 char) derived from a short `blake3` digest of the preceding parts to catch typos.

**Examples:**

- `L3A-7QK9PZ-9` → Difficulty bucket **3**, Size **A**, nanoid `7QK9PZ`, checksum `9`.
- `LFD-X1R8M2-4` → Difficulty **F** (very hard), Size **D**, nanoid `X1R8M2`, checksum `4`.

### Generation Steps

1. **Bucket Difficulty → **``
   - Compute a normalized difficulty score `d ∈ [0,1]` (e.g., mean linklet difficulty scaled).
   - `bucket_d = clamp( floor(d * 16), 0, 15 )` → hex nibble `0–F`.
2. **Bucket Size → **``
   - Use total linklets `n`. Define thresholds or logarithmic buckets.
   - Example: `bucket_s = clamp( floor(log2(n)) , 0, 15 )` → hex nibble.
3. **Create **`` with `nanoid` 6–8 chars, custom alphabet `0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ` (stable, terminal‑friendly).
   - Example: `nanoid!(6, &ALPHABET)` where `ALPHABET` is the set above.
4. **Checksum **``
   - `h = blake3("L{D}{S}-{BODY}")`
   - `{C} = hex(h)[0]` (first hex char of digest).
   - Display final as `L{D}{S}-{BODY}-{C}`.

### Decoding in UI (for the Ledger/Mining panes)

- Read `{D}` to color the ID badge (e.g., green→yellow→red across 0–F).
- Read `{S}` to hint grid size before opening details.
- Verify checksum on input/paste to prevent invalid IDs.

### Why this works well

- **Flavorful & useful**: IDs aren’t just random; they telegraph challenge/scale.
- **Lightweight**: No real blockchain needed; still feels “chain‑y.”
- **Backward compatible**: You can later add more buckets or switch `BODY` length without breaking existing IDs (keep parser tolerant).

