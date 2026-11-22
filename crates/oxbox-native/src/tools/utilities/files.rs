use super::{bytes, field};
use crate::jobs::Task;
use eframe::egui::{self, Ui};
use oxbox_core::tools::{
    archive::{self, ArchiveFormat},
    checksum::{self, Algorithm},
    clean_drive::{self, CleanCategory},
    duplicates::{self, DuplicateResult},
    encrypt,
};
use std::{collections::HashSet, path::PathBuf};

fn confirm(ctx: &egui::Context, title: &str, body: impl Into<String>) -> bool {
    crate::host::dialogs::message(ctx)
        .set_title(title)
        .set_description(body)
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}
fn picker(ui: &mut Ui, label: &str, value: &mut String, kind: u8) -> Option<PathBuf> {
    ui.push_id(label, |ui| {
        field(ui, label, value);
        if ui.button(format!("Choose {label}…")).clicked() {
            let dialog = crate::host::dialogs::file(ui.ctx()).set_title(label);
            match kind {
                1 => dialog.pick_folder(),
                2 => dialog.save_file(),
                _ => dialog.pick_file(),
            }
        } else {
            None
        }
    })
    .inner
}
fn dropped(ui: &Ui) -> Vec<PathBuf> {
    // Disabling widgets does not stop closure code from reading RawInput. Ignore
    // drops here too, so an active job keeps the exact inputs it was started with.
    if !ui.is_enabled() {
        return Vec::new();
    }
    ui.ctx().input(|input| {
        input
            .raw
            .dropped_files
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect()
    })
}

fn password_field(ui: &mut Ui, label: &str, value: &mut String) -> egui::Response {
    let label_response = ui.label(label);
    let mut output = egui::TextEdit::singleline(value)
        .id_salt(("password", label))
        .password(true)
        .interactive(ui.is_enabled())
        .char_limit(4096)
        .show(ui);
    // egui's password mode masks display/copy, but its ordinary undoer stores
    // plaintext snapshots. Never retain that history across password frames.
    output.state.clear_undoer();
    output.state.store(ui.ctx(), output.response.id);
    output.response.response.labelled_by(label_response.id)
}

fn list(ui: &mut Ui, paths: &mut Vec<PathBuf>) {
    let mut remove = None;
    for (index, path) in paths.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(path.display().to_string());
            let remove_button = ui.button("Remove");
            crate::accessibility::name(
                &remove_button,
                &format!("Remove selected path {}", path.display()),
            );
            if remove_button.clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        paths.remove(index);
    }
}
fn cache_path_display(path: &std::path::Path) -> String {
    match path.to_str() {
        Some(text) if !text.chars().any(char::is_control) => text.to_owned(),
        _ => format!("Escaped path: {:?}", path.as_os_str()),
    }
}

fn cache_path_rows(ui: &mut Ui, categories: &[&CleanCategory], salt: &str) {
    ui.small(
        "Names containing control characters or non-Unicode data are marked ‘Escaped path’ and shown in quoted OS-string notation (for example \\n). The selected path is unchanged.",
    );
    let total = categories.iter().fold(0_usize, |total, category| {
        total.saturating_add(category.candidates.len())
    });
    let row_height =
        ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
    egui::ScrollArea::both()
        .id_salt(("exact-cache-paths", salt))
        .max_height(180.0)
        .show_rows(ui, row_height, total, |ui, range| {
            for index in range {
                let mut remaining = index;
                for category in categories {
                    if let Some(path) = category.candidates.get(remaining) {
                        let display = cache_path_display(path);
                        ui.add(
                            egui::Label::new(egui::RichText::new(display).monospace())
                                .wrap_mode(egui::TextWrapMode::Extend),
                        );
                        break;
                    }
                    remaining = remaining.saturating_sub(category.candidates.len());
                }
            }
        });
}

struct OriginalCleanup {
    source: PathBuf,
    output: PathBuf,
    trashed: bool,
}
struct CleanupReview {
    categories: Vec<CleanCategory>,
    selected_ids: HashSet<String>,
}
#[derive(Clone, Copy, Debug)]
enum PathField {
    Checksum,
    ArchiveInput,
    ArchiveOutput,
    CryptSource,
    CryptOutput,
}
#[derive(PartialEq, Eq)]
struct ArchiveDraft {
    extract: bool,
    sources: Vec<PathBuf>,
    input: String,
    output: String,
    tar_gzip: bool,
    max_mib: u64,
}

