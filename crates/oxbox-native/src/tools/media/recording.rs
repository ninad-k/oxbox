use super::{
    Shared,
    widgets::{self, Asset, Preview, Providers},
};
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::{
    platform::Os,
    tools::{
        media_provider::RecordingStatus,
        mute_microphone, record_audio,
        record_screen::{self, ScreenRequest, ScreenSource},
        screenshot, take_photo, take_video,
    },
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct RecordingUi {
    providers: Providers,
    audio_device: String,
    camera_device: String,
    output_audio: String,
    output_video: String,
    output_screen: String,
    output_photo: String,
    seconds: u32,
    fps: u32,
    source: usize,
    title: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    consent: bool,
    windows: Task<Vec<screenshot::TargetInfo>>,
    window_names: Vec<String>,
    start: Task<RecordingStatus>,
    stop: Task<RecordingStatus>,
    photo: Task<(PathBuf, Asset)>,
    photo_result: Option<PathBuf>,
    preview: Preview,
    microphone: Task<bool>,
    muted: Option<bool>,
    status: RecordingStatus,
    announced_id: String,
    last_poll: Instant,
}
impl Default for RecordingUi {
    fn default() -> Self {
        Self {
            providers: Providers::default(),
            audio_device: if cfg!(target_os = "linux") {
                "default".into()
            } else if cfg!(target_os = "macos") {
                "0".into()
            } else {
                String::new()
            },
            camera_device: if cfg!(target_os = "linux") {
                "/dev/video0".into()
            } else if cfg!(target_os = "macos") {
                "0".into()
            } else {
                String::new()
            },
            output_audio: String::new(),
            output_video: String::new(),
            output_screen: String::new(),
            output_photo: String::new(),
            seconds: 300,
            fps: 15,
            source: 0,
            title: String::new(),
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
            consent: false,
            windows: Task::default(),
            window_names: Vec::new(),
            start: Task::default(),
            stop: Task::default(),
            photo: Task::default(),
            photo_result: None,
            preview: Preview::default(),
            microphone: Task::default(),
            muted: None,
            status: RecordingStatus::default(),
            announced_id: String::new(),
            last_poll: Instant::now(),
        }
    }
}
impl RecordingUi {
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, _host: &mut Host, shared: &Shared) {
        if id == "mute-microphone" {
            self.microphone(ui);
            return;
        }
        let photo = id == "take-photo";
        let screen = id == "record-screen";
        let audio = id == "record-audio";
        widgets::introduction(
            ui,
            if photo {
                "Take one photo from a camera you choose. The camera is opened only after confirmation."
            } else {
                "Record only after an explicit start. A visible recording banner and Stop control remain available across tools."
            },
        );
        self.providers.show(ui);
        if self.status.active {
            self.banner(ui, shared);
        }
        let disabled = self.status.active || self.start.running() || self.photo.running();
        ui.add_enabled_ui(!disabled,|ui|{
            if !screen{
                widgets::text(ui,if audio{"Microphone device"}else{"Camera device"},if audio{&mut self.audio_device}else{&mut self.camera_device});
                widgets::note(ui,match Os::current(){Os::Windows=>"Enter the exact FFmpeg DirectShow device name. To list names yourself: ffmpeg -list_devices true -f dshow -i dummy",Os::Macos=>"Enter a local AVFoundation device index. List devices yourself with ffmpeg -f avfoundation -list_devices true -i \"\".",_=>if audio{"Use the local PulseAudio/PipeWire input name, or default."}else{"Use a local V4L2 path, for example /dev/video0."}});
            }else{
                widgets::choice(ui,"Capture source",&mut self.source,&["Desktop","Area · physical pixels","Window · Windows"]);
                if self.source==1{ui.horizontal(|ui|{let label=ui.label("Left (physical X)");ui.add(egui::DragValue::new(&mut self.x)).labelled_by(label.id);let label=ui.label("Top (physical Y)");ui.add(egui::DragValue::new(&mut self.y)).labelled_by(label.id);});widgets::number(ui,"Width (pixels)",&mut self.width,1..=16384);widgets::number(ui,"Height (pixels)",&mut self.height,1..=16384);}
                if self.source==2{
                    widgets::text(ui,"Exact window title",&mut self.title);
                    if ui.button("Find open windows").clicked(){self.windows.start(ui.ctx(),|cancel,_|{cancel.check()?;screenshot::targets()});}
                    egui::ComboBox::from_label("Listed recording window").selected_text("Choose a listed window…").show_ui(ui,|ui|{for name in &self.window_names{if ui.selectable_label(self.title==*name,name).clicked(){self.title=name.clone();}}});
                    self.windows.show(ui);
                }
                widgets::number(ui,"Frames per second",&mut self.fps,1..=60);
                if cfg!(target_os="macos"){widgets::note(ui,"Screen recording is not available through this provider on macOS. Camera and audio recording remain separate tools.");}
                if cfg!(target_os="linux"){widgets::note(ui,"Screen recording requires a local X11 session. Wayland and Linux window capture are not available through this provider.");}
            }
            if !photo{widgets::number(ui,"Maximum duration (seconds)",&mut self.seconds,1..=86400);}
            let output=if photo{&mut self.output_photo}else if audio{&mut self.output_audio}else if screen{&mut self.output_screen}else{&mut self.output_video};
            if widgets::output(ui,"New output file",output,if photo{"Oxbox-photo.png"}else if audio{"Oxbox-recording.wav"}else{"Oxbox-recording.mp4"}) && photo { self.photo.abandon(); self.photo_result=None; self.preview=Preview::default(); }
            ui.checkbox(&mut self.consent,if audio{"I consent to recording this microphone"}else if screen{"I consent to recording this screen/window/area"}else{"I consent to opening this camera"});
            let available=!output.trim().is_empty()&&self.consent&&(!screen||!cfg!(target_os="macos"))&&(!screen||self.source!=2||cfg!(target_os="windows"));
            if ui.add_enabled(available,egui::Button::new(if photo{"Take photo"}else{"Start recording"})).clicked(){
                let ticket=shared.recording_dispatch.reserve_start();
                let description=format!("{}\nDevice/source: {}\nNew output: {}\n{}",if photo{"Capture one camera frame?"}else{"Start a live recording?"},if screen{"the selected screen source"}else if audio{self.audio_device.as_str()}else{self.camera_device.as_str()},output,if photo{String::new()}else{format!("Maximum duration: {} seconds. Stop finalizes output; Quit stops recording.",self.seconds)});
                if widgets::confirm(ui.ctx(), if photo{"Camera capture"}else{"Start recording"},&description){
                    let path=PathBuf::from(output.as_str());
                    if photo{
                        let device=self.camera_device.clone();let quit=shared.quit.clone();
                        self.photo.start(ui.ctx(),move|cancel,progress|{quit.check()?;let path=take_photo::capture(&device,&path,&cancel,progress)?;cancel.check()?;let asset=Asset::load(&path)?;Ok((path,asset))});
                    }else{
                        let seconds=u64::from(self.seconds);let device=if audio{self.audio_device.clone()}else{self.camera_device.clone()};
                        let request=ScreenRequest{source:match self.source{1=>ScreenSource::Area{x:self.x,y:self.y,width:self.width,height:self.height},2=>ScreenSource::Window{title:self.title.clone()},_=>ScreenSource::Desktop},fps:self.fps,seconds};
                        let recorder=shared.recorder.clone();let dispatch=shared.recording_dispatch.clone();let quit=shared.quit.clone();
                        self.start.start(ui.ctx(),move|cancel,_|{
                            dispatch.start(ticket,&cancel,&quit,||{
                            let args=if screen{record_screen::plan(Os::current(),&request)?}else if audio{record_audio::plan(Os::current(),&device,seconds)?}else{take_video::plan(Os::current(),&device,seconds)?};
                            cancel.check()?;recorder.start(if screen{"screen"}else if audio{"microphone"}else{"camera"},args,&path,if audio{"wav"}else{"mp4"})})
                        });
                    }
                }
            }
        });
        self.start.poll();
        self.stop.poll();
        if self.start.running() {
            ui.spinner();
            ui.label("Starting recording…");
        }
        if self.stop.running() {
            ui.spinner();
            ui.label("Stopping and saving recording…");
        }
        widgets::error(ui, &self.start.error);
        widgets::error(ui, &self.stop.error);
        self.photo.show(ui);
        if self.start.running() && ui.button("Cancel pending start").clicked() {
            self.stop(ui.ctx(), shared);
        }
        if let Some(error) = &self.status.error {
            widgets::note(ui, error);
        }
        if !self.status.active
            && let Some(path) = &self.status.output
        {
            widgets::note(ui, format!("Recording output: {}", path.display()));
        }
        if photo {
            if let Some(path) = &self.photo_result {
                widgets::note(ui, format!("Photo saved to {}", path.display()));
            }
            self.preview.show(ui);
        }
        widgets::note(
            ui,
            "Only one recording can run at a time. Maximum 24 hours or 20 GiB; output files are created exclusively. Screen/camera recording does not include microphone audio. Originals and recovery recordings are preserved.",
        );
    }
    fn microphone(&mut self, ui: &mut egui::Ui) {
        widgets::introduction(
            ui,
            "Inspect or change the default microphone's operating-system mute state. This affects other applications using that input device.",
        );
        if ui
            .add_enabled(
                !self.microphone.running(),
                egui::Button::new("Refresh microphone state"),
            )
            .clicked()
        {
            self.microphone.start(ui.ctx(), |cancel, _| {
                cancel.check()?;
                mute_microphone::status_with_cancel(&cancel)
            });
        }
        if let Some(muted) = self.muted {
            widgets::note(
                ui,
                if muted {
                    "Default microphone is muted"
                } else {
                    "Default microphone is unmuted"
                },
            );
            if ui
                .add_enabled(
                    !self.microphone.running(),
                    egui::Button::new(if muted {
                        "Unmute microphone"
                    } else {
                        "Mute microphone"
                    }),
                )
                .clicked()
                && (!muted
                    || widgets::confirm(
                        ui.ctx(),
                        "Unmute microphone?",
                        "Allow applications to receive audio from the default microphone again?",
                    ))
            {
                self.microphone.start(ui.ctx(), move |cancel, _| {
                    cancel.check()?;
                    mute_microphone::set_muted_with_cancel(!muted, &cancel)
                });
            }
        }
        self.microphone.show(ui);
        if cfg!(target_os = "macos") {
            widgets::note(
                ui,
                "Default microphone mute is not available through this backend on macOS.",
            );
        }
    }
    pub fn stop(&mut self, ctx: &egui::Context, shared: &Shared) {
        self.start.cancel();
        shared.recording_dispatch.invalidate_pending();
        let recorder = shared.recorder.clone();
        let dispatch = shared.recording_dispatch.clone();
        self.stop.start(ctx, move |_, _| {
            dispatch.stop(|| {
                let status = recorder.status()?;
                if status.active {
                    recorder.stop(&status.id)
                } else {
                    Ok(status)
                }
            })
        });
    }
    pub fn banner(&mut self, ui: &mut egui::Ui, shared: &Shared) {
        if self.status.active {
            ui.group(|ui| {
                ui.strong(format!("● RECORDING ACTIVE · {}", self.status.kind));
                if let Some(path) = &self.status.output {
                    ui.label(path.display().to_string());
                }
                if ui
                    .add_enabled(
                        !self.stop.running(),
                        egui::Button::new("Stop and save recording"),
                    )
                    .clicked()
                {
                    self.stop(ui.ctx(), shared);
                }
            });
        }
    }
    pub fn tick(&mut self, ctx: Option<&egui::Context>, host: &mut Host, shared: &Shared) {
        self.providers.tick();
        self.start.poll();
        self.stop.poll();
        self.photo.poll();
        self.microphone.poll();
        self.windows.poll();
        if let Some(value) = self.microphone.result.take() {
            self.muted = Some(value);
        }
        if let Some(targets) = self.windows.result.take() {
            self.window_names = targets
                .into_iter()
                .filter(|target| target.kind == screenshot::CaptureKind::Window)
                .map(|target| target.name)
                .collect();
        }
        if let Some((path, asset)) = self.photo.result.take() {
            self.photo_result = Some(path);
            if let Some(ctx) = ctx {
                self.preview.set(ctx, asset);
            }
        }
        if self.last_poll.elapsed() >= Duration::from_millis(250) {
            self.last_poll = Instant::now();
            if let Ok(status) = shared.recorder.status() {
                if !status.active && !status.id.is_empty() && self.announced_id != status.id {
                    host.notifier.notify(
                        "Recording finished",
                        status
                            .error
                            .as_deref()
                            .unwrap_or("The local recording has finished."),
                    );
                    self.announced_id = status.id.clone();
                }
                self.status = status;
            }
        }
        if (self.status.active || self.start.running() || self.stop.running())
            && let Some(ctx) = ctx
        {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }
    pub fn shutdown(&mut self) {
        self.start.cancel();
        self.stop.cancel();
        self.photo.cancel();
        self.microphone.cancel();
        self.windows.cancel();
        self.providers.cancel();
        self.start.shutdown();
        self.stop.shutdown();
        self.photo.shutdown();
        self.microphone.shutdown();
        self.windows.shutdown();
        self.providers.shutdown();
    }
    pub fn leave(&mut self, id: &str) {
        self.providers.leave();
        self.windows.abandon();
        if id == "take-photo" {
            self.photo.abandon();
            self.preview = Preview::default();
            self.photo_result = None;
        }
        if id == "mute-microphone" {
            self.microphone.abandon();
            self.muted = None;
        }
        // Confirmed microphone/camera/screen recordings and their pending Start
        // and Stop workers stay owned across navigation and launcher hiding.
    }
}
