use std::io;

use anyhow::{Result, anyhow};
use clap::Args;
use crossterm::{
    event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};
use tokio::{sync::mpsc, task::JoinHandle, time};
use tokio_util::sync::CancellationToken;

use crate::{
    checker::MojangChecker,
    model::{SearchEvent, SearchProgress, SearchSummary},
    search::run_search,
    validation::{is_valid_name_char, validate_search_options},
};

#[derive(Clone, Debug, Args)]
pub struct TuiArgs {
    #[arg(long, default_value_t = 4)]
    pub length: u8,
    #[arg(long = "starts-with", default_value = "")]
    pub starts_with: String,
    #[arg(long, default_value_t = 10)]
    pub results: usize,
    #[arg(long, default_value_t = 200)]
    pub max_checks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Length,
    Prefix,
    Results,
    MaxChecks,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Self::Length => Self::Prefix,
            Self::Prefix => Self::Results,
            Self::Results => Self::MaxChecks,
            Self::MaxChecks => Self::Length,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Length => Self::MaxChecks,
            Self::Prefix => Self::Length,
            Self::Results => Self::Prefix,
            Self::MaxChecks => Self::Results,
        }
    }
}

#[derive(Debug)]
enum UiMessage {
    SearchEvent(SearchEvent),
    SearchFailed(String),
}

#[derive(Debug)]
struct App {
    length: String,
    prefix: String,
    results: String,
    max_checks: String,
    selected: Field,
    status: String,
    progress: SearchProgress,
    hits: Vec<String>,
    searching: bool,
    should_quit: bool,
    cancel: Option<CancellationToken>,
    search_task: Option<JoinHandle<()>>,
    tx: mpsc::UnboundedSender<UiMessage>,
    rx: mpsc::UnboundedReceiver<UiMessage>,
}

const ACCENT: Color = Color::Rgb(92, 197, 176);
const ACCENT_SOFT: Color = Color::Rgb(74, 120, 136);
const TEXT: Color = Color::Rgb(229, 236, 240);
const MUTED: Color = Color::Rgb(129, 146, 156);
const SUCCESS: Color = Color::Rgb(120, 214, 151);
const WARN: Color = Color::Rgb(245, 183, 84);
const DANGER: Color = Color::Rgb(239, 98, 98);

impl App {
    fn new(args: TuiArgs) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            length: args.length.to_string(),
            prefix: args.starts_with,
            results: args.results.to_string(),
            max_checks: args.max_checks.to_string(),
            selected: Field::Length,
            status: "Console ready. Press Enter to launch a scan.".to_string(),
            progress: SearchProgress::default(),
            hits: Vec::new(),
            searching: false,
            should_quit: false,
            cancel: None,
            search_task: None,
            tx,
            rx,
        }
    }

    fn start_search(&mut self) -> Result<()> {
        let length = self
            .length
            .parse::<u8>()
            .map_err(|_| anyhow!("length must be a number"))?;
        let results = self
            .results
            .parse::<usize>()
            .map_err(|_| anyhow!("results must be a number"))?;
        let max_checks = self
            .max_checks
            .parse::<usize>()
            .map_err(|_| anyhow!("max checks must be a number"))?;

        let options = validate_search_options(length, &self.prefix, results, max_checks)?;

        if let Some(task) = self.search_task.take() {
            task.abort();
        }

        self.progress = SearchProgress::default();
        self.hits.clear();
        self.status = "Scanning Mojang public API for likely-available names...".to_string();
        self.searching = true;

        let tx = self.tx.clone();
        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        self.cancel = Some(cancel);

        self.search_task = Some(tokio::spawn(async move {
            let checker = match MojangChecker::new() {
                Ok(checker) => checker,
                Err(error) => {
                    let _ = tx.send(UiMessage::SearchFailed(error.to_string()));
                    return;
                }
            };

            let result = run_search(options, &checker, worker_cancel, |event| {
                let _ = tx.send(UiMessage::SearchEvent(event));
            })
            .await;

            if let Err(error) = result {
                let _ = tx.send(UiMessage::SearchFailed(error.to_string()));
            }
        }));

        Ok(())
    }

    fn stop_search(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
            self.status = "Stopping scan...".to_string();
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.searching {
                    self.stop_search();
                }
                self.should_quit = true;
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Down => self.selected = self.selected.next(),
            KeyCode::BackTab | KeyCode::Left | KeyCode::Up => {
                self.selected = self.selected.previous()
            }
            KeyCode::Enter => {
                if self.searching {
                    self.stop_search();
                } else if let Err(error) = self.start_search() {
                    self.status = error.to_string();
                }
            }
            KeyCode::Backspace => {
                if !self.searching {
                    self.remove_char();
                }
            }
            KeyCode::Char(ch) => {
                if !self.searching {
                    self.insert_char(ch);
                }
            }
            _ => {}
        }
    }

    fn remove_char(&mut self) {
        match self.selected {
            Field::Length => {
                self.length.pop();
            }
            Field::Prefix => {
                self.prefix.pop();
            }
            Field::Results => {
                self.results.pop();
            }
            Field::MaxChecks => {
                self.max_checks.pop();
            }
        }
    }

    fn insert_char(&mut self, ch: char) {
        match self.selected {
            Field::Length | Field::Results | Field::MaxChecks => {
                if ch.is_ascii_digit() {
                    let target = match self.selected {
                        Field::Length => &mut self.length,
                        Field::Results => &mut self.results,
                        Field::MaxChecks => &mut self.max_checks,
                        Field::Prefix => unreachable!(),
                    };
                    target.push(ch);
                }
            }
            Field::Prefix => {
                if is_valid_name_char(ch) {
                    self.prefix.push(ch);
                }
            }
        }
    }

    fn handle_ui_message(&mut self, message: UiMessage) {
        match message {
            UiMessage::SearchEvent(SearchEvent::Progress(progress)) => {
                self.progress = progress;
            }
            UiMessage::SearchEvent(SearchEvent::Hit(name)) => {
                self.hits.push(name);
            }
            UiMessage::SearchEvent(SearchEvent::Finished(summary)) => {
                self.finish_search(summary);
            }
            UiMessage::SearchFailed(error) => {
                self.searching = false;
                self.cancel = None;
                self.status = format!("Search failed: {error}");
            }
        }
    }

    fn finish_search(&mut self, summary: SearchSummary) {
        self.searching = false;
        self.cancel = None;
        self.progress = summary.progress;
        self.status = format!(
            "Complete: {}. Logged {} likely-available names.",
            summary.stop_reason.label(),
            self.progress.found
        );
    }
}