pub(super) struct FileTools {
    path_input_error: Option<String>,
    checksum_path: String,
    algorithm: Algorithm,
    expected: String,
    hash: Task<String>,
    roots: Vec<PathBuf>,
    hidden: bool,
    duplicates: Task<DuplicateResult>,
    selected: HashSet<PathBuf>,
    archive_extract: bool,
    archive_folders: bool,
    sources: Vec<PathBuf>,
    archive_input: String,
    archive_output: String,
    tar: bool,
    max_mib: u64,
    archive: Task<PathBuf>,
    cleanup: Task<Vec<CleanCategory>>,
    categories: HashSet<String>,
    clean_paths: Option<String>,
    clean_review: Option<CleanupReview>,
    trash: Task<usize>,
    message: String,
    decrypt: bool,
    source: String,
    output: String,
    password: String,
    confirmation: String,
    offer_trash: bool,
    crypt: Task<encrypt::TransformReceipt>,
    original_cleanup: Option<OriginalCleanup>,
}
impl Default for FileTools {
    fn default() -> Self {
        Self {
            path_input_error: None,
            checksum_path: String::new(),
            algorithm: Algorithm::Sha256,
            expected: String::new(),
            hash: Task::default(),
            roots: Vec::new(),
            hidden: false,
            duplicates: Task::default(),
            selected: HashSet::new(),
            archive_extract: false,
            archive_folders: false,
            sources: Vec::new(),
            archive_input: String::new(),
            archive_output: String::new(),
            tar: false,
            max_mib: 1024,
            archive: Task::default(),
            cleanup: Task::default(),
            categories: HashSet::new(),
            clean_paths: None,
            clean_review: None,
            trash: Task::default(),
            message: String::new(),
            decrypt: false,
            source: String::new(),
            output: String::new(),
            password: String::new(),
            confirmation: String::new(),
            offer_trash: false,
            crypt: Task::default(),
            original_cleanup: None,
        }
    }
}
impl FileTools {
    fn path_field(&mut self, field: PathField) -> &mut String {
        match field {
            PathField::Checksum => &mut self.checksum_path,
            PathField::ArchiveInput => &mut self.archive_input,
            PathField::ArchiveOutput => &mut self.archive_output,
            PathField::CryptSource => &mut self.source,
            PathField::CryptOutput => &mut self.output,
        }
    }
    fn admit_path(
        &mut self,
        field: PathField,
        path: &std::path::Path,
    ) -> oxbox_core::CoreResult<()> {
        match crate::host::path_text(path) {
            Ok(text) => {
                *self.path_field(field) = text;
                self.path_input_error = None;
                Ok(())
            }
            Err(error) => {
                self.path_input_error = Some(format!("Selection unchanged: {error}"));
                match field {
                    PathField::Checksum => self.hash.abandon(),
                    PathField::ArchiveInput | PathField::ArchiveOutput => self.archive.abandon(),
                    PathField::CryptSource | PathField::CryptOutput => {
                        self.crypt.abandon();
                        self.trash.abandon();
                        self.reset_crypt_result();
                        self.password.clear();
                        self.confirmation.clear();
                    }
                }
                Err(error)
            }
        }
    }
    fn pick_field(&mut self, ui: &mut Ui, label: &str, field: PathField, kind: u8) {
        let previous = self.path_field(field).clone();
        let chosen = picker(ui, label, self.path_field(field), kind);
        if previous != *self.path_field(field) {
            self.path_input_error = None;
        }
        if let Some(path) = chosen {
            let _ = self.admit_path(field, &path);
        }
    }
    pub fn checksum_preset(&self) -> oxbox_core::presets::ChecksumPreset {
        oxbox_core::presets::ChecksumPreset {
            algorithm: self.algorithm,
        }
    }
    pub fn apply_checksum_preset(
        &mut self,
        p: &oxbox_core::presets::ChecksumPreset,
    ) -> oxbox_core::CoreResult<()> {
        self.hash.poll();
        if self.hash.running() {
            return Err(oxbox_core::CoreError::InvalidInput(
                "Finish or cancel the checksum job before applying a preset.".into(),
            ));
        }
        if self.algorithm != p.algorithm {
            self.expected.clear();
        }
        self.algorithm = p.algorithm;
        self.hash.abandon();
        Ok(())
    }
    pub(super) fn clear_secrets(&mut self) {
        self.password.clear();
        self.confirmation.clear();
    }
    pub(super) fn leave_tool(&mut self, id: &str) {
        self.path_input_error = None;
        match id {
            "verify-checksum" => self.hash.abandon(),
            "find-duplicates" => {
                self.duplicates.abandon();
                self.trash.abandon();
                self.selected.clear();
            }
            "archive" => self.archive.abandon(),
            "clean-drive" => {
                self.cleanup.abandon();
                self.trash.abandon();
                self.categories.clear();
                self.clean_paths = None;
                self.clean_review = None;
            }
            "encrypt-files" => {
                self.crypt.abandon();
                self.trash.abandon();
                self.reset_crypt_result();
                self.clear_secrets();
            }
            _ => {}
        }
    }
    pub fn poll(&mut self) {
        self.hash.poll();
        self.duplicates.poll();
        self.archive.poll();
        self.cleanup.poll();
        self.trash.poll();
        self.crypt.poll();
        if let Some(count) = self.trash.result.take() {
            if self
                .original_cleanup
                .as_ref()
                .is_some_and(|state| !state.trashed)
            {
                if count == 1 {
                    if let Some(state) = &mut self.original_cleanup {
                        state.trashed = true;
                    }
                } else {
                    self.original_cleanup = None;
                }
            }
            self.message =
                format!("Moved {count} files to Trash. Scan again before further cleanup.");
            self.duplicates.result = None;
            self.cleanup.result = None;
            self.clean_review = None;
            self.clean_paths = None;
            self.selected.clear();
            self.categories.clear();
        }
        if !self.trash.running()
            && self.trash.error.is_some()
            && self
                .original_cleanup
                .as_ref()
                .is_some_and(|state| !state.trashed)
        {
            self.original_cleanup = None;
        }
    }
    pub fn shutdown(&mut self) {
        self.clear_secrets();
        self.hash.cancel();
        self.duplicates.cancel();
        self.archive.cancel();
        self.cleanup.cancel();
        self.trash.cancel();
        self.crypt.cancel();
        self.hash.shutdown();
        self.duplicates.shutdown();
        self.archive.shutdown();
        self.cleanup.shutdown();
        self.trash.shutdown();
        self.crypt.shutdown();
    }
    pub fn show(&mut self, ui: &mut Ui, id: &str) {
        match id {
            "verify-checksum" => self.checksum(ui),
            "find-duplicates" => self.duplicates(ui),
            "archive" => self.archive(ui),
            "clean-drive" => self.clean(ui),
            "encrypt-files" => self.encrypt(ui),
            _ => {}
        }
        if let Some(error) = &self.path_input_error {
            crate::accessibility::alert(ui, error);
        }
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        self.trash.show(ui);
    }
    fn checksum(&mut self, ui: &mut Ui) {
        let busy = self.hash.running();
        ui.add_enabled_ui(!busy, |ui| {
            let before = self.checksum_path.clone();
            self.pick_field(ui, "file", PathField::Checksum, 0);
            if let Some(path) = dropped(ui).first() {
                let _ = self.admit_path(PathField::Checksum, path);
            }
            let old = self.algorithm;
            let choice_response = egui::ComboBox::from_label("Hash algorithm")
                .selected_text(format!("{:?}", self.algorithm))
                .show_ui(ui, |ui| {
                    for (algorithm, label) in [
                        (Algorithm::Sha256, "SHA-256"),
                        (Algorithm::Sha1, "SHA-1"),
                        (Algorithm::Md5, "MD5"),
                        (Algorithm::Blake3, "BLAKE3"),
                    ] {
                        ui.selectable_value(&mut self.algorithm, algorithm, label);
                    }
                });
            crate::accessibility::combo_expansion(&choice_response.response);
            if before != self.checksum_path || old != self.algorithm {
                self.hash.result = None;
            }
            if ui
                .add_enabled(
                    !self.checksum_path.is_empty() && self.path_input_error.is_none(),
                    egui::Button::new("Calculate checksum"),
                )
                .clicked()
            {
                let path = PathBuf::from(&self.checksum_path);
                let algorithm = self.algorithm;
                self.hash.start(ui.ctx(), move |cancel, progress| {
                    checksum::hash_file_with_progress(&path, algorithm, &cancel, progress)
                });
            }
        });
        self.hash.show(ui);
        field(ui, "Expected checksum (optional)", &mut self.expected);
        if let Some(hash) = &self.hash.result {
            ui.monospace(hash);
            if ui.button("Copy checksum").clicked() {
                ui.ctx().copy_text(hash.clone());
            }
            if !self.expected.trim().is_empty() {
                ui.label(if checksum::digests_match(hash, &self.expected) {
                    "Checksum matches."
                } else {
                    "Checksum does not match."
                });
            }
        }
        ui.label("Reads in chunks. SHA-1 and MD5 are compatibility algorithms, not modern authenticity checks.");
    }
    fn duplicates(&mut self, ui: &mut Ui) {
        let busy = self.duplicates.running() || self.trash.running();
        ui.add_enabled_ui(!busy, |ui| {
            if ui.button("Add folders…").clicked()
                && let Some(paths) = crate::host::dialogs::file(ui.ctx()).pick_folders()
            {
                for path in paths {
                    if !self.roots.contains(&path) {
                        self.roots.push(path);
                    }
                }
                self.duplicates.result = None;
                self.selected.clear();
            }
            for path in dropped(ui) {
                if !self.roots.contains(&path) {
                    self.roots.push(path);
                    self.duplicates.result = None;
                    self.selected.clear();
                }
            }
            let roots_before = self.roots.len();
            list(ui, &mut self.roots);
            if self.roots.len() != roots_before {
                self.duplicates.result = None;
                self.selected.clear();
            }
            if ui
                .checkbox(&mut self.hidden, "Include hidden files and folders")
                .changed()
            {
                self.duplicates.result = None;
                self.selected.clear();
            }
            if ui
                .add_enabled(!self.roots.is_empty(), egui::Button::new("Find duplicates"))
                .clicked()
            {
                let roots = self.roots.clone();
                let hidden = self.hidden;
                self.selected.clear();
                self.message.clear();
                self.duplicates.start(ui.ctx(), move |cancel, progress| {
                    duplicates::find_duplicates(&roots, hidden, &cancel, progress)
                });
            }
        });
        self.duplicates.show(ui);
        if let Some(result) = &self.duplicates.result {
            ui.label(format!(
                "{} groups in {} files; duplicate copies use {}.",
                result.groups.len(),
                result.scanned_files,
                bytes(result.wasted_bytes)
            ));
            ui.add_enabled_ui(!busy,|ui|{ui.horizontal(|ui|{for(oldest,label)in[(true,"Keep oldest"),(false,"Keep newest")]{if ui.button(label).clicked(){self.selected.clear();for group in &result.groups{let mut files=group.files.clone();files.sort_by(|a,b|{let order=a.modified_ms.cmp(&b.modified_ms);(if oldest{order}else{order.reverse()}).then_with(||a.path.cmp(&b.path))});self.selected.extend(files.into_iter().skip(1).map(|file|file.path));}}}if ui.button("Clear selection").clicked(){self.selected.clear();}});
                for(index,group)in result.groups.iter().enumerate(){ui.group(|ui|{ui.label(format!("Group {} · {} per file",index+1,bytes(group.size)));for file in &group.files{let mut checked=self.selected.contains(&file.path);if ui.checkbox(&mut checked,file.path.display().to_string()).changed(){if checked{self.selected.insert(file.path.clone());}else{self.selected.remove(&file.path);}}}});}
                let removes_group=result.groups.iter().any(|group|group.files.iter().all(|file|self.selected.contains(&file.path)));
                if removes_group{ui.label("Keep at least one file in every group.");}
                let total=result.groups.iter().map(|group|group.size*group.files.iter().filter(|file|self.selected.contains(&file.path)).count() as u64).sum();
                if ui.add_enabled(!self.selected.is_empty()&&!removes_group,egui::Button::new(format!("Review {} copies ({}) for Trash…",self.selected.len(),bytes(total)))).clicked(){let paths=self.selected.iter().cloned().collect::<Vec<_>>();let detail=paths.iter().take(20).map(|path|path.display().to_string()).collect::<Vec<_>>().join("\n");if confirm(ui.ctx(), "Review duplicate cleanup",format!("Move {} selected copies ({}) to Recycle Bin / Trash?\n\n{detail}\n\nOne copy per group is kept. Review all selected paths in the list.",paths.len(),bytes(total))){let groups=result.groups.clone();self.trash.start(ui.ctx(),move|cancel,progress|duplicates::trash_selected(&groups,&paths,&cancel,progress));}}
            });
        }
    }
    fn archive(&mut self, ui: &mut Ui) {
        let busy = self.archive.running();
        let previous = self.archive_draft();
        ui.add_enabled_ui(!busy,|ui|{
        ui.horizontal(|ui|{ui.selectable_value(&mut self.archive_extract,false,"Create archive");ui.selectable_value(&mut self.archive_extract,true,"Extract archive");});
        if self.archive_extract {self.pick_field(ui,"archive input",PathField::ArchiveInput,0);if let Some(path)=dropped(ui).first(){let _=self.admit_path(PathField::ArchiveInput,path);}self.pick_field(ui,"new destination folder",PathField::ArchiveOutput,2);ui.label("The destination must not exist. Choose or enter a new folder name, not an existing folder.");ui.horizontal(|ui|{let label=ui.label("Maximum extracted size (MiB)");let mut max_mib=self.max_mib;ui.add(egui::DragValue::new(&mut max_mib).range(1..=1_048_576)).labelled_by(label.id);if ui.is_enabled(){self.max_mib=max_mib;}});}
        else {ui.checkbox(&mut self.archive_folders,"Select folders in picker");if ui.button("Add archive inputs…").clicked(){let dialog=crate::host::dialogs::file(ui.ctx());let paths=if self.archive_folders{dialog.pick_folders()}else{dialog.pick_files()};if let Some(paths)=paths{for path in paths{if !self.sources.contains(&path){self.sources.push(path);}}}}for path in dropped(ui){if !self.sources.contains(&path){self.sources.push(path);}}list(ui,&mut self.sources);ui.horizontal(|ui|{ui.selectable_value(&mut self.tar,false,"ZIP");ui.selectable_value(&mut self.tar,true,"TAR.GZ");});self.pick_field(ui,"new archive filename",PathField::ArchiveOutput,2);}
        if self.archive_draft()!=previous{self.archive.result=None;self.archive.error=None;self.archive.progress=None;}
        let ready=self.path_input_error.is_none()&&!self.archive_output.is_empty()&&if self.archive_extract{!self.archive_input.is_empty()}else{!self.sources.is_empty()};
        if ui.add_enabled(ready,egui::Button::new(if self.archive_extract{"Review and extract…"}else{"Create archive"})).clicked(){let output=PathBuf::from(&self.archive_output);if self.archive_extract{let input=PathBuf::from(&self.archive_input);let limit=self.max_mib*1024*1024;if confirm(ui.ctx(), "Review extraction",format!("Extract into a NEW folder:\n{}\n\nOutput limited to {} MiB or 20× compressed size, whichever is smaller. Existing folders are never merged.",output.display(),self.max_mib)){self.archive.start(ui.ctx(),move|cancel,progress|archive::extract_archive(&input,&output,limit,&cancel,progress));}}else{let sources=self.sources.clone();let format=if self.tar{ArchiveFormat::TarGz}else{ArchiveFormat::Zip};self.archive.start(ui.ctx(),move|cancel,progress|archive::create_archive(&sources,&output,format,&cancel,progress));}}
    });
        self.archive.show(ui);
        if let Some(path) = &self.archive.result {
            ui.label(format!("Completed: {}", path.display()));
            if ui.button("Copy output path").clicked() {
                ui.ctx().copy_text(path.display().to_string());
            }
        }
        ui.label("Existing files stay intact. ZIP, TAR, TAR.GZ and 7z extraction rejects links/traversal and bounds metadata/output. A failed extraction may leave its separate new partial folder; review it before retrying.");
    }
    fn archive_draft(&self) -> ArchiveDraft {
        ArchiveDraft {
            extract: self.archive_extract,
            sources: self.sources.clone(),
            input: self.archive_input.clone(),
            output: self.archive_output.clone(),
            tar_gzip: self.tar,
            max_mib: self.max_mib,
        }
    }
    fn clean(&mut self, ui: &mut Ui) {
        let busy = self.cleanup.running() || self.trash.running();
        ui.label("Scan exact OS/user cache locations, review eligible files, then move only those reviewed files to Trash. Close affected browsers/apps first. History, cookies, passwords, documents and downloads are not cleanup categories. The Recycle Bin is size-only and never emptied.");
        ui.small("Each category is limited to 50,000 entries and 20 levels. Partial scans are read-only. Cache eligibility uses the latest available modification/access timestamp and keeps files newer than 24 hours; the OS may not maintain reliable last-access timestamps.");
        ui.small("Caches of recognized running Chrome, Chromium, Edge, Firefox or Safari processes are read-only. Browser activity is checked again before each Trash request; unrecognized builds and a process starting during the final OS call remain limitations. Close browsers first.");
        if ui
            .add_enabled(!busy, egui::Button::new("Scan user caches"))
            .clicked()
        {
            self.categories.clear();
            self.clean_paths = None;
            self.clean_review = None;
            self.message.clear();
            self.cleanup.start(ui.ctx(), move |cancel, progress| {
                clean_drive::scan_current(&cancel, progress)
            });
        }
        self.cleanup.show(ui);
        let previous_selection = self.categories.clone();
        let mut requested_review = None;
        if let Some(categories) = &self.cleanup.result {
            ui.add_enabled_ui(!busy, |ui| {
                for category in categories {
                    let mut selected = self.categories.contains(&category.id);
                    ui.group(|ui| {
                        if ui.add_enabled(!category.read_only && !category.limited && category.files > 0, egui::Checkbox::new(&mut selected, &category.name)).changed() {
                            if selected { self.categories.insert(category.id.clone()); } else { self.categories.remove(&category.id); }
                        }
                        ui.label(category.path.display().to_string());
                        ui.label(format!("{} files · {} · {} skipped{}", category.files, bytes(category.bytes), category.skipped,
                            if category.limited { " · Partial scan: read only" } else if category.read_only { " · Read only" } else { "" }));
                        if crate::accessibility::enabled_button(ui, !category.candidates.is_empty(), "Review exact file paths…", &format!("Review exact file paths for {}", category.name)).clicked() {
                            self.clean_paths = Some(category.id.clone());
                        }
                        if self.clean_paths.as_deref() == Some(category.id.as_str()) {
                            ui.label(format!("{} exact paths from this scan · scroll to inspect; this does not move anything", category.candidates.len()));
                            cache_path_rows(ui, &[category], &category.id);
                        }
                    });
                }
                let reviewed = categories.iter().filter(|category| self.categories.contains(&category.id) && !category.read_only && !category.limited && !category.candidates.is_empty()).collect::<Vec<_>>();
                let files = reviewed.iter().fold(0_u64, |total, category| total.saturating_add(category.candidates.len() as u64));
                let size = reviewed.iter().fold(0_u64, |total, category| total.saturating_add(category.bytes));
                ui.label(format!("Selected: {files} exact files, {}.", bytes(size)));
                if ui.add_enabled(files > 0, egui::Button::new("Review and move selected cache files to Trash…")).clicked() {
                    requested_review = Some(CleanupReview { categories: reviewed.into_iter().cloned().collect(), selected_ids: self.categories.clone() });
                }
            });
        }
        if previous_selection != self.categories
            || self
                .clean_review
                .as_ref()
                .is_some_and(|review| review.selected_ids != self.categories)
        {
            self.clean_review = None;
        }
        if let Some(review) = requested_review {
            self.clean_review = Some(review);
        }
        let mut cancel_review = false;
        let mut apply_review = false;
        if let Some(review) = &self.clean_review {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong("Confirm exact cache-file review");
                let categories = review.categories.iter().collect::<Vec<_>>();
                let files = categories.iter().fold(0_usize, |total, category| total.saturating_add(category.candidates.len()));
                let size = categories.iter().fold(0_u64, |total, category| total.saturating_add(category.bytes));
                ui.label(format!("Move exactly these {files} reviewed files ({}) to Recycle Bin / Trash?", bytes(size)));
                cache_path_rows(ui, &categories, "selected-cleanup-confirmation");
                ui.label("This immutable list is the selection sent to Trash. New files are excluded. Age, file identity and recognized browser activity are checked again; changed or locked entries are not forced. Cancel cannot reverse a Trash request already admitted to the OS.");
                ui.horizontal(|ui| {
                    apply_review = ui.add_enabled(!busy, egui::Button::new("Confirm these exact cache files for Trash")).clicked();
                    cancel_review = ui.add_enabled(!busy, egui::Button::new("Cancel cleanup review")).clicked();
                });
            });
        }
        if cancel_review {
            self.clean_review = None;
        } else if apply_review && let Some(review) = self.clean_review.take() {
            self.trash.start(ui.ctx(), move |cancel, progress| {
                clean_drive::clean_reviewed_categories(&review.categories, true, &cancel, progress)
            });
        }
    }
    fn encrypt(&mut self, ui: &mut Ui) {
        let busy = self.crypt.running() || self.trash.running();
        ui.label("age scrypt encryption. Originals are kept; output must be new. Passphrases remain local and are never logged or persisted. Losing the passphrase loses access.");
        ui.label("New files use scrypt factor 18 (about 256 MiB plus overhead). Decryption supports factors through 20 (about 1 GiB plus overhead). Higher-factor legacy age files are refused and preserved; this is not a general memory-exhaustion guarantee.");
        ui.add_enabled_ui(!busy, |ui| {
            let previous_source = self.source.clone();
            let previous_output = self.output.clone();
            ui.horizontal(|ui| {
                let encrypt_selected = ui
                    .selectable_value(&mut self.decrypt, false, "Encrypt")
                    .changed();
                let decrypt_selected = ui
                    .selectable_value(&mut self.decrypt, true, "Decrypt")
                    .changed();
                if encrypt_selected || decrypt_selected {
                    self.source.clear();
                    self.output.clear();
                    self.password.clear();
                    self.confirmation.clear();
                    self.reset_crypt_result();
                }
            });
            self.pick_field(ui, "source file", PathField::CryptSource, 0);
            if let Some(path) = dropped(ui).first() {
                let _ = self.admit_path(PathField::CryptSource, path);
            }
            self.pick_field(ui, "new output filename", PathField::CryptOutput, 2);
            if self.source != previous_source || self.output != previous_output {
                self.reset_crypt_result();
            }
            password_field(ui, "Passphrase", &mut self.password);
            if !self.decrypt {
                password_field(ui, "Confirm passphrase", &mut self.confirmation);
                ui.label(if self.password.len() >= 24 {
                    "Long passphrase; prefer randomly chosen unrelated words."
                } else {
                    "Use at least 12 characters, preferably several unrelated words."
                });
            }
            ui.checkbox(
                &mut self.offer_trash,
                "Offer to move original to Trash after success (separate confirmation)",
            );
            let valid = self.path_input_error.is_none()
                && !self.source.is_empty()
                && !self.output.is_empty()
                && !self.password.is_empty()
                && (self.decrypt || self.password == self.confirmation);
            if ui
                .add_enabled(
                    valid,
                    egui::Button::new(if self.decrypt {
                        "Decrypt file"
                    } else {
                        "Encrypt file"
                    }),
                )
                .clicked()
            {
                let source = PathBuf::from(&self.source);
                let output = PathBuf::from(&self.output);
                let password = std::mem::take(&mut self.password);
                self.confirmation.clear();
                let decrypt = self.decrypt;
                self.reset_crypt_result();
                self.trash.result = None;
                self.trash.error = None;
                self.message.clear();
                self.crypt.start(ui.ctx(), move |cancel, progress| {
                    encrypt::transform_file_with_receipt(
                        &source, &output, password, decrypt, &cancel, progress,
                    )
                });
            }
        });
        self.crypt.show(ui);
        if let Some(receipt) = &self.crypt.result
            && self.original_cleanup_review(ui, &receipt.source, &receipt.output)
        {
            let receipt = receipt.clone();
            if confirm(
                ui.ctx(),
                "Move original to Trash?",
                format!(
                    "Output saved successfully to:\n{}\n\nMove ONLY the original to Recycle Bin / Trash?\n{}\n\nBoth files will be revalidated against the bytes processed by this job. Keep them closed and unchanged; this check is not an atomic lock against other programs.",
                    receipt.output.display(),
                    receipt.source.display()
                ),
            ) {
                let source = receipt.source.clone();
                let output = receipt.output.clone();
                if self.trash.start(ui.ctx(), move |cancel, progress| {
                    encrypt::trash_original(&receipt, true, &cancel, progress)
                }) {
                    self.track_original_cleanup(&source, &output);
                }
            }
        }
    }
    fn track_original_cleanup(&mut self, source: &std::path::Path, output: &std::path::Path) {
        self.original_cleanup = Some(OriginalCleanup {
            source: source.to_path_buf(),
            output: output.to_path_buf(),
            trashed: false,
        });
    }
    fn original_was_trashed(&self, source: &std::path::Path, output: &std::path::Path) -> bool {
        self.original_cleanup
            .as_ref()
            .is_some_and(|state| state.trashed && state.source == source && state.output == output)
    }
    fn original_cleanup_review(
        &self,
        ui: &mut Ui,
        source: &std::path::Path,
        output: &std::path::Path,
    ) -> bool {
        let trashed = self.original_was_trashed(source, output);
        let summary = format!(
            "Saved {}. {}",
            output.display(),
            if trashed {
                "Original moved to Trash."
            } else if self.trash.error.is_some() {
                // Cancellation cannot undo a Trash call already admitted to the OS.
                "Check the original and Trash before retrying cleanup."
            } else if self.original_cleanup.is_some() && self.trash.running() {
                "Original cleanup is in progress."
            } else {
                "Original kept."
            }
        );
        if trashed {
            crate::accessibility::status(ui, &summary);
        } else {
            ui.label(summary);
        }
        self.offer_trash
            && !trashed
            && ui
                .add_enabled(
                    !self.trash.running(),
                    egui::Button::new("Review original for Trash…"),
                )
                .clicked()
    }
    fn reset_crypt_result(&mut self) {
        self.crypt.result = None;
        self.original_cleanup = None;
        self.message.clear();
    }
}

