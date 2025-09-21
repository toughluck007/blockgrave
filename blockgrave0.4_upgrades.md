# Blockgrave – Extended Upgrades, Visuals & Events

## Upgrade Tree & Pricing

Progression and pricing rebalanced for smoother scaling and meaningful late‑game goals.

**Formula:**
```
price(n) = base × multiplier^(owned_n)
```

### Upgrade Tiers
| Tier            | RL/s per unit | Base price (Credits) | Multiplier | Soft daily cap |
|-----------------|---------------|----------------------|------------|----------------|
| Processor       | 1.0           | 75                   | 1.18       | 12             |
| Server          | 4.0           | 420                  | 1.20       | 8              |
| Rack            | 18.0          | 2,100                | 1.22       | 6              |
| Lab             | 65.0          | 9,500                | 1.24       | 4              |
| Supercomputer   | 220           | 38,000               | 1.26       | 3              |
| Datacenter      | 800           | 150,000              | 1.28       | 2              |
| **Quantum Array** | 3,000        | 620,000              | 1.31       | 2              |
| **Orbital Node**  | 10,500       | 2,400,000            | 1.34       | 1              |
| **Darknet Farm**  | 34,000       | 8,600,000            | 1.38       | 1              |
| **Foundry Core**  | 120,000      | 28,000,000           | 1.42       | 1              |

**Optional mechanics:**
- Diminishing returns: beyond N units, each new purchase is −2–5% efficient.
- Upkeep: small drain per tier, encouraging rental markets.
- Rental pricing floor tied to Chain price, avoiding collapse.

---

## Visual Experiments: “3D” in TUI

### A) Isometric 2.5D Projection
Each linklet drawn as a small cube/diamond. Shade faces based on progress.

**Projection math (simplified):**
```
u = (x - y) * cos(30°)
v = (x + y) * sin(30°) - z
```
- `x, y`: grid coords.
- `z`: progress (0 → unfinished, 1 → complete).

**ASCII shading:**
```
░▒▓█   (from light to dark)
```
Example progression of a single linklet cube:
```
  ▒▒      ▓▓      ███
 ▒▒▒▒ → ▓▓▓▓ → ██████
  ▒▒      ▓▓      ███
```
As more linklets complete, cubes “rise” visually, giving the feel of reconstructing a block.

### B) Unicode Braille Canvas
Braille chars pack 8 pixels into one cell (2×4 grid). Lets you fake higher‑resolution visuals.

**Mining visualization:**
- Dots fill in linklet‑shaped cells gradually.
- Each tick reveals more dots until the cell is full.

Example (dots filling left to right):
```
⠁ ⠃ ⠇ ⠟ ⠿
```
(From one dot → partial fill → fully solid cell.)

This works well for:
- Smooth progress bars within linklets.
- Tiny schematic “rebuilding the chain” animations.

---

## Events & Market Influence

Events spice up the loop and create moments of surprise. Two classes:

### Minor Events
- Trigger: ~1 in 30–60 Links mined.
- Effects: ±1–5% market price shift, temporary RL/s buff/debuff, NPC rumor text.
- Example: *“Whispers of a ghost miner surge…”* → −2% Chain price.

### Major Events
- Trigger: ~1 in 200 Links mined (rarer).
- Effects: ±10–20% price, or introduce temporary mechanics.
- Types:
  - **Crash Spike**: sudden −15% Chain, recovers slowly.
  - **Surge**: +12% Chain, decays quickly.
  - **Protocol Fork**: certain Links double payout for 10 minutes.
  - **Hash Storm**: rental market costs spike ×1.5.

### Event Timing Influence
- Events weighted by *market conditions*:
  - In a rising market: crashes more likely.
  - In a flat market: neutral anomalies (cosmetic lore, small buffs).
  - In a falling market: occasional big surges for relief.
- Each mined Link can carry a hidden probability of being a “trigger link,” rolling at completion.

### Ledger Integration
- Every event logged with timestamp + short description.
- Market rumors or anomalies could be shown in the ticker, giving the bottom bar new life.

---

## Balancing Guardrails
- Always generate at least one contract that finishes <60s, and one >5min, at current RL/s.
- Payout baseline: `credits ≈ 0.35 × difficulty`.
- Exchange spread: buy at `P × 1.01`, sell at `P × 0.99`.
- Volatility: σ shrinks as total Chain supply grows, simulating a maturing economy.

---

