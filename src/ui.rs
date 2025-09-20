use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use std::time::Duration;

use crate::app::{
    ActiveJob, App, LedgerEntry, LinkletStatus, PaneFocus, format_duration, format_price_delta,
    format_relings,
};

pub fn draw(f: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[0]);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[1]);

    draw_mining(f, left_chunks[0], app);
    draw_bank(f, left_chunks[1], app);
    draw_hashpower(f, right_chunks[0], app);
    draw_ledger(f, right_chunks[1], app);

    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(3)])
        .split(chunks[1]);

    draw_ticker(f, footer[0], app);
    draw_footer(f, footer[1], app);
}

fn draw_mining(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = pane_block("Mining", app.focus == PaneFocus::Mining);
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);
    let segments = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    draw_active_job(f, segments[0], app);
    draw_contracts(f, segments[1], app);
}

fn draw_active_job(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .title("Active Link")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    if let Some(active) = &app.mining.active_job {
        let segments = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner);

        let ratio = active.completion_ratio();
        let gauge = Gauge::default()
            .block(Block::default().title(active.job.name.as_str()))
            .ratio(ratio)
            .gauge_style(
                Style::default()
                    .fg(Color::Green)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .label(format!("{:.0}%", ratio * 100.0));
        f.render_widget(gauge, segments[0]);

        let info = build_active_job_lines(active, app.hashpower.total_power());
        let paragraph = Paragraph::new(info).wrap(Wrap { trim: false });
        f.render_widget(paragraph, segments[1]);
    } else {
        let placeholder =
            Paragraph::new("No active mining contract. Select one below and press Enter.")
                .wrap(Wrap { trim: true });
        f.render_widget(placeholder, inner);
    }
}

fn build_active_job_lines(active: &ActiveJob, power: f64) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let job = &active.job;
    let remaining = active.remaining_work();
    let estimate = if power > 0.01 { remaining / power } else { 0.0 };

    lines.push(Line::from(vec![
        Span::styled("Difficulty ", Style::default().fg(Color::Gray)),
        Span::raw(format!("{:.1}", job.difficulty)),
        Span::raw(" | Payout "),
        Span::styled(
            format!("{:.2} ⛓", job.payout_chain),
            Style::default().fg(Color::LightCyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Estimate ", Style::default().fg(Color::Gray)),
        Span::raw(format_duration(Duration::from_secs_f64(estimate))),
        Span::raw(" @ "),
        Span::styled(format_relings(power), Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(""));

    let statuses = active.status_map();
    for row in 0..job.rows {
        let mut buffer = String::new();
        for col in 0..job.cols {
            let idx = row * job.cols + col;
            let ch = match statuses[idx] {
                LinkletStatus::Complete => '✓',
                LinkletStatus::Active => '▒',
                LinkletStatus::Pending => '·',
            };
            buffer.push(ch);
            if col + 1 < job.cols {
                buffer.push(' ');
            }
        }
        lines.push(Line::from(buffer));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        job.lore.clone(),
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )]));

    lines
}

fn draw_contracts(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .title("Contracts")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    if app.mining.available_jobs.is_empty() {
        let paragraph =
            Paragraph::new("No contracts available. Wait for new fragments to surface.")
                .wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .mining
        .available_jobs
        .iter()
        .enumerate()
        .map(|(idx, job)| {
            let est = if app.hashpower.total_power() > 0.01 {
                job.difficulty / app.hashpower.total_power()
            } else {
                0.0
            };
            let content = vec![Line::from(vec![
                Span::styled(job.name.clone(), Style::default().fg(Color::LightCyan)),
                Span::raw(format!("  {}x{}", job.rows, job.cols)),
                Span::raw("  Δ"),
                Span::raw(format!("{:.1}", job.difficulty)),
                Span::raw("  ⛓"),
                Span::raw(format!("{:.2}", job.payout_chain)),
                Span::raw("  η"),
                Span::raw(format_duration(Duration::from_secs_f64(est))),
            ])];
            let mut item = ListItem::new(content);
            if idx == app.mining.selected_job {
                item = item.style(Style::default().fg(Color::Yellow));
            }
            item
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    let mut state = ListState::default();
    state.select(Some(app.mining.selected_job));
    f.render_stateful_widget(list, inner, &mut state);
}

fn draw_hashpower(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = pane_block("Hashpower", app.focus == PaneFocus::Hashpower);
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let total_power = app.hashpower.total_power();
    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled("Total ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_relings(total_power),
            Style::default().fg(Color::LightGreen),
        ),
        Span::raw("  |  Credits "),
        Span::styled(
            format!("{:.2}₵", app.bank.credits_balance),
            Style::default().fg(Color::LightCyan),
        ),
    ])]);
    let segments = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner);
    f.render_widget(header, segments[0]);

    let items: Vec<ListItem> = app
        .hashpower
        .tiers
        .iter()
        .map(|tier| {
            let content = Line::from(vec![
                Span::styled(
                    format!("{:>2}×", tier.owned),
                    Style::default().fg(if tier.owned > 0 {
                        Color::LightGreen
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:<12}", tier.name),
                    Style::default().fg(Color::White),
                ),
                Span::raw(format!(" {:>6.2} Rl/s", tier.total_power())),
                Span::raw("  next:"),
                Span::styled(
                    format!(" {:.2}₵", tier.cost_for_next()),
                    Style::default().fg(Color::LightCyan),
                ),
            ]);
            ListItem::new(vec![content])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default();
    state.select(Some(app.hashpower.selected));
    f.render_stateful_widget(list, segments[1], &mut state);
}

fn draw_bank(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = pane_block("Bank & Exchange", app.focus == PaneFocus::Bank);
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let delta = format_price_delta(app.ticker.last_delta);
    let lines = vec![
        Line::from(vec![
            Span::styled("Chain ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2} ⛓", app.bank.chain_balance),
                Style::default().fg(Color::LightCyan),
            ),
            Span::raw("  |  Credits "),
            Span::styled(
                format!("{:.2}₵", app.bank.credits_balance),
                Style::default().fg(Color::LightGreen),
            ),
        ]),
        Line::from(vec![
            Span::styled("Market ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}₵", app.ticker.price),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  ("),
            Span::styled(delta, Style::default().fg(Color::Gray)),
            Span::raw(")"),
        ]),
        Line::from(""),
        Line::from("← sell 1 ⛓  |  → buy 1 ⛓  |  [B] buy 5  |  [M] sell 5"),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);
}

fn draw_ledger(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = pane_block("Ledger", app.focus == PaneFocus::Ledger);
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    if app.ledger.entries.is_empty() {
        let paragraph = Paragraph::new("Ledger empty. Restore links to uncover history.")
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner);
        return;
    }

    let visible_height = inner.height.saturating_sub(2) as usize;
    let start = app.ledger.scroll.min(app.ledger.entries.len());
    let end = (start + visible_height).min(app.ledger.entries.len());
    let items: Vec<ListItem> = app.ledger.entries[start..end]
        .iter()
        .map(build_ledger_item)
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().fg(Color::Yellow));
    f.render_widget(list, inner);
}