#[cfg(test)]
mod input_state_tests {
    use super::*;
    use std::{
        path::Path,
        sync::{Arc, mpsc},
        time::Duration,
    };

    #[derive(Debug)]
    struct GeneratedDrop(PathBuf);
    impl egui::DroppedFile for GeneratedDrop {
        fn path(&self) -> &Path {
            &self.0
        }
        fn bytes(&self) -> Result<Vec<u8>, String> {
            Err("UI draft test must not read the dropped file".into())
        }
    }
    fn render_drop(tools: &mut FileTools, context: &egui::Context, id: &str, path: PathBuf) {
        let mut output = context.run_ui(
            egui::RawInput {
                dropped_files: vec![Arc::new(GeneratedDrop(path))],
                ..Default::default()
            },
            |ui| tools.show(ui, id),
        );
        output.textures_delta.clear();
    }
    fn hold<T: Send + 'static>(task: &mut Task<T>, context: &egui::Context) -> mpsc::Sender<()> {
        let (ready_send, ready_receive) = mpsc::channel();
        let (release_send, release_receive) = mpsc::channel();
        assert!(task.start(context, move |cancel, _| {
            ready_send.send(()).unwrap();
            loop {
                cancel.check()?;
                match release_receive.recv_timeout(Duration::from_millis(5)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(oxbox_core::CoreError::Cancelled);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        }));
        ready_receive.recv_timeout(Duration::from_secs(2)).unwrap();
        release_send
    }
    fn drafts(tools: &FileTools) -> (String, Vec<PathBuf>, Vec<PathBuf>, String, String) {
        (
            tools.checksum_path.clone(),
            tools.roots.clone(),
            tools.sources.clone(),
            tools.archive_input.clone(),
            tools.source.clone(),
        )
    }

    #[test]
    fn busy_file_path_fields_refuse_accessibility_value_changes() {
        for (id, field) in [
            ("verify-checksum", PathField::Checksum),
            ("archive", PathField::ArchiveInput),
            ("archive", PathField::ArchiveOutput),
            ("encrypt-files", PathField::CryptSource),
            ("encrypt-files", PathField::CryptOutput),
        ] {
            let original = "Generated path belonging to the active job".to_owned();
            let mut tools = FileTools {
                archive_extract: true,
                ..Default::default()
            };
            *tools.path_field(field) = original.clone();
            let context = egui::Context::default();
            context.enable_accesskit();
            let _release = match id {
                "verify-checksum" => hold(&mut tools.hash, &context),
                "archive" => hold(&mut tools.archive, &context),
                _ => hold(&mut tools.crypt, &context),
            };
            let mut first = context.run_ui(egui::RawInput::default(), |ui| tools.show(ui, id));
            first.textures_delta.clear();
            let target = first
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == egui::accesskit::Role::TextInput
                        && node.value() == Some(original.as_str()))
                    .then_some(id)
                })
                .unwrap();
            let mut second = context.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::AccessKitActionRequest(
                        egui::accesskit::ActionRequest {
                            action: egui::accesskit::Action::SetValue,
                            target_node: target,
                            target_tree: egui::accesskit::TreeId::ROOT,
                            data: Some(egui::accesskit::ActionData::Value(
                                "Generated unrelated replacement path".into(),
                            )),
                        },
                    )],
                    ..Default::default()
                },
                |ui| tools.show(ui, id),
            );
            second.textures_delta.clear();
            assert_eq!(
                tools.path_field(field),
                &original,
                "disabled {id} input accepted a new label for an active job"
            );
            tools.shutdown();
        }
    }

    #[test]
    fn busy_archive_budget_refuses_accessibility_numeric_changes() {
        use egui::accesskit::{Action, ActionData, ActionRequest, Role, TreeId};
        for (action, data) in [
            (Action::SetValue, Some(ActionData::NumericValue(9999.0))),
            (Action::Increment, None),
        ] {
            let mut tools = FileTools {
                archive_extract: true,
                max_mib: 1024,
                ..Default::default()
            };
            let context = egui::Context::default();
            context.enable_accesskit();
            let _release = hold(&mut tools.archive, &context);
            let mut first =
                context.run_ui(egui::RawInput::default(), |ui| tools.show(ui, "archive"));
            first.textures_delta.clear();
            let target = first
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == Role::SpinButton && node.numeric_value() == Some(1024.0))
                        .then_some(id)
                })
                .unwrap();
            let mut second = context.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::AccessKitActionRequest(ActionRequest {
                        action,
                        data,
                        target_tree: TreeId::ROOT,
                        target_node: target,
                    })],
                    ..Default::default()
                },
                |ui| tools.show(ui, "archive"),
            );
            second.textures_delta.clear();
            assert_eq!(
                tools.max_mib, 1024,
                "disabled extraction budget changed during an active job"
            );
            tools.shutdown();
        }
    }

    #[test]
    fn busy_drop_cannot_relabel_an_active_file_job_with_different_inputs() {
        for (id, extract) in [
            ("verify-checksum", false),
            ("find-duplicates", false),
            ("archive", false),
            ("archive", true),
            ("encrypt-files", false),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let old = directory.path().join("old-input.txt");
            let new = directory.path().join("new-input.txt");
            std::fs::write(&old, b"old fixture").unwrap();
            std::fs::write(&new, b"new fixture").unwrap();
            let old_root = directory.path().join("old-root");
            let new_root = directory.path().join("new-root");
            std::fs::create_dir(&old_root).unwrap();
            std::fs::create_dir(&new_root).unwrap();
            let mut tools = FileTools {
                checksum_path: old.display().to_string(),
                roots: vec![old_root],
                sources: vec![old.clone()],
                archive_input: old.display().to_string(),
                source: old.display().to_string(),
                archive_extract: extract,
                ..Default::default()
            };
            let context = egui::Context::default();
            let _release = match id {
                "verify-checksum" => hold(&mut tools.hash, &context),
                "find-duplicates" => hold(&mut tools.duplicates, &context),
                "archive" => hold(&mut tools.archive, &context),
                _ => hold(&mut tools.crypt, &context),
            };
            let before = drafts(&tools);
            render_drop(
                &mut tools,
                &context,
                id,
                if id == "find-duplicates" {
                    new_root
                } else {
                    new
                },
            );
            assert_eq!(
                drafts(&tools),
                before,
                "busy {id} (extract={extract}) accepted a new drop for an old job"
            );
            tools.shutdown();
        }
    }

    #[test]
    fn idle_drop_still_accepts_a_new_checksum_input_and_clears_old_digest() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("new-input.txt");
        std::fs::write(&path, b"fixture").unwrap();
        let mut tools = FileTools::default();
        tools.hash.result = Some("digest of previous input".into());
        render_drop(
            &mut tools,
            &egui::Context::default(),
            "verify-checksum",
            path.clone(),
        );
        assert_eq!(tools.checksum_path, path.display().to_string());
        assert!(tools.hash.result.is_none());
        tools.shutdown();
    }

    #[test]
    fn archive_changed_input_does_not_keep_an_old_completion_or_error() {
        for failed in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let old = directory.path().join("old-input.txt");
            let new = directory.path().join("new-input.txt");
            std::fs::write(&old, b"old fixture").unwrap();
            std::fs::write(&new, b"new fixture").unwrap();
            let mut tools = FileTools {
                sources: vec![old],
                ..Default::default()
            };
            if failed {
                tools.archive.error = Some("previous archive attempt failed".into());
            } else {
                tools.archive.result = Some(directory.path().join("previous-output.zip"));
            }
            render_drop(
                &mut tools,
                &egui::Context::default(),
                "archive",
                new.clone(),
            );
            assert!(tools.sources.contains(&new));
            assert!(
                tools.archive.result.is_none(),
                "an old archive success survived a changed input set"
            );
            assert!(
                tools.archive.error.is_none(),
                "an old archive error survived a changed input set"
            );
            tools.shutdown();
        }
    }
}

