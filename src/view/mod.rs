//! Widget tree. Pure functions of the app state.

mod advanced;
mod simple;
mod status;

use iced::widget::{column, container, scrollable};
use iced::{Element, Length};

use crate::app::{App, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let content = column![status::view(app), simple::view(app), advanced::view(app)]
        .spacing(16)
        .padding(20)
        .max_width(960);

    scrollable(container(content).center_x(Length::Fill))
        .height(Length::Fill)
        .into()
}

/// Bytes as a short megabyte string.
pub fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}
