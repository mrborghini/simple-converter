//! The ffmpeg availability banner at the top of the window.

use iced::widget::{button, container, progress_bar, row, text};
use iced::{Alignment, Element, Length};

use super::megabytes;
use crate::app::{App, FfmpegState, Message};

pub fn view(app: &App) -> Element<'_, Message> {
    let (label, progress, style): (String, Option<f32>, fn(&iced::Theme) -> container::Style) =
        match &app.ffmpeg {
            FfmpegState::Checking => (
                "Checking for ffmpeg...".to_string(),
                None,
                container::secondary,
            ),
            FfmpegState::Downloading { done, total } => {
                let label = match total {
                    Some(total) => format!(
                        "Downloading ffmpeg: {} of {}",
                        megabytes(*done),
                        megabytes(*total)
                    ),
                    None => format!("Downloading ffmpeg: {}", megabytes(*done)),
                };
                let fraction = total
                    .filter(|t| *t > 0)
                    .map(|t| (*done as f32 / t as f32).clamp(0.0, 1.0));
                (label, Some(fraction.unwrap_or(0.0)), container::secondary)
            }
            FfmpegState::Extracting => (
                "Extracting ffmpeg...".to_string(),
                Some(1.0),
                container::secondary,
            ),
            FfmpegState::Ready(paths) => (paths.describe(), None, container::success),
            FfmpegState::Failed(e) => (
                format!("ffmpeg is not available: {e}"),
                None,
                container::danger,
            ),
        };

    let mut content = row![text(label).width(Length::Fill)]
        .spacing(12)
        .align_y(Alignment::Center);
    if let Some(value) = progress {
        content = content.push(progress_bar(0.0..=1.0, value).length(Length::Fixed(220.0)));
    }
    if matches!(app.ffmpeg, FfmpegState::Failed(_)) {
        content = content.push(button("Retry download").on_press(Message::RedownloadFfmpeg));
    }

    container(content).padding(12).style(style).into()
}