#[cfg(test)]
mod verification56_tests {
    use super::*;
    #[test]
    fn cleanup_review_display_distinguishes_exact_os_names_and_escaped_controls() {
        let parent = std::env::temp_dir().join("oxbox-clean-display-only");
        let ordinary = parent.join("ordinary cache.bin");
        assert_eq!(cache_path_display(&ordinary), ordinary.to_str().unwrap());
        let newline = parent.join("line\ncache.bin");
        let literal = parent.join("line\\ncache.bin");
        assert_ne!(cache_path_display(&newline), cache_path_display(&literal));
        assert!(!cache_path_display(&newline).contains('\n'));
        let carriage = parent.join("line\rcache.bin");
        let literal_carriage = parent.join("line\\rcache.bin");
        assert_ne!(
            cache_path_display(&carriage),
            cache_path_display(&literal_carriage)
        );
        assert!(!cache_path_display(&carriage).contains('\r'));
        #[cfg(windows)]
        let invalid = {
            use std::os::windows::ffi::OsStringExt;
            [0xD800_u16, 0xD801_u16].map(|unit| parent.join(std::ffi::OsString::from_wide(&[unit])))
        };
        #[cfg(unix)]
        let invalid = {
            use std::os::unix::ffi::OsStringExt;
            [0xFF_u8, 0xFE_u8].map(|byte| parent.join(std::ffi::OsString::from_vec(vec![byte])))
        };
        #[cfg(any(windows, unix))]
        {
            let replacement = parent.join("\u{FFFD}");
            assert_ne!(
                cache_path_display(&invalid[0]),
                cache_path_display(&invalid[1])
            );
            for path in invalid {
                assert_ne!(cache_path_display(&path), cache_path_display(&replacement));
                assert!(!cache_path_display(&path).contains('\n'));
                assert!(!cache_path_display(&path).contains('\r'));
            }
        }
    }