fn build_ledger_item(entry: &LedgerEntry) -> ListItem<'static> {
    let timestamp = entry.finished_local().format("%H:%M:%S");
    let mut market_note = String::new();
    if entry.market_impact.abs() > f64::EPSILON {
        market_note = format!(" Δ{:.2}₵", entry.market_impact);
    }
    let line = Line::from(vec![
        Span::styled(timestamp.to_string(), Style::default().fg(Color::Gray)),
        Span::raw("  "),
        Span::styled(entry.id.clone(), Style::default().fg(Color::LightCyan)),
        Span::raw("  "),
        Span::styled(entry.name.clone(), Style::default().fg(Color::White)),
        Span::raw("  ⛓"),
        Span::styled(
            format!("{:.2}", entry.payout_chain),
            Style::default().fg(Color::White),
        ),
        Span::raw("  ≈"),
        Span::styled(
            format!("{:.2}₵", entry.credits_at_completion),
            Style::default().fg(Color::LightGreen),
        ),
        Span::raw("  Δ"),
        Span::styled(
            format!("{:.1}", entry.difficulty),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  τ"),
        Span::styled(
            format_duration(entry.duration),
            Style::default().fg(Color::Gray),
        ),
        Span::raw(market_note),
    ]);
    ListItem::new(vec![line])
}

fn draw_ticker(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .title("Ticker")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!("Chain {:.2}₵", app.ticker.price),
        Style::default().fg(Color::Yellow),
    ));
    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(
        format!("Credits {:.2}₵", app.bank.credits_balance),
        Style::default().fg(Color::LightGreen),
    ));
    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(
        format!("⛓ {:.2}", app.bank.chain_balance),
        Style::default().fg(Color::LightCyan),
    ));

    let paragraph = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);
    f.render_widget(paragraph, inner);
}

fn draw_footer(f: &mut Frame<'_>, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Ops & Feed")
        .border_style(Style::default().fg(Color::Gray));
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner);

    let instruction_lines = vec![
        Line::from("Tab cycle focus | Q quit"),
        Line::from("Mining: ↑↓ select  Enter accept  Ctrl+R reroll"),
        Line::from("Hashpower: ↑↓ focus tier  Enter purchase"),
        Line::from("Bank: ← sell → buy  B bulk buy  M bulk sell"),
        Line::from("Ledger: ↑↓ scroll"),
    ];
    let instruction = Paragraph::new(instruction_lines).wrap(Wrap { trim: true });
    f.render_widget(instruction, columns[0]);

    let mut message_lines: Vec<Line> = Vec::new();
    for msg in app.messages.iter() {
        message_lines.push(Line::from(Span::raw(msg.clone())));
    }
    if message_lines.is_empty() {
        message_lines.push(Line::from(Span::styled(
            "Awaiting signal...",
            Style::default().fg(Color::DarkGray),
        )));
    }
    let feed = Paragraph::new(message_lines).wrap(Wrap { trim: true });
    f.render_widget(feed, columns[1]);
}

fn pane_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let border_style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    Block::default()
        .title(Span::styled(title, Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style)
}
