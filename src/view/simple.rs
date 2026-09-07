//! Simple mode: pick a file, pick an output type, convert.

use iced::widget::{
    Column, button, checkbox, column, container, pick_list, progress_bar, row, text, text_input,
};
use iced::{Alignment, Element, Font, Length};

use crate::app::{App, JobState, Message};
use crate::formats;

const LABEL_WIDTH: f32 = 110.0;

pub fn view(app: &App) -> Element<'_, Message> {
    let file_name = app
        .input
        .as_ref()
        .and_then(|i| i.path.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "No file selected".to_string());

    let mut pick_row = row![
        button("Choose file...").on_press(Message::PickFile),
        text(file_name),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if !app.wayland && app.input.is_none() {
        pick_row =
            pick_row.push(text("or drop a file anywhere in this window").style(text::secondary));
    }

    let summary = app
        .input
        .as_ref()
        .map(|i| text(i.probe.summary()).size(13).style(text::secondary));

    let is_image = app.input.as_ref().is_some_and(|i| i.probe.is_image);
    let audio_locked = app
        .input
        .as_ref()
        .is_some_and(|i| !i.probe.has_video && i.probe.has_audio);
    let options = formats::options(app.effective_audio_only(), is_image);

    let mut format_row = row![
        text("Output type").width(Length::Fixed(LABEL_WIDTH)),
        pick_list(options, Some(app.format), Message::FormatChosen).width(Length::Fixed(240.0)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if !is_image {
        format_row = format_row.push(
            checkbox(app.effective_audio_only())
                .label("Audio only")
                .on_toggle_maybe((!audio_locked).then_some(Message::AudioOnlyToggled)),
        );
    }

    let output_row = row![
        text("Save to").width(Length::Fixed(LABEL_WIDTH)),
        text_input("Output file", &app.output)
            .on_input(Message::OutputEdited)
            .width(Length::Fill),
        button("Save as...").on_press(Message::SaveAs),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut actions = row![
        button(text("Convert").size(16))
            .padding([10, 28])
            .on_press_maybe(app.can_convert().then_some(Message::Convert)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut result: Column<'_, Message> = column![].spacing(8);
    match &app.job {
        JobState::Idle => {}
        JobState::Probing => actions = actions.push(text("Reading file...").style(text::secondary)),
        JobState::Converting { percent, speed } => {
            actions = actions.push(
                button("Cancel")
                    .style(button::danger)
                    .on_press(Message::Cancel),
            );
            let value = percent.unwrap_or(0.0);
            actions = actions.push(progress_bar(0.0..=100.0, value).length(Length::Fill));
            let label = match percent {
                Some(p) => format!("{p:.0}% {speed}"),
                None => format!("Working... {speed}"),
            };
            actions = actions.push(text(label).width(Length::Fixed(110.0)));
        }
        JobState::Done(path) => {
            result = result.push(
                row![
                    text(format!("Saved to {}", path.display()))
                        .style(text::success)
                        .width(Length::Fill),
                    button("Show in folder")
                        .style(button::secondary)
                        .on_press(Message::Reveal),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        JobState::Failed(error) => {
            result = result.push(
                container(
                    text(error)
                        .font(Font::MONOSPACE)
                        .size(12)
                        .style(text::danger),
                )
                .padding(10)
                .width(Length::Fill)
                .style(container::bordered_box),
            );
        }
    }

    let mut content = column![pick_row].spacing(14);
    if let Some(summary) = summary {
        content = content.push(summary);
    }
    let content = content
        .push(format_row)
        .push(output_row)
        .push(actions)
        .push(result);

    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(container::rounded_box)
        .into()
}