    fn unrepresentable_path() -> PathBuf {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            PathBuf::from(std::ffi::OsString::from_wide(&[
                67, 58, 92, 103, 101, 110, 101, 114, 97, 116, 101, 100, 92, 0xD800,
            ]))
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            PathBuf::from(std::ffi::OsString::from_vec(
                b"/generated/invalid-\xff".to_vec(),
            ))
        }
        #[cfg(not(any(windows, unix)))]
        {
            PathBuf::from("generated\ninvalid")
        }
    }
    #[test]
    fn utility_string_picker_fields_refuse_unrepresentable_paths_and_clear_old_admission() {
        for field in [
            PathField::Checksum,
            PathField::ArchiveInput,
            PathField::ArchiveOutput,
            PathField::CryptSource,
            PathField::CryptOutput,
        ] {
            for invalid in [
                unrepresentable_path(),
                std::env::temp_dir().join("generated\npath"),
                std::env::temp_dir().join("generated\rpath"),
            ] {
                let original = std::env::temp_dir()
                    .join("previous-selected-file")
                    .display()
                    .to_string();
                let mut tools = FileTools::default();
                *tools.path_field(field) = original.clone();
                tools.hash.result = Some("previous digest".into());
                tools.archive.result = Some(PathBuf::from("previous archive"));
                tools.track_original_cleanup(
                    std::path::Path::new("old-source"),
                    std::path::Path::new("old-output"),
                );
                assert!(
                    tools.admit_path(field, &invalid).is_err(),
                    "{field:?} accepted an unrepresentable picker path"
                );
                assert_eq!(*tools.path_field(field), original);
                assert!(tools.path_input_error.is_some());
                match field {
                    PathField::Checksum => assert!(tools.hash.result.is_none()),
                    PathField::ArchiveInput | PathField::ArchiveOutput => {
                        assert!(tools.archive.result.is_none())
                    }
                    PathField::CryptSource | PathField::CryptOutput => {
                        assert!(tools.original_cleanup.is_none())
                    }
                }
                tools.shutdown();
            }
        }
    }
    #[test]
    fn utility_string_drop_fields_refuse_unrepresentable_paths_without_retargeting() {
        #[derive(Debug)]
        struct ModelDrop(PathBuf);
        impl egui::DroppedFile for ModelDrop {
            fn path(&self) -> &std::path::Path {
                &self.0
            }
            fn bytes(&self) -> Result<Vec<u8>, String> {
                Err("model must not read file bytes".into())
            }
        }
        for (id, field) in [
            ("verify-checksum", PathField::Checksum),
            ("archive", PathField::ArchiveInput),
            ("encrypt-files", PathField::CryptSource),
        ] {
            let original = std::env::temp_dir()
                .join("old-selected-file")
                .display()
                .to_string();
            let mut tools = FileTools {
                archive_extract: true,
                ..Default::default()
            };
            *tools.path_field(field) = original.clone();
            let context = egui::Context::default();
            let mut output = context.run_ui(
                egui::RawInput {
                    dropped_files: vec![std::sync::Arc::new(ModelDrop(unrepresentable_path()))],
                    ..Default::default()
                },
                |ui| tools.show(ui, id),
            );
            output.textures_delta.clear();
            assert_eq!(
                *tools.path_field(field),
                original,
                "{id} retargeted an invalid drop"
            );
            assert!(tools.path_input_error.is_some());
            assert!(!tools.hash.running() && !tools.archive.running() && !tools.crypt.running());
            tools.shutdown();
        }
    }
    #[test]
    fn cleanup_exact_candidate_paths_can_be_reviewed_before_any_trash() {
        let directory = tempfile::tempdir().unwrap();
        let cache = directory.path().join("generated-cache");
        std::fs::create_dir(&cache).unwrap();
        let candidate = cache.join("exact-reviewed-cache-file.bin");
        std::fs::write(&candidate, b"generated fixture").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 86400);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&candidate)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(old)
                    .set_accessed(old),
            )
            .unwrap();
        let roots = [clean_drive::CleanRoot {
            id: "generated-review".into(),
            name: "Generated test cache".into(),
            path: cache,
            older_than_day: true,
            read_only: false,
            file_prefix: None,
        }];
        let review = clean_drive::scan_roots(
            &roots,
            &oxbox_core::jobs::CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            review[0].candidates,
            vec![candidate.clone()],
            "fixture must yield an exact reviewed file"
        );
        let mut tools = FileTools::default();
        tools.cleanup.result = Some(review);
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut frame = |events| {
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| tools.show(ui, "clean-drive"),
            );
            output.textures_delta.clear();
            output.platform_output.accesskit_update.unwrap().nodes
        };
        let nodes = frame(Vec::new());
        let review_button = nodes.iter().find(|(_,node)|node.role()==egui::accesskit::Role::Button&&node.label()==Some("Review exact file paths for Generated test cache")).map(|(id,_)|*id).expect("Clean Drive needs a control to inspect actual candidate paths, not only category counts");
        let nodes = frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_node: review_button,
                target_tree: egui::accesskit::TreeId::ROOT,
                data: None,
            },
        )]);
        let expected = candidate.display().to_string();
        assert!(
            nodes.iter().any(|(_, node)| node
                .label()
                .or_else(|| node.value())
                .is_some_and(|text| text.contains(&expected))),
            "exact reviewed candidate path must be visible"
        );
        assert!(!tools.trash.running());
        assert!(candidate.exists());
        tools.shutdown();
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn encrypted_original_cleanup_survives_poll_and_does_not_reoffer_trash() {
        let source = PathBuf::from("generated-original.txt");
        let output = PathBuf::from("generated-backup.age");
        let mut tools = FileTools {
            offer_trash: true,
            ..Default::default()
        };
        tools.track_original_cleanup(&source, &output);
        tools.trash.result = Some(1);
        tools.poll();
        assert!(
            tools.trash.result.is_none(),
            "shared poller consumes the completion"
        );
        let nodes = crate::accessibility::test_tree(|ui| {
            tools.original_cleanup_review(ui, &source, &output);
        });
        assert!(
            nodes.iter().any(|node| {
                node.label()
                    .or_else(|| node.value())
                    .is_some_and(|label| label.contains("Original moved to Trash"))
            }),
            "cleanup state={}, nodes={nodes:?}",
            tools.original_was_trashed(&source, &output)
        );
        assert!(
            !nodes
                .iter()
                .any(|node| node.role() == egui::accesskit::Role::Button
                    && node.label() == Some("Review original for Trash…"))
        );
        tools.poll();
        assert!(tools.original_was_trashed(&source, &output));
        tools.reset_crypt_result();
        assert!(!tools.original_was_trashed(&source, &output));
        tools.trash.result = Some(1); // unrelated generic cleanup completion
        tools.poll();
        assert!(!tools.original_was_trashed(&source, &output));
        tools.track_original_cleanup(&source, &output);
        tools.trash.error = Some("Cancelled".into());
        tools.poll();
        assert!(!tools.original_was_trashed(&source, &output));
        let nodes = crate::accessibility::test_tree(|ui| {
            tools.original_cleanup_review(ui, &source, &output);
        });
        assert!(nodes.iter().any(|node| {
            node.label()
                .or_else(|| node.value())
                .is_some_and(|text| text.contains("Check the original and Trash"))
        }));
        assert!(!nodes.iter().any(|node| {
            node.label()
                .or_else(|| node.value())
                .is_some_and(|text| text.contains("Original kept."))
        }));
        tools.track_original_cleanup(&source, &output);
        tools.trash.result = Some(1);
        tools.trash.error = None;
        tools.poll();
        tools.leave_tool("encrypt-files");
        assert!(!tools.original_was_trashed(&source, &output));
    }

    fn render_password(
        context: &egui::Context,
        value: &mut String,
        label: &str,
        time: f64,
    ) -> egui::Id {
        let mut id = None;
        let mut output = context.run_ui(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                id = Some(password_field(ui, label, value).id);
            },
        );
        output.textures_delta.clear();
        id.unwrap()
    }

    #[test]
    fn cleared_passphrase_cannot_be_restored_from_widget_undo_history() {
        for label in ["Passphrase", "Confirm passphrase"] {
            let context = egui::Context::default();
            let mut draft = "generated passphrase undo fixture".to_owned();
            let id = render_password(&context, &mut draft, label, 0.0);
            context.memory_mut(|memory| memory.request_focus(id));
            render_password(&context, &mut draft, label, 1.0);
            draft.clear();
            render_password(&context, &mut draft, label, 2.0);
            let state = egui::text_edit::TextEditState::load(&context, id).unwrap();
            let mut undoer = state.undoer();
            assert!(
                undoer
                    .undo(&(egui::text::CCursorRange::default(), String::new()))
                    .is_none(),
                "cleared password retained a restorable undo entry"
            );
            assert!(
                !undoer.is_in_flux(),
                "password retained an in-progress undo snapshot"
            );
            assert!(draft.is_empty());
        }
    }

    #[test]
    fn archive_extraction_limit_has_an_accessible_name() {
        let mut tools = FileTools {
            archive_extract: true,
            ..Default::default()
        };
        let nodes = crate::accessibility::test_tree(|ui| tools.show(ui, "archive"));
        crate::accessibility::assert_named_controls(&nodes, "archive extraction");
    }

    #[test]
    fn selected_path_remove_button_names_its_target() {
        let mut paths = vec![PathBuf::from("generated-file-fixture.txt")];
        let nodes = crate::accessibility::test_tree(|ui| list(ui, &mut paths));
        assert!(
            nodes
                .iter()
                .any(|node| node.role() == egui::accesskit::Role::Button
                    && node.label() == Some("Remove selected path generated-file-fixture.txt"))
        );
    }
}