pub async fn run_tui(args: TuiArgs) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(error.into());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error.into());
        }
    };
    let result = run_app(&mut terminal, App::new(args)).await;

    let cleanup = (|| -> Result<()> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    })();

    result.and(cleanup)
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> Result<()> {
    let mut events = EventStream::new();
    let mut tick = time::interval(std::time::Duration::from_millis(200));

    while !app.should_quit {
        terminal.draw(|frame| draw(frame, &app))?;

        tokio::select! {
            _ = tick.tick() => {}
            Some(message) = app.rx.recv() => app.handle_ui_message(message),
            maybe_event = events.next() => {
                if let Some(Ok(CrosstermEvent::Key(key))) = maybe_event {
                    app.handle_key(key);
                }
            }
        }
    }

    if let Some(cancel) = &app.cancel {
        cancel.cancel();
    }

    if let Some(task) = app.search_task.take() {
        task.abort();
        let _ = task.await;
    }

    Ok(())
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let root = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(14),
        Constraint::Length(3),
    ])
    .split(frame.area());

    draw_header(frame, root[0], app);

    if root[1].width >= 92 {
        let columns = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(root[1]);
        let left = Layout::vertical([Constraint::Length(5), Constraint::Min(6)]).split(columns[0]);
        draw_form(frame, left[0], app);
        draw_overview(frame, left[1], app);
        draw_results(frame, columns[1], app);
    } else {
        let sections = Layout::vertical([
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Min(8),
        ])
        .split(root[1]);
        draw_form(frame, sections[0], app);
        draw_overview(frame, sections[1], app);
        draw_results(frame, sections[2], app);
    }

    draw_footer(frame, root[2], app);
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let badge = if app.searching {
        Span::styled(
            " LIVE ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " READY ",
            Style::default()
                .fg(Color::Black)
                .bg(WARN)
                .add_modifier(Modifier::BOLD),
        )
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "MNF",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "Mission Control",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            badge,
        ]),
        Line::from(vec![
            Span::styled("Public Mojang lookup", Style::default().fg(MUTED)),
            Span::raw("  |  "),
            Span::styled("results are likely available", Style::default().fg(WARN)),
        ]),
    ])
    .block(panel("Terminal Radar", ACCENT));
    frame.render_widget(header, area);
}

