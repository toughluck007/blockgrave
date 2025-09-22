use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nanoid::nanoid;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, LogNormal};
use serde::{Deserialize, Serialize};

const MAX_MESSAGES: usize = 5;
const JOB_POOL_SIZE: usize = 4;
const NANO_ALPHABET: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];
const SAVE_FILE: &str = "blockgrave-save.json";
const PRICE_UPDATE_MIN_SECS: f64 = 5.0;
const PRICE_UPDATE_MAX_SECS: f64 = 15.0;
const EXCHANGE_BUY_MULTIPLIER: f64 = 1.01;
const EXCHANGE_SELL_MULTIPLIER: f64 = 0.99;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneFocus {
    Mining,
    Hashpower,
    Bank,
    Ledger,
}

impl PaneFocus {
    fn next(self) -> Self {
        match self {
            PaneFocus::Mining => PaneFocus::Hashpower,
            PaneFocus::Hashpower => PaneFocus::Bank,
            PaneFocus::Bank => PaneFocus::Ledger,
            PaneFocus::Ledger => PaneFocus::Mining,
        }
    }

    fn prev(self) -> Self {
        match self {
            PaneFocus::Mining => PaneFocus::Ledger,
            PaneFocus::Hashpower => PaneFocus::Mining,
            PaneFocus::Bank => PaneFocus::Hashpower,
            PaneFocus::Ledger => PaneFocus::Bank,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PauseMenuItem {
    Resume,
    Save,
    Load,
    Quit,
}

impl PauseMenuItem {
    pub fn label(self) -> &'static str {
        match self {
            PauseMenuItem::Resume => "Resume",
            PauseMenuItem::Save => "Save",
            PauseMenuItem::Load => "Load",
            PauseMenuItem::Quit => "Exit",
        }
    }
}

#[derive(Debug, Default)]
pub struct PauseMenuState {
    selected: usize,
    status: Option<String>,
}

impl PauseMenuState {
    const OPTIONS: [PauseMenuItem; 4] = [
        PauseMenuItem::Resume,
        PauseMenuItem::Save,
        PauseMenuItem::Load,
        PauseMenuItem::Quit,
    ];

    fn select_next(&mut self) {
        self.selected = (self.selected + 1) % Self::OPTIONS.len();
    }

    fn select_previous(&mut self) {
        if self.selected == 0 {
            self.selected = Self::OPTIONS.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    fn current(&self) -> PauseMenuItem {
        Self::OPTIONS[self.selected]
    }

    pub fn items(&self) -> &'static [PauseMenuItem] {
        &Self::OPTIONS
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn status(&self) -> Option<&String> {
        self.status.as_ref()
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
    }
}

pub struct App {
    pub focus: PaneFocus,
    pub should_quit: bool,
    pub paused: bool,
    pub pause_menu: PauseMenuState,
    pub mining: MiningState,
    pub hashpower: HashpowerState,
    pub bank: BankState,
    pub ledger: LedgerState,
    pub ticker: TickerState,
    pub messages: VecDeque<String>,
    rng: StdRng,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut rng = StdRng::from_entropy();
        let mut mining = MiningState::new();
        for _ in 0..JOB_POOL_SIZE {
            mining.available_jobs.push(generate_job(&mut rng));
        }

        let mut hashpower = HashpowerState::default();
        hashpower.tiers[0].owned = 1; // Give the player a humble starting rig.

        Ok(Self {
            focus: PaneFocus::Mining,
            should_quit: false,
            paused: false,
            pause_menu: PauseMenuState::default(),
            mining,
            hashpower,
            bank: BankState::default(),
            ledger: LedgerState::default(),
            ticker: TickerState::new(32.0, &mut rng),
            messages: VecDeque::new(),
            rng,
        })
    }

    pub fn on_tick(&mut self, dt: Duration) {
        if self.paused {
            return;
        }
        let secs = dt.as_secs_f64();
        self.ticker.tick(dt, &mut self.rng);

        let power = self.hashpower.total_power();
        if let Some(completed) = self.mining.apply_work(power * secs) {
            let price = self.ticker.price;
            let credits_value = completed.job.payout_chain * price;
            let id = generate_link_id(&completed.job);
            let message = format!(
                "{} restored for {:.2} ⛓ ({:.2}₵)",
                id, completed.job.payout_chain, credits_value
            );
            self.push_message(message);
            self.bank.chain_balance += completed.job.payout_chain;
            let delta = self
                .ticker
                .apply_market_nudge(completed.job.market_impact, completed.job.payout_chain);
            let entry = LedgerEntry {
                id,
                name: completed.job.name,
                finished_at: completed.finished_at,
                difficulty: completed.job.difficulty,
                payout_chain: completed.job.payout_chain,
                credits_at_completion: credits_value,
                duration: completed.duration,
                market_impact: delta,
            };
            self.ledger.add_entry(entry);
            self.mining.available_jobs.push(generate_job(&mut self.rng));
        }

        self.mining.replenish_pool(&mut self.rng);
    }

    fn push_message(&mut self, msg: impl Into<String>) {
        self.messages.push_front(msg.into());
        while self.messages.len() > MAX_MESSAGES {
            self.messages.pop_back();
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if !matches!(key.kind, KeyEventKind::Press) {
            return;
        }

        if self.paused {
            self.handle_pause_input(key);
            return;
        }

        if matches!(key.code, KeyCode::Char('q' | 'Q')) {
            self.enter_pause();
            return;
        }

        match key.code {
            KeyCode::Tab => {
                self.focus = self.focus.next();
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
            }
            _ => match self.focus {
                PaneFocus::Mining => self.handle_mining_input(key),
                PaneFocus::Hashpower => self.handle_hashpower_input(key),
                PaneFocus::Bank => self.handle_bank_input(key),
                PaneFocus::Ledger => self.handle_ledger_input(key),
            },
        }
    }

    fn enter_pause(&mut self) {
        self.paused = true;
        self.pause_menu.set_status(None);
    }

    fn resume(&mut self) {
        self.paused = false;
        self.pause_menu.set_status(None);
    }

    fn handle_mining_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.mining.select_previous(),
            KeyCode::Down => self.mining.select_next(),
            KeyCode::Enter => {
                if let Some(job) = self.mining.take_selected_job() {
                    let name = job.name.clone();
                    self.mining.active_job = Some(ActiveJob::new(job));
                    self.push_message(format!("Accepted mining contract: {}", name));
                }
            }
            KeyCode::Char('r') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.mining.shuffle_jobs(&mut self.rng);
                    self.push_message("Contracts reshuffled".to_string());
                }
            }
            _ => {}
        }
    }

