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

