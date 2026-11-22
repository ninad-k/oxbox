use super::{
    Shared,
    widgets::{self, Asset, Preview},
};
use crate::jobs::Task;
use eframe::egui;
use oxbox_core::{
    CoreError,
    tools::{
        clipboard_history::{Content, Snapshot},
        screenshot::RgbaImage,
    },
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn pause_pending(
    action: &mut Task<()>,
    pause: impl FnOnce() -> oxbox_core::CoreResult<()>,
) -> oxbox_core::CoreResult<()> {
    // An earlier Start may already be inside native setup. Wait for it to
    // relinquish ownership before applying the newer Pause/Clear intent.
    action.shutdown();
    pause()
}

pub struct ClipboardUi {
    consent: bool,
    query: String,
    view_active: bool,
    context: Option<egui::Context>,
    last: Instant,
    snapshot_job: Task<Snapshot>,
    snapshot: Option<Snapshot>,
    action: Task<()>,
    preview_job: Task<Asset>,
    preview: Preview,
    selected: Option<u64>,
    error: Option<String>,
    service: Option<Arc<oxbox_core::tools::clipboard_history::ClipboardService>>,
}
impl Default for ClipboardUi {
    fn default() -> Self {
        Self {
            consent: false,
            query: String::new(),
            view_active: false,
            context: None,
            last: Instant::now() - Duration::from_secs(1),
            snapshot_job: Task::default(),
            snapshot: None,
            action: Task::default(),
            preview_job: Task::default(),
            preview: Preview::default(),
            selected: None,
            error: None,
            service: None,
        }
    }
}
impl ClipboardUi {
    pub fn show(&mut self, ui: &mut egui::Ui, shared: &Shared) {
        self.view_active = true;
        self.context = Some(ui.ctx().clone());
        self.service = Some(shared.clipboard.clone());
        widgets::introduction(
            ui,
            "Keep copied text and images in a bounded, memory-only history. History starts paused and is erased on Quit.",
        );
        widgets::note(
            ui,
            "Maximum 200 items and 50 MiB, including pinned entries. Windows content marked as concealed is skipped where exposed by the OS. No history is saved to disk or synchronized.",
        );
        if ui
            .checkbox(
                &mut self.consent,
                "I consent to recording text and images copied on this computer",
            )
            .changed()
            && !self.consent
        {
            self.error = pause_pending(&mut self.action, || shared.clipboard.set_paused(true))
                .err()
                .map(|error| error.to_string());
            self.snapshot_job.abandon();
            self.snapshot = None;
            self.last = Instant::now() - Duration::from_secs(1);
        }
        let paused = self
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.paused);
        ui.horizontal(|ui|{
            if paused{
                if ui.add_enabled(self.consent&&!self.action.running(),egui::Button::new("Start clipboard history")).clicked(){let service=shared.clipboard.clone();self.action.start(ui.ctx(),move|cancel,_|{cancel.check()?;service.start()?;cancel.check()?;service.set_paused(false)});self.last=Instant::now()-Duration::from_secs(1);}
            }else if ui.button("Pause history").clicked(){self.error=pause_pending(&mut self.action,||shared.clipboard.set_paused(true)).err().map(|error|error.to_string());self.last=Instant::now()-Duration::from_secs(1);}
            if ui.button("Pause and clear history…").clicked() && widgets::confirm(ui.ctx(),"Pause and clear clipboard history?","Pause recording and remove all remembered entries, including pinned entries, from Oxbox memory? The system clipboard is unchanged."){
                self.snapshot_job.abandon();self.preview_job.abandon();
                self.error=pause_pending(&mut self.action,||shared.clipboard.set_paused(true).and_then(|()|shared.clipboard.clear())).err().map(|error|error.to_string());self.preview=Preview::default();self.selected=None;self.snapshot=None;self.last=Instant::now()-Duration::from_secs(1);
            }
        });
        let search_label = ui.label("Search history");
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search remembered text")
                    .char_limit(4096)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(search_label.id)
            .changed()
        {
            self.snapshot_job.abandon();
            self.snapshot = None;
            self.last = Instant::now() - Duration::from_secs(1);
        }
        if let Some(snapshot) = &self.snapshot {
            ui.label(format!(
                "{} · {} items · {:.1} MiB",
                if snapshot.paused {
                    "Paused"
                } else {
                    "Recording copied content"
                },
                snapshot.total_items,
                snapshot.bytes as f64 / 1048576.0
            ));
            if let Some(error) = &snapshot.error {
                widgets::note(ui, error);
            }
            if snapshot.items.is_empty() {
                widgets::note(
                    ui,
                    if self.query.is_empty() {
                        "Nothing has been remembered yet. Start history explicitly, then copy text or an image."
                    } else {
                        "No history entries match this search."
                    },
                );
            }
            egui::ScrollArea::vertical().max_height(350.0).show(ui,|ui|{for item in &snapshot.items{
                ui.push_id(item.id, |ui| {
                let context = format!("{} item {}", item.kind, item.id);
                ui.group(|ui|{
                    ui.horizontal(|ui|{ui.strong(&context);ui.small(format!("{} bytes{}",item.bytes,if item.pinned{" · Pinned"}else{""}));});
                    ui.label(&item.preview);
                    ui.horizontal(|ui|{
                        if crate::accessibility::enabled_button(ui,!self.action.running(),"Copy back",&format!("Copy {context} back")).clicked(){let service=shared.clipboard.clone();let id=item.id;self.action.start(ui.ctx(),move|cancel,_|{cancel.check()?;service.copy(id)});}
                        if crate::accessibility::button(ui,if item.pinned{"Unpin"}else{"Pin"},&format!("{} {context}",if item.pinned{"Unpin"}else{"Pin"})).clicked(){self.error=shared.clipboard.pin(item.id,!item.pinned).err().map(|error|error.to_string());self.last=Instant::now()-Duration::from_secs(1);}
                        if item.kind=="image" && crate::accessibility::enabled_button(ui,!self.preview_job.running(),"Preview image",&format!("Preview {context}")).clicked(){
                            self.selected=Some(item.id);let service=shared.clipboard.clone();let id=item.id;
                            self.preview_job.start(ui.ctx(),move|cancel,_|{cancel.check()?;let Content::Image{width,height,rgba}=service.content(id)? else{return Err(CoreError::InvalidInput("this entry is not an image".into()))};
                                let width=u32::try_from(width).map_err(|_|CoreError::InvalidInput("image width is invalid".into()))?;let height=u32::try_from(height).map_err(|_|CoreError::InvalidInput("image height is invalid".into()))?;
                                let image=RgbaImage::from_raw(width,height,rgba).ok_or_else(||CoreError::InvalidInput("image data is invalid".into()))?;Ok(Asset::new(image))});
                        }
                        if crate::accessibility::button(ui,"Remove…",&format!("Remove {context}")).clicked() && widgets::confirm(ui.ctx(),"Remove remembered item?","Remove this item from Oxbox's in-memory history? The system clipboard is unchanged."){
                            self.error=shared.clipboard.delete(item.id).err().map(|error|error.to_string());if self.selected==Some(item.id){self.preview=Preview::default();self.selected=None;self.preview_job.cancel();}self.last=Instant::now()-Duration::from_secs(1);
                        }
                    });
                });
                });
            }});
        }
        self.snapshot_job.show_quiet(ui);
        self.action.show(ui);
        self.preview_job.show(ui);
        widgets::error(ui, &self.error);
        self.preview.show(ui);
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
    pub fn tick(&mut self) {
        self.snapshot_job.poll();
        self.action.poll();
        self.preview_job.poll();
        if let Some(snapshot) = self.snapshot_job.result.take() {
            self.snapshot = Some(snapshot);
        }
        if let Some(asset) = self.preview_job.result.take()
            && let Some(ctx) = &self.context
            && self.selected.is_some()
        {
            self.preview.set(ctx, asset);
        }
        if self.view_active
            && !self.snapshot_job.running()
            && self.last.elapsed() >= Duration::from_millis(500)
            && let (Some(ctx), Some(service)) = (&self.context, &self.service)
        {
            let service = service.clone();
            let query = self.query.clone();
            self.snapshot_job.start(ctx, move |cancel, _| {
                cancel.check()?;
                service.snapshot(&query)
            });
            self.last = Instant::now();
        }
        self.view_active = false;
    }
    pub fn shutdown(&mut self) {
        self.snapshot_job.shutdown();
        self.action.shutdown();
        self.preview_job.shutdown();
        self.preview = Preview::default();
        self.snapshot = None;
        self.query.clear();
        self.consent = false;
    }
    pub fn leave(&mut self) {
        self.snapshot_job.abandon();
        self.action.abandon();
        self.preview_job.abandon();
        self.preview = Preview::default();
        self.snapshot = None;
        self.selected = None;
        self.query.clear();
        self.view_active = false;
        // Consent and the explicitly started background watcher survive view
        // navigation; its permanent shutdown belongs only to application Quit.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    };

    #[test]
    fn community_search_edit_discards_old_filter_rows_and_queued_snapshot() {
        let old = Snapshot {
            paused: true,
            bytes: 8,
            total_items: 1,
            error: None,
            items: vec![oxbox_core::tools::clipboard_history::EntrySummary {
                id: 1,
                pinned: false,
                captured_at: 0,
                kind: "text",
                preview: "Generated old-query match".into(),
                bytes: 8,
            }],
        };
        let mut view = ClipboardUi {
            query: "old search".into(),
            snapshot: Some(old.clone()),
            ..Default::default()
        };
        view.snapshot_job.result = Some(old);
        let shared = Shared::default();
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut first = context.run_ui(egui::RawInput::default(), |ui| view.show(ui, &shared));
        first.textures_delta.clear();
        let field = first
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::TextInput
                    && node.value() == Some("old search"))
                .then_some(id)
            })
            .unwrap();
        let mut second = context.run_ui(
            egui::RawInput {
                events: vec![egui::Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::SetValue,
                        target_node: field,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        data: Some(egui::accesskit::ActionData::Value("new search".into())),
                    },
                )],
                ..Default::default()
            },
            |ui| view.show(ui, &shared),
        );
        second.textures_delta.clear();
        assert_eq!(view.query, "new search");
        assert!(
            view.snapshot.is_none(),
            "new filter kept rows for the old query"
        );
        assert!(
            view.snapshot_job.result.is_none(),
            "old query completion can reappear"
        );
        assert!(shared.clipboard.snapshot("").unwrap().paused);
        view.shutdown();
    }

    #[test]
    fn generated_clipboard_rows_have_distinct_named_actions_without_observing_clipboard() {
        use oxbox_core::tools::clipboard_history::EntrySummary;
        let mut view = ClipboardUi {
            snapshot: Some(Snapshot {
                paused: true,
                bytes: 16,
                total_items: 2,
                error: None,
                items: [(1, "text"), (2, "image")]
                    .into_iter()
                    .map(|(id, kind)| EntrySummary {
                        id,
                        kind,
                        pinned: false,
                        captured_at: 0,
                        preview: format!("Generated {kind} fixture"),
                        bytes: 8,
                    })
                    .collect(),
            }),
            ..Default::default()
        };
        let shared = Shared::default();
        let nodes = crate::accessibility::test_tree(|ui| view.show(ui, &shared));
        let labels = nodes
            .iter()
            .filter_map(|node| node.label())
            .collect::<Vec<_>>();
        for label in [
            "Copy text item 1 back",
            "Remove text item 1",
            "Copy image item 2 back",
            "Remove image item 2",
        ] {
            assert!(
                labels.contains(&label),
                "missing clipboard action name: {label}"
            );
        }
        assert!(shared.clipboard.snapshot("").unwrap().paused);
        view.shutdown();
    }

    #[test]
    fn pause_joins_an_older_start_before_applying_the_final_paused_state() {
        let ctx = egui::Context::default();
        let mut action = Task::default();
        let recording = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let worker_recording = recording.clone();
        let worker_paused = paused.clone();
        let (entered, started) = mpsc::sync_channel(1);
        assert!(action.start(&ctx, move |cancel, _| {
            cancel.check()?;
            entered.send(()).unwrap();
            // Fake a delayed native Start already past its initial cancel check.
            let deadline = Instant::now() + Duration::from_secs(5);
            while cancel.check().is_ok() && !worker_paused.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline, "fake Start was not released");
                std::thread::sleep(Duration::from_millis(1));
            }
            worker_recording.store(true, Ordering::Release);
            Ok(())
        }));
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        pause_pending(&mut action, || {
            recording.store(false, Ordering::Release);
            paused.store(true, Ordering::Release);
            Ok(())
        })
        .unwrap();
        let pending_after_pause = action.running();
        action.shutdown();
        assert!(!pending_after_pause, "an earlier Start outlived Pause");
        assert!(
            !recording.load(Ordering::Acquire),
            "history resumed after Pause"
        );
    }
}