    fn handle_hashpower_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.hashpower.select_previous(),
            KeyCode::Down => self.hashpower.select_next(),
            KeyCode::Enter => {
                if let Some(cost) = self.hashpower.purchase_selected(&mut self.bank) {
                    self.push_message(format!(
                        "Purchased {} for {:.2}₵",
                        self.hashpower.selected_name(),
                        cost
                    ));
                }
            }
            _ => {}
        }
    }

    fn handle_bank_input(&mut self, key: KeyEvent) {
        let price = self.ticker.price;
        match key.code {
            KeyCode::Left => {
                if let Some(proceeds) = self.bank.sell_chain(1.0, price) {
                    self.push_message(format!(
                        "Sold 1.0 ⛓ for {:.2}₵ ({:.2}₵/⛓)",
                        proceeds, proceeds
                    ));
                }
            }
            KeyCode::Right => {
                if let Some(cost) = self.bank.buy_chain(1.0, price) {
                    self.push_message(format!("Bought 1.0 ⛓ for {:.2}₵ ({:.2}₵/⛓)", cost, cost));
                }
            }
            KeyCode::Char('m') => {
                let amount = 5.0;
                if let Some(proceeds) = self.bank.sell_chain(amount, price) {
                    self.push_message(format!(
                        "Market order: sold {:.1} ⛓ for {:.2}₵ ({:.2}₵/⛓)",
                        amount,
                        proceeds,
                        proceeds / amount
                    ));
                }
            }
            KeyCode::Char('b') => {
                let amount = 5.0;
                if let Some(cost) = self.bank.buy_chain(amount, price) {
                    self.push_message(format!(
                        "Bulk order: bought {:.1} ⛓ for {:.2}₵ ({:.2}₵/⛓)",
                        amount,
                        cost,
                        cost / amount
                    ));
                }
            }
            _ => {}
        }
    }

    fn handle_ledger_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.ledger.scroll_up(),
            KeyCode::Down => self.ledger.scroll_down(),
            _ => {}
        }
    }

    fn handle_pause_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.pause_menu.select_previous(),
            KeyCode::Down => self.pause_menu.select_next(),
            KeyCode::Enter => self.activate_pause_selection(),
            KeyCode::Esc => self.resume(),
            KeyCode::Char('q' | 'Q') => self.resume(),
            _ => {}
        }
    }

    fn activate_pause_selection(&mut self) {
        match self.pause_menu.current() {
            PauseMenuItem::Resume => self.resume(),
            PauseMenuItem::Save => match self.save_game() {
                Ok(()) => {
                    self.pause_menu.set_status(Some("Game saved.".to_string()));
                    self.push_message("Snapshot stored to disk.");
                }
                Err(err) => {
                    self.pause_menu
                        .set_status(Some(format!("Save failed: {}", err)));
                    self.push_message(format!("Save error: {}", err));
                }
            },
            PauseMenuItem::Load => match self.load_game() {
                Ok(()) => {
                    self.pause_menu.set_status(Some("Game loaded.".to_string()));
                    self.push_message("Restored state from snapshot.");
                }
                Err(err) => {
                    self.pause_menu
                        .set_status(Some(format!("Load failed: {}", err)));
                    self.push_message(format!("Load error: {}", err));
                }
            },
            PauseMenuItem::Quit => {
                self.should_quit = true;
            }
        }
    }

    fn save_game(&mut self) -> Result<()> {
        let snapshot = SaveData::from_app(self);
        let payload = serde_json::to_vec_pretty(&snapshot)?;
        fs::write(SAVE_FILE, payload).with_context(|| format!("writing {}", SAVE_FILE))?;
        Ok(())
    }

    fn load_game(&mut self) -> Result<()> {
        if !Path::new(SAVE_FILE).exists() {
            return Err(anyhow!("no save data available"));
        }
        let data = fs::read(SAVE_FILE).with_context(|| format!("reading {}", SAVE_FILE))?;
        let snapshot: SaveData =
            serde_json::from_slice(&data).context("parsing stored game state")?;
        snapshot.apply(self)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MiningState {
    pub available_jobs: Vec<MiningJob>,
    pub selected_job: usize,
    pub active_job: Option<ActiveJob>,
}

impl MiningState {
    fn new() -> Self {
        Self {
            available_jobs: Vec::new(),
            selected_job: 0,
            active_job: None,
        }
    }

    pub fn apply_work(&mut self, work: f64) -> Option<CompletedJob> {
        if let Some(active) = self.active_job.as_mut() {
            active.apply_work(work);
            if active.is_complete() {
                let active = self.active_job.take().unwrap();
                return Some(active.finish());
            }
        }
        None
    }

    pub fn take_selected_job(&mut self) -> Option<MiningJob> {
        if self.active_job.is_some() || self.available_jobs.is_empty() {
            return None;
        }
        if self.selected_job >= self.available_jobs.len() {
            self.selected_job = 0;
        }
        let job = self.available_jobs.remove(self.selected_job);
        if self.available_jobs.is_empty() {
            self.selected_job = 0;
        } else if self.selected_job >= self.available_jobs.len() {
            self.selected_job = self.available_jobs.len() - 1;
        }
        Some(job)
    }

    pub fn select_next(&mut self) {
        if self.available_jobs.is_empty() {
            return;
        }
        self.selected_job = (self.selected_job + 1) % self.available_jobs.len();
    }

    pub fn select_previous(&mut self) {
        if self.available_jobs.is_empty() {
            return;
        }
        if self.selected_job == 0 {
            self.selected_job = self.available_jobs.len() - 1;
        } else {
            self.selected_job -= 1;
        }
    }

    fn replenish_pool(&mut self, rng: &mut StdRng) {
        while self.available_jobs.len() < JOB_POOL_SIZE {
            self.available_jobs.push(generate_job(rng));
        }
    }

    fn shuffle_jobs(&mut self, rng: &mut StdRng) {
        self.available_jobs.shuffle(rng);
        self.selected_job = 0;
    }

    fn to_save(&self) -> MiningSave {
        MiningSave {
            available_jobs: self.available_jobs.clone(),
            selected_job: self.selected_job,
            active_job: self.active_job.as_ref().map(ActiveJobSave::from_active),
        }
    }

    fn apply_save(&mut self, save: MiningSave) {
        self.available_jobs = save.available_jobs;
        if self.available_jobs.is_empty() {
            self.selected_job = 0;
        } else {
            self.selected_job = save.selected_job.min(self.available_jobs.len() - 1);
        }
        self.active_job = save.active_job.map(ActiveJob::from_save);
    }
}

#[derive(Debug)]
pub struct ActiveJob {
    pub job: MiningJob,
    pub linklets: Vec<LinkletProgress>,
    pub current_index: usize,
    pub started_at: Instant,
}

impl ActiveJob {
    fn new(job: MiningJob) -> Self {
        let linklets = job
            .linklet_difficulties
            .iter()
            .copied()
            .map(LinkletProgress::new)
            .collect();
        Self {
            job,
            linklets,
            current_index: 0,
            started_at: Instant::now(),
        }
    }

    fn apply_work(&mut self, mut work: f64) {
        while work > 0.0 && self.current_index < self.linklets.len() {
            let linklet = &mut self.linklets[self.current_index];
            if linklet.remaining > work {
                linklet.remaining -= work;
                work = 0.0;
            } else {
                work -= linklet.remaining;
                linklet.remaining = 0.0;
                self.current_index += 1;
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.current_index >= self.linklets.len()
    }

    fn finish(self) -> CompletedJob {
        let finished_at = Utc::now();
        let duration = self.started_at.elapsed();
        CompletedJob {
            job: self.job,
            finished_at,
            duration,
        }
    }

    pub fn completion_ratio(&self) -> f64 {
        if self.linklets.is_empty() {
            return 0.0;
        }
        let total: f64 = self.linklets.iter().map(|l| l.difficulty).sum();
        let remaining: f64 = self.linklets[self.current_index..]
            .iter()
            .map(|l| l.remaining)
            .sum();
        ((total - remaining) / total).clamp(0.0, 1.0)
    }

    pub fn status_map(&self) -> Vec<LinkletStatus> {
        self.linklets
            .iter()
            .enumerate()
            .map(|(idx, linklet)| {
                if linklet.remaining <= 0.0 {
                    LinkletStatus::Complete
                } else if idx == self.current_index {
                    LinkletStatus::Active
                } else {
                    LinkletStatus::Pending
                }
            })
            .collect()
    }

    pub fn remaining_work(&self) -> f64 {
        self.linklets.iter().map(|l| l.remaining).sum()
    }

    fn from_save(save: ActiveJobSave) -> Self {
        let now = Instant::now();
        let elapsed = Duration::from_secs_f64(save.elapsed_secs.max(0.0));
        let started_at = now.checked_sub(elapsed).unwrap_or(now);
        let linklets = save.linklets;
        let len = linklets.len();
        Self {
            job: save.job,
            linklets,
            current_index: save.current_index.min(len),
            started_at,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LinkletStatus {
    Pending,
    Active,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkletProgress {
    pub difficulty: f64,
    pub remaining: f64,
}

impl LinkletProgress {
    fn new(difficulty: f64) -> Self {
        Self {
            difficulty,
            remaining: difficulty,
        }
    }
}

#[derive(Debug)]
pub struct CompletedJob {
    pub job: MiningJob,
    pub finished_at: DateTime<Utc>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningJob {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub difficulty: f64,
    pub payout_chain: f64,
    pub linklet_difficulties: Vec<f64>,
    pub market_impact: f64,
    pub lore: String,
}

#[derive(Debug, Clone)]
pub struct HashpowerTier {
    pub name: &'static str,
    pub base_cost: f64,
    pub cost_multiplier: f64,
    pub power: f64,
    pub owned: u32,
}

impl HashpowerTier {
    pub fn cost_for_next(&self) -> f64 {
        let scaling = self.cost_multiplier.powi(self.owned as i32);
        self.base_cost * scaling
    }

    pub fn total_power(&self) -> f64 {
        self.power * self.owned as f64
    }
}

#[derive(Debug)]
pub struct HashpowerState {
    pub tiers: Vec<HashpowerTier>,
    pub selected: usize,
}

impl Default for HashpowerState {
    fn default() -> Self {
        Self {
            tiers: vec![
                HashpowerTier {
                    name: "Processor",
                    base_cost: 75.0,
                    cost_multiplier: 1.18,
                    power: 1.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Server",
                    base_cost: 420.0,
                    cost_multiplier: 1.20,
                    power: 4.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Rack",
                    base_cost: 2100.0,
                    cost_multiplier: 1.22,
                    power: 18.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Lab",
                    base_cost: 9500.0,
                    cost_multiplier: 1.24,
                    power: 65.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Supercomputer",
                    base_cost: 38000.0,
                    cost_multiplier: 1.26,
                    power: 220.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Datacenter",
                    base_cost: 150000.0,
                    cost_multiplier: 1.28,
                    power: 800.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Quantum Array",
                    base_cost: 620000.0,
                    cost_multiplier: 1.31,
                    power: 3000.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Orbital Node",
                    base_cost: 2_400_000.0,
                    cost_multiplier: 1.34,
                    power: 10_500.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Darknet Farm",
                    base_cost: 8_600_000.0,
                    cost_multiplier: 1.38,
                    power: 34_000.0,
                    owned: 0,
                },
                HashpowerTier {
                    name: "Foundry Core",
                    base_cost: 28_000_000.0,
                    cost_multiplier: 1.42,
                    power: 120_000.0,
                    owned: 0,
                },
            ],
            selected: 0,
        }
    }
}

impl HashpowerState {
    pub fn total_power(&self) -> f64 {
        self.tiers.iter().map(|tier| tier.total_power()).sum()
    }

    fn owned_counts(&self) -> Vec<u32> {
        self.tiers.iter().map(|tier| tier.owned).collect()
    }

    fn apply_owned(&mut self, owned: &[u32], selected: usize) {
        for (tier, &amount) in self.tiers.iter_mut().zip(owned.iter()) {
            tier.owned = amount;
        }
        if owned.len() < self.tiers.len() {
            for tier in self.tiers[owned.len()..].iter_mut() {
                tier.owned = 0;
            }
        }
        if self.tiers.is_empty() {
            self.selected = 0;
        } else {
            self.selected = selected.min(self.tiers.len() - 1);
        }
    }

    fn select_next(&mut self) {
        self.selected = (self.selected + 1) % self.tiers.len();
    }

    fn select_previous(&mut self) {
        if self.selected == 0 {
            self.selected = self.tiers.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    fn purchase_selected(&mut self, bank: &mut BankState) -> Option<f64> {
        let tier = &mut self.tiers[self.selected];
        let cost = tier.cost_for_next();
        if bank.credits_balance >= cost {
            bank.credits_balance -= cost;
            tier.owned += 1;
            Some(cost)
        } else {
            None
        }
    }

    fn selected_name(&self) -> &str {
        self.tiers[self.selected].name
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankState {
    pub chain_balance: f64,
    pub credits_balance: f64,
}

impl Default for BankState {
    fn default() -> Self {
        Self {
            chain_balance: 0.0,
            credits_balance: 100.0,
        }
    }
}

impl BankState {
    pub fn sell_chain(&mut self, amount: f64, market_price: f64) -> Option<f64> {
        if self.chain_balance + 1e-6 < amount {
            return None;
        }
        let unit_price = market_price * EXCHANGE_SELL_MULTIPLIER;
        let proceeds = amount * unit_price;
        self.chain_balance -= amount;
        self.credits_balance += proceeds;
        Some(proceeds)
    }

    pub fn buy_chain(&mut self, amount: f64, market_price: f64) -> Option<f64> {
        let unit_price = market_price * EXCHANGE_BUY_MULTIPLIER;
        let cost = amount * unit_price;
        if self.credits_balance + 1e-6 < cost {
            return None;
        }
        self.credits_balance -= cost;
        self.chain_balance += amount;
        Some(cost)
    }
}

#[derive(Debug, Default)]
pub struct LedgerState {
    pub entries: Vec<LedgerEntry>,
    pub scroll: usize,
}

impl LedgerState {
    fn add_entry(&mut self, entry: LedgerEntry) {
        self.entries.insert(0, entry);
    }

    fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    fn scroll_down(&mut self) {
        if self.scroll + 1 < self.entries.len() {
            self.scroll += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub id: String,
    pub name: String,
    pub finished_at: DateTime<Utc>,
    pub difficulty: f64,
    pub payout_chain: f64,
    pub credits_at_completion: f64,
    pub duration: Duration,
    pub market_impact: f64,
}

impl LedgerEntry {
    pub fn finished_local(&self) -> DateTime<Local> {
        self.finished_at.with_timezone(&Local)
    }
}

#[derive(Debug)]
pub struct TickerState {
    pub price: f64,
    pub last_delta: f64,
    pub history: VecDeque<f64>,
    time_since_update: Duration,
    update_interval: Duration,
}

impl TickerState {
    fn new(initial_price: f64, rng: &mut StdRng) -> Self {
        let mut history = VecDeque::new();
        history.push_back(initial_price);
        Self {
            price: initial_price,
            last_delta: 0.0,
            history,
            time_since_update: Duration::ZERO,
            update_interval: Self::roll_interval(rng),
        }
    }

    fn tick(&mut self, dt: Duration, rng: &mut StdRng) {
        self.time_since_update += dt;
        while self.time_since_update >= self.update_interval {
            self.time_since_update -= self.update_interval;
            self.apply_random_walk(rng);
            self.update_interval = Self::roll_interval(rng);
        }
    }

    pub fn seconds_until_update(&self) -> f64 {
        if self.time_since_update >= self.update_interval {
            0.0
        } else {
            (self.update_interval - self.time_since_update).as_secs_f64()
        }
    }

    fn roll_interval(rng: &mut StdRng) -> Duration {
        Duration::from_secs_f64(rng.gen_range(PRICE_UPDATE_MIN_SECS..=PRICE_UPDATE_MAX_SECS))
    }

    fn apply_random_walk(&mut self, rng: &mut StdRng) {
        let drift = rng.gen_range(-0.25..0.35);
        let noise = rng.gen_range(-0.15..0.15);
        let delta = drift * 0.012 + noise * 0.006;
        let new_price = (self.price * (1.0 + delta)).max(0.25);
        self.last_delta = new_price - self.price;
        self.price = new_price;
        self.record_price();
    }

    fn apply_market_nudge(&mut self, impact: f64, payout_chain: f64) -> f64 {
        let impulse = (impact * payout_chain * 0.01).clamp(-5.0, 5.0);
        let new_price = (self.price + impulse).max(0.25);
        let delta = new_price - self.price;
        self.price = new_price;
        self.last_delta = delta;
        self.record_price();
        delta
    }

    fn record_price(&mut self) {
        self.history.push_back(self.price);
        while self.history.len() > 256 {
            self.history.pop_front();
        }
    }

    fn to_save(&self) -> TickerSave {
        TickerSave {
            price: self.price,
            last_delta: self.last_delta,
            history: self.history.iter().copied().collect(),
            time_since_update_secs: self.time_since_update.as_secs_f64(),
            update_interval_secs: self.update_interval.as_secs_f64(),
        }
    }

    fn from_save(save: TickerSave) -> Self {
        let mut history = VecDeque::from(save.history);
        if history.is_empty() {
            history.push_back(save.price);
        }
        let mut state = Self {
            price: save.price,
            last_delta: save.last_delta,
            history,
            time_since_update: Duration::from_secs_f64(save.time_since_update_secs.max(0.0)),
            update_interval: Duration::from_secs_f64(
                save.update_interval_secs
                    .clamp(PRICE_UPDATE_MIN_SECS, PRICE_UPDATE_MAX_SECS),
            ),
        };
        if state.time_since_update > state.update_interval {
            state.time_since_update = state.update_interval;
        }
        state
    }
}

fn generate_job(rng: &mut StdRng) -> MiningJob {
    const ADJECTIVES: &[&str] = &[
    "Fractured", "Dim", "Sharded", "Glitched", "Ghost", "Silent", "Echoing", "Cascading",
    "Prismatic", "Encrypted", "Obsidian", "Harmonic",
    "Errant", "Flickering", "Hollow", "Recursive", "Spectral", "Jagged", "Volatile", "Drifting",
    "Lucid", "Null", "Resonant", "Entropic",
    // new crypto-occult
    "Forked",
    "Haunted",
    "Burned",
    "Ancestral",
    "Ledgerbound",
    "Spectral",
    "Cursed",
    "Transient",
    "Byzantine",
    "Hallowed",
    "Orphaned",
    "Immutable",
];

    const NOUNS: &[&str] = &[
    "Segment", "Archive", "Spindle", "Glyph", "Node", "Fragment", "Shard", "Atlas", "Conduit",
    "Vault", "Kernel", "Beacon",
    "Ledger", "Relic", "Circuit", "Echo", "Phantom", "Codex", "Anchor", "Relay", "Cipher",
    "Obelisk", "Tomb", "Vector",
    // new crypto-occult
    "Domain",
    "Epoch",
    "Oracle",
    "Burn Address",
    "Hash",
    "Mnemonic",
    "Witness",
    "Oblivion",
    "Saint",
    "Consensus",
    "Daemon",
    "Halving",
];
    const LORE: &[&str] = &[
        "Ancient checksum mismatch logs recur in the metadata.",
        "Ledger note claims this link once belonged to the Archivist.",
        "The fragment hums at low frequencies when restored.",
        "OSINT chatter suggests this shard triggered a market spike decades ago.",
        "An abandoned relay stamped 'BLOCKGRAVE' is encoded in the payload.",
        "Hidden comment references a broken covenant between miners.",
        "Telemetry pings from an Orbital Node still reference this checksum.",
        "A Darknet broker swears this fragment bankrolls quantum rent strikes.",
        "Foundry Core schematics mark this link as a stabiliser conduit.",
        "The Null Archivist left an apology packet hidden in the footer.",
        "Quantum foreman chatter hints at an unfinished ritual encoded here.",
        "Obsolete syscalls still loop when this node is queried.",
        "An encrypted diary fragment maps trade routes through dead space.",
        "The checksum aligns with a forgotten guild’s initiation rites.",
        "Deep cache inspection reveals timestamps older than the chain itself.",
        "Data-miners whisper this archive once looped the whole market.",
        "A phantom transaction ID resurfaces every equinox cycle.",
        "Broken glyphs resemble prayers to a machine-saint.",
        "Sector cartographers note this vector bends in non-Euclidean paths.",
        "The echo of a relay collapse is still trapped in its logs.",
        "Packet sniffers detect laughter embedded in corrupted frames.",
        "A genesis block fragment bleeds through the checksum, signed by no known miner.",
        "Obelisk engravings match hash outputs from a lost forked chain.",
        "The Null Archivist’s covenant required burning coins to prove loyalty.",
        "Forked ledgers in this shard contradict every known consensus protocol.",
        "Ghost miners still broadcast shares to this phantom pool.",
        "A forgotten saint of code is named in corrupted transaction notes.",
        "Some claim this anchor wallet drains value every lunar cycle.",
        "Consensus rites were once chanted in machine-halls around this relic.",
        "A worm-eaten codex reveals rituals for binding hashes into prayer.",
        "This beacon’s logs oscillate between laughter and market orders.",
        "Miners whisper the shard was staked to summon liquidity apparitions.",
        "An orphan block here encodes a hymn to the Overseer of Chains.",
        "Hexed glyphs on the relay resemble burn addresses etched in ash.",
        "An abandoned vault pulses as if waiting for a final signature.",
        "Market prophets swore this vector predicted halvings centuries early.",
    ];

    let adjective = ADJECTIVES[rng.gen_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.gen_range(0..NOUNS.len())];
    let name = format!("{} {}", adjective, noun);
    let lore = LORE[rng.gen_range(0..LORE.len())].to_string();

    let rows = rng.gen_range(3..=6);
    let cols = rng.gen_range(4..=8);
    let count = rows * cols;

    let base_scale = 1.0 + (count as f64 / 36.0);
    let lognormal = LogNormal::new(0.8, 0.55).unwrap();
    let mut linklet_difficulties = Vec::with_capacity(count);
    let mut total_difficulty = 0.0;
    for _ in 0..count {
        let sample = lognormal.sample(rng);
        let difficulty = (sample * base_scale).max(0.4);
        total_difficulty += difficulty;
        linklet_difficulties.push(difficulty);
    }

    let payout_chain = (total_difficulty * rng.gen_range(0.05_f64..0.09_f64)).max(0.8_f64);
    let market_impact = rng.gen_range(-0.8_f64..1.2_f64);

    MiningJob {
        name,
        rows,
        cols,
        difficulty: total_difficulty,
        payout_chain,
        linklet_difficulties,
        market_impact,
        lore,
    }
}

fn generate_link_id(job: &MiningJob) -> String {
    let difficulty_bucket = ((job.difficulty / 220.0).clamp(0.0, 1.0) * 15.0).floor() as u8;
    let size_bucket = ((job.linklet_difficulties.len() as f64).log2().floor()) as u8;
    let body = nanoid!(6, NANO_ALPHABET);
    let id_core = format!("L{:X}{:X}-{}", difficulty_bucket, size_bucket.min(15), body);
    let hash = blake3::hash(id_core.as_bytes());
    let nibble = (hash.as_bytes()[0] >> 4) as u32;
    let checksum = std::char::from_digit(nibble, 16)
        .unwrap_or('0')
        .to_ascii_uppercase();
    format!("{}-{}", id_core, checksum)
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    if secs >= 3600 {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        format!("{:02}h{:02}m", hours, minutes)
    } else if secs >= 60 {
        let minutes = secs / 60;
        let seconds = secs % 60;
        format!("{:02}m{:02}s", minutes, seconds)
    } else {
        format!("{:02}.{:03}s", secs, millis)
    }
}

pub fn format_price_delta(delta: f64) -> String {
    if delta.abs() < 0.005 {
        "±0.00".to_string()
    } else if delta >= 0.0 {
        format!("+{:.2}", delta)
    } else {
        format!("{:.2}", delta)
    }
}

pub fn format_relings(power: f64) -> String {
    const UNITS: [(&str, f64); 5] = [
        ("Rl/s", 1.0),
        ("kRl/s", 1_000.0),
        ("MRl/s", 1_000_000.0),
        ("GRl/s", 1_000_000_000.0),
        ("TRl/s", 1_000_000_000_000.0),
    ];
    let mut value = power;
    let mut idx = 0usize;
    while value >= 1000.0 && idx + 1 < UNITS.len() {
        value /= 1000.0;
        idx += 1;
    }
    format!("{:.2} {}", value, UNITS[idx].0)
}

#[derive(Serialize, Deserialize)]
struct SaveData {
    focus: PaneFocus,
    mining: MiningSave,
    hashpower_owned: Vec<u32>,
    hashpower_selected: usize,
    bank: BankState,
    ledger: Vec<LedgerEntrySave>,
    ledger_scroll: usize,
    ticker: TickerSave,
    messages: Vec<String>,
}

impl SaveData {
    fn from_app(app: &App) -> Self {
        Self {
            focus: app.focus,
            mining: app.mining.to_save(),
            hashpower_owned: app.hashpower.owned_counts(),
            hashpower_selected: app.hashpower.selected,
            bank: app.bank.clone(),
            ledger: app
                .ledger
                .entries
                .iter()
                .map(LedgerEntrySave::from_entry)
                .collect(),
            ledger_scroll: app.ledger.scroll,
            ticker: app.ticker.to_save(),
            messages: app.messages.iter().cloned().collect(),
        }
    }

    fn apply(self, app: &mut App) -> Result<()> {
        app.focus = self.focus;
        app.mining.apply_save(self.mining);
        app.hashpower
            .apply_owned(&self.hashpower_owned, self.hashpower_selected);
        app.bank = self.bank;
        app.ledger.entries = self
            .ledger
            .into_iter()
            .map(LedgerEntrySave::into_entry)
            .collect::<Result<Vec<_>>>()?;
        if app.ledger.entries.is_empty() {
            app.ledger.scroll = 0;
        } else {
            app.ledger.scroll = self.ledger_scroll.min(app.ledger.entries.len() - 1);
        }
        app.ticker = TickerState::from_save(self.ticker);
        app.messages = VecDeque::from(self.messages);
        while app.messages.len() > MAX_MESSAGES {
            app.messages.pop_back();
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct MiningSave {
    available_jobs: Vec<MiningJob>,
    selected_job: usize,
    active_job: Option<ActiveJobSave>,
}

#[derive(Serialize, Deserialize)]
struct ActiveJobSave {
    job: MiningJob,
    linklets: Vec<LinkletProgress>,
    current_index: usize,
    elapsed_secs: f64,
}

impl ActiveJobSave {
    fn from_active(active: &ActiveJob) -> Self {
        Self {
            job: active.job.clone(),
            linklets: active.linklets.clone(),
            current_index: active.current_index,
            elapsed_secs: active.started_at.elapsed().as_secs_f64(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct LedgerEntrySave {
    id: String,
    name: String,
    finished_at_ms: i64,
    difficulty: f64,
    payout_chain: f64,
    credits_at_completion: f64,
    duration_secs: f64,
    market_impact: f64,
}

impl LedgerEntrySave {
    fn from_entry(entry: &LedgerEntry) -> Self {
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            finished_at_ms: entry.finished_at.timestamp_millis(),
            difficulty: entry.difficulty,
            payout_chain: entry.payout_chain,
            credits_at_completion: entry.credits_at_completion,
            duration_secs: entry.duration.as_secs_f64(),
            market_impact: entry.market_impact,
        }
    }

    fn into_entry(self) -> Result<LedgerEntry> {
        let finished_at = DateTime::<Utc>::from_timestamp_millis(self.finished_at_ms)
            .ok_or_else(|| anyhow!("invalid timestamp in save data"))?;
        Ok(LedgerEntry {
            id: self.id,
            name: self.name,
            finished_at,
            difficulty: self.difficulty,
            payout_chain: self.payout_chain,
            credits_at_completion: self.credits_at_completion,
            duration: Duration::from_secs_f64(self.duration_secs.max(0.0)),
            market_impact: self.market_impact,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct TickerSave {
    price: f64,
    last_delta: f64,
    history: Vec<f64>,
    time_since_update_secs: f64,
    update_interval_secs: f64,
}
