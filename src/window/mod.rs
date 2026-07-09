pub mod delete;

use anyhow::Result;
use crossterm::event::KeyEvent;
pub use delete::DeletePopupWindow;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::ui::AppKeyAction;

pub trait PopupWindow {
    type ShowData;
    type OnKeyPressData;

    fn on_key_press(
        &mut self,
        key: KeyEvent,
        data: &mut Self::OnKeyPressData,
    ) -> Result<AppKeyAction>;

    fn is_active(&self) -> bool;
    fn show(&mut self, data: Self::ShowData);
    fn hide(&mut self);
    fn toggle(&mut self);
    fn render(&mut self, f: &mut Frame);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

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
