//! Application state and update logic.

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::task::Handle;
use iced::{Event, Subscription, Task, event, window};

use crate::ffmpeg::args::{
    self, AdvancedOptions, AudioCodec, BuiltCommand, Channels, Fps, Preset, QualityMode,
    Resolution, SampleRate, VideoCodec,
};
use crate::ffmpeg::probe::{self, ProbeInfo};
use crate::ffmpeg::run::{self, ConvertEvent};
use crate::ffmpeg::{self, FfmpegEvent, FfmpegPaths};
use crate::formats::{self, Kind, OutputFormat};
use crate::paths;
use crate::templates::Template;
use crate::view;

pub struct InputFile {
    pub path: PathBuf,
    pub probe: ProbeInfo,
}

pub enum FfmpegState {
    Checking,
    Downloading { done: u64, total: Option<u64> },
    Extracting,
    Ready(FfmpegPaths),
    Failed(String),
}

pub enum JobState {
    Idle,
    Probing,
    Converting { percent: Option<f32>, speed: String },
    Done(PathBuf),
    Failed(String),
}

pub struct App {
    pub ffmpeg: FfmpegState,
    pub input: Option<InputFile>,
    pub pending_input: Option<PathBuf>,
    pub audio_only: bool,
    pub format: OutputFormat,
    pub output: String,
    pub output_edited: bool,
    pub advanced_open: bool,
    pub adv: AdvancedOptions,
    pub template: Option<Template>,
    pub job: JobState,
    pub wayland: bool,
    convert_handle: Option<Handle>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ffmpeg(FfmpegEvent),
    RedownloadFfmpeg,
    PickFile,
    FilePicked(Option<PathBuf>),
    FileDropped(PathBuf),
    Probed(PathBuf, Result<ProbeInfo, String>),
    AudioOnlyToggled(bool),
    FormatChosen(OutputFormat),
    OutputEdited(String),
    SaveAs,
    OutputPicked(Option<PathBuf>),
    ToggleAdvanced,
    Adv(AdvMessage),
    TemplateChosen(Template),
    Convert,
    ConvertEvent(ConvertEvent),
    Cancel,
    Cancelled,
    Reveal,
    CopyCommand,
}

