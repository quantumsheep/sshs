use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[allow(clippy::wildcard_imports)]
use ratatui::{prelude::*, widgets::*};
use std::{cell::RefCell, error::Error, io, rc::Rc};
use style::palette::tailwind;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use unicode_width::UnicodeWidthStr;

use crate::ssh;

const INFO_TEXT: &str = "(Esc) quit | (↑) move up | (↓) move down | (enter) select";

pub struct AppConfig {
    pub config_path: String,
    pub search_filter: Option<String>,
    pub sort_by_name: bool,

    pub display_proxy_command: bool,
}

pub struct App {
    search: Input,

    table_state: TableState,
    hosts: Vec<ssh::Host>,
    table_longest_item_lens: (u16, u16, u16, u16, u16),

    palette: tailwind::Palette,
}

impl App {
    /// # Errors
    ///
    /// Will return `Err` if the SSH configuration file cannot be parsed.
    pub fn new(config: &AppConfig) -> Result<App, Box<dyn Error>> {
        let mut hosts = ssh::parse_config(&config.config_path)?;
        if config.sort_by_name {
            hosts.sort_by(|a, b| a.hostname.cmp(&b.hostname));
        }

        let search_input = config.search_filter.clone().unwrap_or_default();

        Ok(App {
            search: search_input.into(),

            table_state: TableState::default().with_selected(0),
            table_longest_item_lens: constraint_len_calculator(&hosts),
            palette: tailwind::BLUE,

            hosts,
        })
    }

    /// # Errors
    ///
    /// Will return `Err` if the terminal cannot be configured.
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        // setup terminal
        enable_raw_mode()?;

        let mut stdout = io::stdout().lock();
        execute!(stdout, Hide, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal: Rc<RefCell<Terminal<CrosstermBackend<io::StdoutLock<'_>>>>> =
            Rc::new(RefCell::new(Terminal::new(backend)?));

        // create app and run it
        let res = self.run(&terminal);

        restore_terminal(&terminal)?;

        if let Err(err) = res {
            println!("{err:?}");
        }

        Ok(())
    }

    fn run<B: Backend>(&mut self, terminal: &Rc<RefCell<Terminal<B>>>) -> Result<(), Box<dyn Error>>
    where
        B: std::io::Write,
    {
        loop {
            terminal.borrow_mut().draw(|f| ui(f, self))?;

            let ev = event::read()?;

            if let Event::Key(key) = ev {
                if key.kind == KeyEventKind::Press {
                    #[allow(clippy::enum_glob_use)]
                    use KeyCode::*;

                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        #[allow(clippy::single_match)]
                        match key.code {
                            Char('c') => return Ok(()),
                            _ => {}
                        }
                    }

                    match key.code {
                        Esc => return Ok(()),
                        Down => self.next(),
                        Up => self.previous(),
                        Enter => {
                            let selected = self.table_state.selected().unwrap_or(0);
                            let host = &self.hosts[selected];

                            restore_terminal(terminal).expect("Failed to restore terminal");
                            ssh::connect(host)?;

                            return Ok(());
                        }
                        _ => {
                            self.search.handle_event(&ev);
                        }
                    }
                }
            }
        }
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.hosts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.hosts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
}

fn restore_terminal<B: Backend>(terminal: &Rc<RefCell<Terminal<B>>>) -> Result<(), Box<dyn Error>>
where
    B: std::io::Write,
{
    let mut terminal = terminal.borrow_mut();

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        Show,
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let rects = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(f.size());

    render_searchbar(f, app, rects[0]);

    render_table(f, app, rects[1]);

    render_footer(f, app, rects[2]);

    f.set_cursor(
        rects[0].x + u16::try_from(app.search.cursor()).unwrap_or_default() + 4,
        rects[0].y + 1,
    );
}

fn render_searchbar(f: &mut Frame, app: &mut App, area: Rect) {
    let info_footer = Paragraph::new(Line::from(app.search.value())).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.palette.c400))
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(3)),
    );
    f.render_widget(info_footer, area);
}

fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default().fg(tailwind::CYAN.c500);
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);

    let header = ["Hostname", "Aliases", "User", "Target", "Port"]
        .iter()
        .copied()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);

    let search_value = app.search.value();

    let rows = app
        .hosts
        .iter()
        .filter(|host| {
            search_value.is_empty()
                || (host.hostname.contains(search_value) || host.aliases.contains(search_value))
        })
        .map(|host| {
            [
                &host.hostname,
                &host.aliases,
                &host.user,
                &host.target,
                &host.port,
            ]
            .iter()
            .copied()
            .map(|content| Cell::from(Text::from(content.to_string())))
            .collect::<Row>()
        });

    let bar = " █ ";
    let t = Table::new(
        rows,
        [
            // + 1 is for padding.
            Constraint::Length(app.table_longest_item_lens.0 + 1),
            Constraint::Min(app.table_longest_item_lens.1 + 1),
            Constraint::Min(app.table_longest_item_lens.2 + 1),
            Constraint::Min(app.table_longest_item_lens.3 + 1),
            Constraint::Min(app.table_longest_item_lens.4 + 1),
        ],
    )
    .header(header)
    .highlight_style(selected_style)
    .highlight_symbol(Text::from(vec![
        "".into(),
        bar.into(),
        bar.into(),
        "".into(),
    ]))
    .highlight_spacing(HighlightSpacing::Always)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.palette.c400))
            .border_type(BorderType::Rounded),
    );

    f.render_stateful_widget(t, area, &mut app.table_state);
}

fn constraint_len_calculator(items: &[ssh::Host]) -> (u16, u16, u16, u16, u16) {
    let hostname_len = items
        .iter()
        .map(|d| d.hostname.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let aliases_len = items
        .iter()
        .map(|d| d.aliases.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let user_len = items
        .iter()
        .map(|d| d.user.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let target_len = items
        .iter()
        .map(|d| d.target.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let port_len = items
        .iter()
        .map(|d| d.port.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    (
        u16::try_from(hostname_len).unwrap_or_default(),
        u16::try_from(aliases_len).unwrap_or_default(),
        u16::try_from(user_len).unwrap_or_default(),
        u16::try_from(target_len).unwrap_or_default(),
        u16::try_from(port_len).unwrap_or_default(),
    )
}

fn render_footer(f: &mut Frame, app: &mut App, area: Rect) {
    let info_footer = Paragraph::new(Line::from(INFO_TEXT)).centered().block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(app.palette.c400))
            .border_type(BorderType::Rounded),
    );
    f.render_widget(info_footer, area);
}