fn draw_form(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = vec![
        control_pair_line(
            ("Length", &app.length, app.selected == Field::Length),
            (
                "Prefix",
                display_value(&app.prefix),
                app.selected == Field::Prefix,
            ),
        ),
        control_pair_line(
            ("Results", &app.results, app.selected == Field::Results),
            ("Max", &app.max_checks, app.selected == Field::MaxChecks),
        ),
        Line::from(vec![
            key_span("Enter"),
            Span::styled(" launch / stop  ", Style::default().fg(MUTED)),
            key_span("Tab"),
            Span::styled(" switch field", Style::default().fg(MUTED)),
        ]),
    ];

    let widget = Paragraph::new(lines).block(panel("Search Plan", ACCENT_SOFT));
    frame.render_widget(widget, area);
}

fn draw_overview(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let budget_target = parse_target(&app.max_checks);
    let results_target = parse_target(&app.results);
    let telemetry = Paragraph::new(vec![
        Line::from(vec![
            state_span(app.searching),
            Span::raw("   "),
            Span::styled(
                format!("generated {}", app.progress.generated),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                format!("batches {}", app.progress.batches),
                Style::default().fg(WARN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("status ", Style::default().fg(MUTED)),
            Span::styled(app.status.as_str(), Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled(
                format!(
                    "checked {}/{} {}",
                    app.progress.checked,
                    budget_target,
                    compact_bar(app.progress.checked, budget_target, 8)
                ),
                Style::default().fg(ACCENT),
            ),
            Span::raw("   "),
            Span::styled(
                format!(
                    "hits {}/{} {}",
                    app.progress.found,
                    results_target,
                    compact_bar(app.progress.found, results_target, 8)
                ),
                Style::default().fg(SUCCESS),
            ),
        ]),
    ])
    .block(panel("Telemetry", ACCENT_SOFT));
    frame.render_widget(telemetry, area);
}

fn draw_results(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = if app.searching {
        "Live Hits"
    } else {
        "Results Archive"
    };
    let items: Vec<ListItem<'_>> = if app.hits.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(
                "stand by",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "No likely-available names logged yet.",
                Style::default().fg(TEXT),
            ),
        ]))]
    } else {
        app.hits
            .iter()
            .enumerate()
            .map(|(index, name)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:02}", index + 1), Style::default().fg(MUTED)),
                    Span::raw("  "),
                    Span::styled(
                        name.clone(),
                        Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled("likely available", Style::default().fg(MUTED)),
                ]))
            })
            .collect()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(SUCCESS))
        .title(Span::styled(
            format!("{title} [{}]", app.hits.len()),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ));
    let results = List::new(items).block(block);
    frame.render_widget(results, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let action = if app.searching {
        "stop scan"
    } else {
        "start scan"
    };
    let edit_hint = if app.searching {
        "editing locked during scan"
    } else {
        "Backspace edit"
    };
    let footer = Paragraph::new(Line::from(vec![
        key_span("Enter"),
        Span::styled(format!(" {action}  "), Style::default().fg(MUTED)),
        key_span("Tab"),
        Span::styled(" focus  ", Style::default().fg(MUTED)),
        key_span("Edit"),
        Span::styled(format!(" {edit_hint}  "), Style::default().fg(MUTED)),
        key_span("Q"),
        Span::styled(" quit", Style::default().fg(MUTED)),
    ]))
    .block(panel("Controls", ACCENT));
    frame.render_widget(footer, area);
}

fn panel(title: &'static str, border: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            title,
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ))
}

fn control_pair_line<'a>(
    left: (&'a str, &'a str, bool),
    right: (&'a str, &'a str, bool),
) -> Line<'a> {
    Line::from(vec![
        control_span(left.0, left.1, left.2),
        Span::raw("   "),
        control_span(right.0, right.1, right.2),
    ])
}

fn control_span<'a>(label: &'a str, value: &'a str, selected: bool) -> Span<'a> {
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };

    Span::styled(format!("{label}: {value}"), style)
}

fn key_span<'a>(label: &'a str) -> Span<'a> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(Color::Black)
            .bg(WARN)
            .add_modifier(Modifier::BOLD),
    )
}

fn state_span(searching: bool) -> Span<'static> {
    let (label, color) = if searching {
        ("state live", SUCCESS)
    } else {
        ("state idle", DANGER)
    };

    Span::styled(
        label,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn parse_target(value: &str) -> u64 {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn compact_bar(current: u64, target: u64, width: usize) -> String {
    let filled = if target == 0 {
        0
    } else {
        ((current.min(target) * width as u64) / target) as usize
    };

    let mut bar = String::with_capacity(width + 2);
    bar.push('[');
    for index in 0..width {
        bar.push(if index < filled { '=' } else { '-' });
    }
    bar.push(']');
    bar
}

fn display_value(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}
