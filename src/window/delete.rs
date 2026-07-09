use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use crate::{ssh::Host, ssh_config, ui::AppKeyAction};

use super::{centered_rect, PopupWindow};

pub struct ShowData {
    hosts: Vec<Host>,
    host_to_delete_index: usize,
    host_to_delete: Host,
}

impl ShowData {
    pub fn new(hosts: Vec<Host>, host_to_delete_index: usize, host_to_delete: Host) -> Self {
        Self {
            hosts,
            host_to_delete_index,
            host_to_delete,
        }
    }
}

pub struct OnKeyPressData {
    pub config_paths: Vec<String>,
    pub hosts: Vec<Host>,
}

impl OnKeyPressData {
    pub fn new(config_paths: Vec<String>, hosts: Vec<Host>) -> Self {
        Self {
            config_paths,
            hosts,
        }
    }
}

#[derive(Default)]
pub struct DeletePopupWindow {
    is_active: bool,

    selected_button_index: usize,
    show_data: Option<ShowData>,
}

impl PopupWindow for DeletePopupWindow {
    type ShowData = ShowData;
    type OnKeyPressData = OnKeyPressData;

    fn on_key_press(
        &mut self,
        key: KeyEvent,
        data: &mut Self::OnKeyPressData,
    ) -> Result<AppKeyAction> {
        #[allow(clippy::enum_glob_use)]
        use KeyCode::*;

        match key.code {
            Enter => {
                if self.selected_button_index == 1 {
                    self.hide();
                    return Ok(AppKeyAction::Continue);
                }

                // Select first path from the config
                // let path = &data.config_paths.get(0).unwrap();
                let path = "~/.ssh/config";
                let show_data = self.show_data.as_ref().unwrap();

                let new_hosts = show_data
                    .hosts
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != show_data.host_to_delete_index)
                    .map(|(_, host)| host.clone())
                    .collect::<Vec<Host>>();

                data.hosts = new_hosts.clone();

                ssh_config::Parser::save_into_file(new_hosts, path)?;
                self.hide();
            }
            Esc => self.hide(),
            Left => {
                if self.is_active() {
                    self.previous();
                }
            }
            Right => {
                if self.is_active() {
                    self.next();
                }
            }
            _ => return Ok(AppKeyAction::Continue),
        }

        Ok(AppKeyAction::Ok)
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn show(&mut self, data: Self::ShowData) {
        self.show_data = Some(data);
        self.is_active = true;
    }

    fn hide(&mut self) {
        self.is_active = false;
    }

    fn toggle(&mut self) {
        self.is_active = !self.is_active;
    }

    fn render(&mut self, f: &mut Frame) {
        if let Some(show_data) = &self.show_data {
            let popup_area = centered_rect(50, 20, f.area());

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .margin(1)
                .split(popup_area);

            let block = Block::bordered().border_type(BorderType::Rounded);
            // .border_style(Style::new().fg(Palette:));

            f.render_widget(Clear, popup_area);
            f.render_widget(block, popup_area);

            let question = Paragraph::new(Text::from(vec![Line::from(vec![Span::raw(format!(
                "Delete `{}` record?",
                show_data.host_to_delete.name
            ))])
            .bold()]))
            .alignment(Alignment::Center);

            f.render_widget(question, chunks[0]);

            let yes_style = if self.selected_button_index == 0 {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };

            let no_style = if self.selected_button_index == 1 {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Red)
            };

            let buttons_line = Line::from(vec![
                Span::styled("  Yes  ", yes_style),
                Span::raw("   "),
                Span::styled("  No  ", no_style),
            ]);

            let buttons = Paragraph::new(Text::from(buttons_line)).alignment(Alignment::Center);

            f.render_widget(buttons, chunks[2]);
        }
    }
}

impl DeletePopupWindow {
    pub fn next(&mut self) {
        self.selected_button_index = 1;
    }

    pub fn previous(&mut self) {
        self.selected_button_index = 0;
    }
}
