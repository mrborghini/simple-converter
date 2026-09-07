//! Collapsible advanced panel: templates, codec settings and the command preview.

use iced::widget::{
    Column, button, checkbox, column, container, pick_list, row, rule, slider, text, text_input,
};
use iced::{Alignment, Element, Font, Length};

use crate::app::{AdvMessage, App, Message};
use crate::ffmpeg::args::{
    AudioCodec, Channels, Fps, Preset, QualityMode, Resolution, SampleRate, VideoCodec,
};
use crate::templates::Template;

pub fn view(app: &App) -> Element<'_, Message> {
    let toggle_label = if app.advanced_open {
        "Hide advanced options"
    } else {
        "Show advanced options"
    };
    let toggle = button(toggle_label)
        .style(button::text)
        .on_press(Message::ToggleAdvanced);

    if !app.advanced_open {
        return toggle.into();
    }

    let adv = &app.adv;

    let template_row = row![
        text("Template").width(Length::Fixed(110.0)),
        pick_list(Template::ALL, app.template, Message::TemplateChosen)
            .placeholder("Custom")
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let codec_row = row![
        pick_list(VideoCodec::ALL, Some(adv.video_codec), |v| Message::Adv(
            AdvMessage::VideoCodec(v)
        )),
        pick_list(Resolution::ALL, Some(adv.resolution), |v| Message::Adv(
            AdvMessage::Resolution(v)
        )),
        pick_list(Fps::ALL, Some(adv.fps), |v| Message::Adv(AdvMessage::Fps(
            v
        ))),
        pick_list(Preset::ALL, Some(adv.preset), |v| Message::Adv(
            AdvMessage::Preset(v)
        )),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut quality_row = row![pick_list(QualityMode::ALL, Some(adv.quality_mode), |v| {
        Message::Adv(AdvMessage::QualityMode(v))
    }),]
    .spacing(10)
    .align_y(Alignment::Center);
    match adv.quality_mode {
        QualityMode::Crf => {
            quality_row = quality_row
                .push(
                    slider(0..=51u8, adv.crf, |v| Message::Adv(AdvMessage::Crf(v)))
                        .width(Length::Fixed(220.0)),
                )
                .push(text(format!("CRF {}", adv.crf)))
                .push(
                    text("lower = better quality, larger file")
                        .size(12)
                        .style(text::secondary),
                );
        }
        QualityMode::Bitrate => {
            quality_row = quality_row
                .push(
                    text_input("4000", &adv.video_bitrate_kbps)
                        .on_input(|v| Message::Adv(AdvMessage::VideoBitrate(v)))
                        .width(Length::Fixed(100.0)),
                )
                .push(text("kbps"));
        }
    }

    let custom_size = (adv.resolution == Resolution::Custom).then(|| {
        row![
            text_input("width", &adv.custom_width)
                .on_input(|v| Message::Adv(AdvMessage::CustomWidth(v)))
                .width(Length::Fixed(90.0)),
            text("x"),
            text_input("height", &adv.custom_height)
                .on_input(|v| Message::Adv(AdvMessage::CustomHeight(v)))
                .width(Length::Fixed(90.0)),
            text("pixels").style(text::secondary),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    });

    let audio_row = row![
        pick_list(AudioCodec::ALL, Some(adv.audio_codec), |v| Message::Adv(
            AdvMessage::AudioCodec(v)
        )),
        text_input("bitrate kbps", &adv.audio_bitrate_kbps)
            .on_input(|v| Message::Adv(AdvMessage::AudioBitrate(v)))
            .width(Length::Fixed(110.0)),
        pick_list(SampleRate::ALL, Some(adv.sample_rate), |v| Message::Adv(
            AdvMessage::SampleRate(v)
        )),
        pick_list(Channels::ALL, Some(adv.channels), |v| Message::Adv(
            AdvMessage::Channels(v)
        )),
        checkbox(adv.strip_audio)
            .label("Strip audio")
            .on_toggle(|v| Message::Adv(AdvMessage::StripAudio(v))),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let extra_row = row![
        text("Extra ffmpeg args").width(Length::Fixed(140.0)),
        text_input("e.g. -metadata title=\"My clip\"", &adv.extra_args)
            .on_input(|v| Message::Adv(AdvMessage::ExtraArgs(v)))
            .font(Font::MONOSPACE)
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let preview = container(text(app.command_preview()).font(Font::MONOSPACE).size(12))
        .padding(10)
        .width(Length::Fill)
        .style(container::bordered_box);

    let warnings: Column<'_, Message> = app
        .build_command()
        .map(|cmd| cmd.warnings)
        .unwrap_or_default()
        .into_iter()
        .fold(column![].spacing(4), |col, w| {
            col.push(text(format!("Note: {w}")).size(13).style(text::warning))
        });

    let bottom_row = row![
        button("Copy command")
            .style(button::secondary)
            .on_press(Message::CopyCommand),
        button("Re-download ffmpeg")
            .style(button::secondary)
            .on_press(Message::RedownloadFfmpeg),
    ]
    .spacing(10);

    let mut panel = column![
        template_row,
        rule::horizontal(1),
        text("Video").size(16),
        codec_row,
        quality_row,
    ]
    .spacing(12);
    if let Some(custom_size) = custom_size {
        panel = panel.push(custom_size);
    }
    let panel = panel
        .push(rule::horizontal(1))
        .push(text("Audio").size(16))
        .push(audio_row)
        .push(rule::horizontal(1))
        .push(extra_row)
        .push(text("Command preview").size(16))
        .push(preview)
        .push(warnings)
        .push(bottom_row);

    column![
        toggle,
        container(panel)
            .padding(16)
            .width(Length::Fill)
            .style(container::rounded_box),
    ]
    .spacing(8)
    .into()
}