#[derive(Debug, Clone)]
pub enum AdvMessage {
    VideoCodec(VideoCodec),
    QualityMode(QualityMode),
    Crf(u8),
    VideoBitrate(String),
    Resolution(Resolution),
    CustomWidth(String),
    CustomHeight(String),
    Fps(Fps),
    Preset(Preset),
    AudioCodec(AudioCodec),
    AudioBitrate(String),
    SampleRate(SampleRate),
    Channels(Channels),
    StripAudio(bool),
    ExtraArgs(String),
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let app = Self {
            ffmpeg: FfmpegState::Checking,
            input: None,
            pending_input: None,
            audio_only: false,
            format: formats::MP4,
            output: String::new(),
            output_edited: false,
            advanced_open: false,
            adv: AdvancedOptions::default(),
            template: None,
            job: JobState::Idle,
            wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
            convert_handle: None,
        };
        (app, Task::run(ffmpeg::ensure(false), Message::Ffmpeg))
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        view::view(self)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(map_event)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ffmpeg(event) => {
                self.ffmpeg = match event {
                    FfmpegEvent::Checking => FfmpegState::Checking,
                    FfmpegEvent::Downloading { done, total } => {
                        FfmpegState::Downloading { done, total }
                    }
                    FfmpegEvent::Extracting => FfmpegState::Extracting,
                    FfmpegEvent::Ready(paths) => FfmpegState::Ready(paths),
                    FfmpegEvent::Failed(e) => FfmpegState::Failed(e),
                };
                if matches!(self.ffmpeg, FfmpegState::Ready(_)) {
                    if let Some(path) = self.pending_input.take() {
                        return self.load_input(path);
                    }
                }
                Task::none()
            }
            Message::RedownloadFfmpeg => {
                self.ffmpeg = FfmpegState::Checking;
                Task::run(ffmpeg::ensure(true), Message::Ffmpeg)
            }
            Message::PickFile => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Choose a file to convert")
                        .pick_file()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::FilePicked,
            ),
            Message::FilePicked(Some(path)) | Message::FileDropped(path) => self.load_input(path),
            Message::FilePicked(None) => Task::none(),
            Message::Probed(path, Ok(info)) => {
                if !info.has_video && info.has_audio {
                    self.audio_only = true;
                } else if info.is_image {
                    self.audio_only = false;
                }
                if !formats::fits(self.format, &info) {
                    self.format = formats::default_for(&info);
                }
                self.input = Some(InputFile { path, probe: info });
                self.job = JobState::Idle;
                self.output_edited = false;
                self.refresh_output();
                Task::none()
            }
            Message::Probed(_, Err(e)) => {
                self.input = None;
                self.job = JobState::Failed(format!("Could not read this file: {e}"));
                Task::none()
            }
            Message::AudioOnlyToggled(enabled) => {
                self.audio_only = enabled;
                if enabled && self.format.kind != Kind::Audio {
                    self.format = formats::MP3;
                } else if !enabled && self.format.kind == Kind::Audio {
                    self.format = formats::MP4;
                }
                self.refresh_output();
                Task::none()
            }
            Message::FormatChosen(format) => {
                self.format = format;
                match format.kind {
                    Kind::Audio => self.audio_only = true,
                    Kind::Video => self.audio_only = false,
                    Kind::Image => {}
                }
                self.refresh_output();
                Task::none()
            }
            Message::OutputEdited(text) => {
                self.output = text;
                self.output_edited = true;
                Task::none()
            }
            Message::SaveAs => {
                let current = self.resolved_output();
                let ext = self.format.ext;
                let label = self.format.label.to_string();
                Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new()
                            .set_title("Save converted file as")
                            .add_filter(label, &[ext]);
                        if let Some(current) = &current {
                            if let Some(dir) = current.parent() {
                                dialog = dialog.set_directory(dir);
                            }
                            if let Some(name) = current.file_name() {
                                dialog = dialog.set_file_name(name.to_string_lossy());
                            }
                        }
                        dialog.save_file().await.map(|h| h.path().to_path_buf())
                    },
                    Message::OutputPicked,
                )
            }
            Message::OutputPicked(Some(mut path)) => {
                match path
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(formats::by_ext)
                {
                    // The user typed a known extension: follow it.
                    Some(format) => {
                        self.format = format;
                        match format.kind {
                            Kind::Audio => self.audio_only = true,
                            Kind::Video => self.audio_only = false,
                            Kind::Image => {}
                        }
                    }
                    None => {
                        path.set_extension(self.format.ext);
                    }
                }
                self.output = path.display().to_string();
                self.output_edited = true;
                Task::none()
            }
            Message::OutputPicked(None) => Task::none(),
            Message::ToggleAdvanced => {
                self.advanced_open = !self.advanced_open;
                Task::none()
            }
            Message::Adv(change) => {
                self.apply_advanced(change);
                self.template = None;
                Task::none()
            }
            Message::TemplateChosen(template) => {
                let (format, audio_only, adv) = template.apply();
                self.format = format;
                self.audio_only = audio_only;
                self.adv = adv;
                self.template = Some(template);
                self.refresh_output();
                Task::none()
            }
            Message::Convert => self.start_conversion(),
            Message::ConvertEvent(event) => {
                match event {
                    ConvertEvent::Progress { percent, speed } => {
                        self.job = JobState::Converting { percent, speed };
                    }
                    ConvertEvent::Done(path) => {
                        self.convert_handle = None;
                        self.job = JobState::Done(path);
                    }
                    ConvertEvent::Failed(e) => {
                        self.convert_handle = None;
                        self.job = JobState::Failed(e);
                    }
                }
                Task::none()
            }
            Message::Cancel => {
                if let Some(handle) = self.convert_handle.take() {
                    handle.abort();
                }
                self.job = JobState::Idle;
                let partial = self.resolved_output();
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        if let Some(path) = partial {
                            let _ = tokio::fs::remove_file(path).await;
                        }
                    },
                    |()| Message::Cancelled,
                )
            }
            Message::Cancelled => Task::none(),
            Message::Reveal => {
                if let JobState::Done(path) = &self.job {
                    if opener::reveal(path).is_err() {
                        if let Some(dir) = path.parent() {
                            let _ = opener::open(dir);
                        }
                    }
                }
                Task::none()
            }
            Message::CopyCommand => iced::clipboard::write(self.command_preview()),
        }
    }

    fn load_input(&mut self, path: PathBuf) -> Task<Message> {
        let FfmpegState::Ready(paths) = &self.ffmpeg else {
            self.pending_input = Some(path);
            return Task::none();
        };
        if matches!(self.job, JobState::Converting { .. }) {
            return Task::none();
        }
        let ffprobe = paths.ffprobe.clone();
        self.job = JobState::Probing;
        Task::perform(
            async move {
                let result = probe::run(ffprobe, path.clone())
                    .await
                    .map_err(|e| format!("{e:#}"));
                (path, result)
            },
            |(path, result)| Message::Probed(path, result),
        )
    }

    fn apply_advanced(&mut self, change: AdvMessage) {
        let adv = &mut self.adv;
        match change {
            AdvMessage::VideoCodec(v) => adv.video_codec = v,
            AdvMessage::QualityMode(v) => adv.quality_mode = v,
            AdvMessage::Crf(v) => adv.crf = v,
            AdvMessage::VideoBitrate(v) => adv.video_bitrate_kbps = v,
            AdvMessage::Resolution(v) => adv.resolution = v,
            AdvMessage::CustomWidth(v) => adv.custom_width = v,
            AdvMessage::CustomHeight(v) => adv.custom_height = v,
            AdvMessage::Fps(v) => adv.fps = v,
            AdvMessage::Preset(v) => adv.preset = v,
            AdvMessage::AudioCodec(v) => adv.audio_codec = v,
            AdvMessage::AudioBitrate(v) => adv.audio_bitrate_kbps = v,
            AdvMessage::SampleRate(v) => adv.sample_rate = v,
            AdvMessage::Channels(v) => adv.channels = v,
            AdvMessage::StripAudio(v) => adv.strip_audio = v,
            AdvMessage::ExtraArgs(v) => adv.extra_args = v,
        }
    }

    fn refresh_output(&mut self) {
        if self.output_edited {
            return;
        }
        if let Some(input) = &self.input {
            self.output = paths::default_output(&input.path, self.format.ext)
                .display()
                .to_string();
        }
    }

    fn start_conversion(&mut self) -> Task<Message> {
        let (Some(input), FfmpegState::Ready(paths)) = (&self.input, &self.ffmpeg) else {
            return Task::none();
        };
        let Some(output) = self.resolved_output() else {
            self.job = JobState::Failed("Choose where to save the output first.".to_string());
            return Task::none();
        };
        if output == input.path {
            self.job =
                JobState::Failed("The output must be a different file than the input.".to_string());
            return Task::none();
        }
        if !output.parent().is_some_and(Path::exists) {
            self.job = JobState::Failed("The output folder does not exist.".to_string());
            return Task::none();
        }
        let Some(command) = self.build_command() else {
            return Task::none();
        };
        let job = run::Job {
            ffmpeg: paths.ffmpeg.clone(),
            args: command.args,
            output,
            duration_secs: input.probe.duration_secs,
        };
        let has_duration = job.duration_secs.is_some();
        let (task, handle) = Task::run(run::convert(job), Message::ConvertEvent).abortable();
        self.convert_handle = Some(handle);
        self.job = JobState::Converting {
            percent: has_duration.then_some(0.0),
            speed: String::new(),
        };
        task
    }

    /// The output path as typed, made absolute relative to the input's folder.
    pub fn resolved_output(&self) -> Option<PathBuf> {
        let text = self.output.trim();
        if text.is_empty() {
            return None;
        }
        let path = PathBuf::from(text);
        if path.is_absolute() {
            return Some(path);
        }
        let base = self
            .input
            .as_ref()
            .and_then(|i| i.path.parent())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())?;
        Some(base.join(path))
    }

    pub fn effective_audio_only(&self) -> bool {
        self.audio_only || self.format.kind == Kind::Audio
    }

    pub fn can_convert(&self) -> bool {
        matches!(self.ffmpeg, FfmpegState::Ready(_))
            && self.input.is_some()
            && !self.output.trim().is_empty()
            && !matches!(self.job, JobState::Converting { .. } | JobState::Probing)
    }

    pub fn build_command(&self) -> Option<BuiltCommand> {
        let input = self.input.as_ref()?;
        let output = self.resolved_output()?;
        Some(args::build(&args::Request {
            input: &input.path,
            output: &output,
            probe: &input.probe,
            format: self.format,
            audio_only: self.audio_only,
            adv: &self.adv,
        }))
    }

    pub fn command_preview(&self) -> String {
        match self.build_command() {
            Some(cmd) => args::preview("ffmpeg", &cmd.args),
            None => "Choose an input file to see the ffmpeg command.".to_string(),
        }
    }
}

fn map_event(event: Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Window(window::Event::FileDropped(path)) => Some(Message::FileDropped(path)),
        _ => None,
    }
}
