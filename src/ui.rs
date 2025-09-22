use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Sparkline, Wrap,
};
use std::time::Duration;

use crate::app::{
    ActiveJob, App, LedgerEntry, LinkletStatus, PaneFocus, format_duration, format_price_delta,
    format_relings,
};

pub fn draw(f: &mut Frame<'_>, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(f.size());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[1]);

    let lower_right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_chunks[1]);

    draw_mining(f, left_chunks[0], app);
    draw_bank(f, left_chunks[1], app);
    draw_hashpower(f, right_chunks[0], app);
    draw_ledger(f, lower_right[0], app);
    draw_ticker(f, lower_right[1], app);

    if app.paused {
        draw_pause_overlay(f, app);
    }
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
    const GLYPHS: [char; 5] = ['·', '░', '▒', '▓', '█'];
    for row in 0..job.rows {
        let mut spans = Vec::new();
        for col in 0..job.cols {
            let idx = row * job.cols + col;
            let linklet = &active.linklets[idx];
            let progress = if linklet.difficulty <= f64::EPSILON {
                1.0
            } else {
                1.0 - (linklet.remaining / linklet.difficulty).clamp(0.0, 1.0)
            };
            let glyph_index = ((progress * ((GLYPHS.len() - 1) as f64)).round() as usize)
                .clamp(0, GLYPHS.len() - 1);
            let glyph = GLYPHS[glyph_index];
            let style = match statuses[idx] {
                LinkletStatus::Complete => Style::default().fg(Color::LightGreen),
                LinkletStatus::Active => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                LinkletStatus::Pending => Style::default().fg(Color::DarkGray),
            };
            spans.push(Span::styled(glyph.to_string(), style));
            if col + 1 < job.cols {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
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
                Span::raw("  ⛓ "),
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
            let owned_style = if tier.owned > 0 {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let total_power = format_relings(tier.total_power());
            let unit_power = format_relings(tier.power);
            let content = Line::from(vec![
                Span::styled(format!("{:>2}×", tier.owned), owned_style),
                Span::raw(" "),
                Span::styled(
                    format!("{:<14}", tier.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(" {:>10}", total_power),
                    Style::default().fg(Color::LightGreen),
                ),
                Span::raw(" total"),
                Span::raw("  +"),
                Span::styled(unit_power, Style::default().fg(Color::Gray)),
                Span::raw("/ea  next:"),
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
        Line::from("← sell 1 chain  |  → buy 1 chain  |  [B] buy 5  |  [M] sell 5"),
        Line::from("Spread 1%: sells settle at 0.99×, buys at 1.01× market."),
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
        Span::raw("  ⛓ "),
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
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner);

    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!("Chain {:.2}₵", app.ticker.price),
        Style::default().fg(Color::Yellow),
    ));
    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(
        format!("Δ {}₵", format_price_delta(app.ticker.last_delta)),
        Style::default().fg(Color::Gray),
    ));
    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(
        format!("Credits {:.2}₵", app.bank.credits_balance),
        Style::default().fg(Color::LightGreen),
    ));
    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(
        format!("Holdings {:.2}", app.bank.chain_balance),
        Style::default().fg(Color::LightCyan),
    ));
    spans.push(Span::raw("  |  "));
    spans.push(Span::styled(
        format!("Next {:.1}s", app.ticker.seconds_until_update()),
        Style::default().fg(Color::LightMagenta),
    ));

    let header = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);
    f.render_widget(header, layout[0]);

    if layout[1].height > 0 && layout[1].width > 0 {
        let width = layout[1].width as usize;
        let mut history: Vec<f64> = app
            .ticker
            .history
            .iter()
            .rev()
            .take(width)
            .copied()
            .collect();
        if !history.is_empty() {
            history.reverse();
            let min = history
                .iter()
                .fold(f64::INFINITY, |acc, value| acc.min(*value));
            let max = history
                .iter()
                .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value));
            let range = (max - min).max(0.01);
            let data: Vec<u64> = history
                .into_iter()
                .map(|value| (((value - min) / range) * 100.0).round().clamp(0.0, 100.0) as u64)
                .collect();
            let sparkline = Sparkline::default()
                .data(&data)
                .style(Style::default().fg(Color::LightGreen));
            f.render_widget(sparkline, layout[1]);
        } else {
            let placeholder = Paragraph::new("Market data stabilising...")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, layout[1]);
        }
    }

    if layout[2].height > 0 {
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
        f.render_widget(feed, layout[2]);
    }
}

fn draw_pause_overlay(f: &mut Frame<'_>, app: &App) {
    let area = centered_rect(40, 50, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            "Paused",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    f.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(inner);

    let items: Vec<ListItem> = app
        .pause_menu
        .items()
        .iter()
        .map(|item| {
            ListItem::new(Line::from(vec![Span::styled(
                item.label(),
                Style::default().fg(Color::White),
            )]))
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.pause_menu.selected()));
    let list = List::new(items).block(Block::default()).highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, layout[0], &mut state);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(status) = app.pause_menu.status() {
        lines.push(Line::from(vec![Span::styled(
            status.clone(),
            Style::default().fg(Color::LightCyan),
        )]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from("↑↓ select  Enter confirm  Esc resume"));
    lines.push(Line::from(""));
    lines.push(Line::from("Tab cycle focus  |  Q pause"));
    lines.push(Line::from("Mining: ↑↓ select  Enter accept  Ctrl+R reroll"));
    lines.push(Line::from("Hashpower: ↑↓ focus tier  Enter purchase"));
    lines.push(Line::from("Bank: ← sell  → buy  B bulk buy  M bulk sell"));
    lines.push(Line::from("Ledger: ↑↓ scroll"));
    let status = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(status, layout[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
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
