//! Bounded local macOS/Linux application inventory and explicitly reviewed Trash.
//! Registered Windows uninstallers remain in the existing `uninstall` tool.
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_SELECTED: usize = 256;
const MAX_DESKTOP_BYTES: usize = 64 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Macos,
    Linux,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kind {
    MacBundle,
    DesktopEntry,
    AppImage,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Root {
    path: PathBuf,
    kind: Kind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
    len: u64,
    modified_secs: u64,
    modified_nanos: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Metadata {
    identity: Identity,
    owner: u32,
    links: u64,
    directory: bool,
    regular: bool,
    link: bool,
    protected: bool,
}
#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub path: PathBuf,
    pub kind: Kind,
    pub executable: Option<String>,
    pub can_trash: bool,
    pub limitation: String,
    root: Root,
    reviewed: Metadata,
    reviewed_path: PathBuf,
}
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    pub applications: Vec<Application>,
    pub warnings: Vec<String>,
}
trait Backend {
    fn list(&mut self, root: &Root, cancel: &CancelToken) -> CoreResult<Vec<(PathBuf, Metadata)>>;
    fn metadata(&mut self, path: &Path) -> CoreResult<Metadata>;
    fn desktop(&mut self, path: &Path) -> CoreResult<String>;
    fn current_user(&self) -> u32;
    fn trash(&mut self, path: &Path, expected: &Metadata) -> CoreResult<()>;
    fn warnings(&mut self) -> Vec<String> {
        Vec::new()
    }
}
fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
pub fn scan(cancel: &CancelToken, progress: &mut dyn FnMut(Progress)) -> CoreResult<Inventory> {
    cancel.check()?;
    scan_with(
        &native_roots()?,
        cancel,
        progress,
        &mut SystemBackend::default(),
    )
}
pub fn trash_selected(
    reviewed: &[Application],
    selected: &[PathBuf],
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<usize> {
    cancel.check()?;
    let roots = native_roots()?;
    if reviewed.iter().any(|app| !roots.contains(&app.root)) {
        return Err(invalid("inventory roots changed; scan applications again"));
    }
    trash_with(
        reviewed,
        selected,
        confirmed,
        cancel,
        progress,
        &mut SystemBackend::default(),
    )
}
fn native_roots() -> CoreResult<Vec<Root>> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let home = dirs::home_dir()
            .ok_or_else(|| invalid("the current user's home directory is unavailable"))?;
        let platform = if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        };
        Ok(roots(platform, &home))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(CoreError::Unsupported(
            "non-Windows local application review; use Windows registered uninstallers",
        ))
    }
}
fn matching_kind(path: &Path, kind: Kind, metadata: &Metadata) -> bool {
    if metadata.link {
        return false;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    match kind {
        Kind::MacBundle => metadata.directory && extension.eq_ignore_ascii_case("app"),
        Kind::DesktopEntry => metadata.regular && extension == "desktop",
        Kind::AppImage => metadata.regular && extension.eq_ignore_ascii_case("AppImage"),
    }
}
fn apple_name(path: &Path) -> bool {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    name.starts_with("com.apple.")
        || name.starts_with("apple ")
        || matches!(
            name.as_str(),
            "app store"
                | "automator"
                | "books"
                | "calculator"
                | "calendar"
                | "chess"
                | "contacts"
                | "dictionary"
                | "facetime"
                | "find my"
                | "font book"
                | "freeform"
                | "home"
                | "image capture"
                | "launchpad"
                | "mail"
                | "maps"
                | "messages"
                | "mission control"
                | "music"
                | "news"
                | "notes"
                | "passwords"
                | "photo booth"
                | "photos"
                | "podcasts"
                | "preview"
                | "quicktime player"
                | "reminders"
                | "safari"
                | "shortcuts"
                | "siri"
                | "stickies"
                | "stocks"
                | "system settings"
                | "system preferences"
                | "textedit"
                | "time machine"
                | "tv"
                | "voice memos"
                | "weather"
                | "utilities"
                | "finder"
                | "terminal"
                | "activity monitor"
                | "disk utility"
        )
}
fn eligible(root: &Root, path: &Path, metadata: &Metadata, user: u32) -> bool {
    user != 0
        && metadata.owner == user
        && !metadata.protected
        && !metadata.link
        && metadata.identity.device != 0
        && metadata.identity.inode != 0
        && path.parent() == Some(root.path.as_path())
        && !path
            .components()
            .any(|component| component == std::path::Component::ParentDir)
        && matching_kind(path, root.kind, metadata)
        && match root.kind {
            Kind::MacBundle => !apple_name(path),
            Kind::AppImage => metadata.links == 1,
            Kind::DesktopEntry => false,
        }
}
fn desktop_fields(text: &str) -> CoreResult<(Option<String>, Option<String>)> {
    if text.len() > MAX_DESKTOP_BYTES || text.contains('\0') {
        return Err(invalid("desktop metadata exceeds 64 KiB or contains a NUL"));
    }
    let mut inside = false;
    let mut name = None;
    let mut exec = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[Desktop Entry]";
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let slot = match key {
            "Name" => &mut name,
            "Exec" => &mut exec,
            _ => continue,
        };
        if slot.is_some() {
            return Err(invalid("duplicate Name or Exec desktop metadata"));
        }
        if value.chars().any(char::is_control) {
            return Err(invalid(
                "desktop metadata contains unsupported control characters",
            ));
        }
        *slot = Some(value.chars().take(1024).collect());
    }
    Ok((name, exec))
}
fn scan_with(
    roots: &[Root],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    backend: &mut impl Backend,
) -> CoreResult<Inventory> {
    cancel.check()?;
    let mut inventory = Inventory::default();
    let mut count = 0_usize;
    let mut seen = HashSet::new();
    for root in roots {
        cancel.check()?;
        if count >= MAX_ENTRIES {
            inventory
                .warnings
                .push("Inventory limit reached; other locations may not have been scanned.".into());
            break;
        }
        let entries = match backend.list(root, cancel) {
            Ok(entries) => entries,
            Err(CoreError::Cancelled) => return Err(CoreError::Cancelled),
            Err(error) => {
                inventory
                    .warnings
                    .push(format!("{}: {error}", root.path.display()));
                continue;
            }
        };
        for (path, metadata) in entries {
            cancel.check()?;
            if count >= MAX_ENTRIES {
                inventory
                    .warnings
                    .push("Inventory limit reached; this is a partial inventory.".into());
                break;
            }
            count += 1;
            if path.parent() != Some(root.path.as_path())
                || !seen.insert(path.clone())
                || !matching_kind(&path, root.kind, &metadata)
            {
                continue;
            }
            let fallback = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let (name, executable) = if root.kind == Kind::DesktopEntry {
                match backend
                    .desktop(&path)
                    .and_then(|text| desktop_fields(&text))
                {
                    Ok((name, exec)) => (
                        name.filter(|value| !value.is_empty()).unwrap_or(fallback),
                        exec,
                    ),
                    Err(error) => {
                        if inventory.warnings.len() < 64 {
                            inventory
                                .warnings
                                .push(format!("{}: {error}", path.display()));
                        }
                        continue;
                    }
                }
            } else {
                (fallback, None)
            };
            let can_trash = eligible(root, &path, &metadata, backend.current_user());
            let limitation=if can_trash{"Eligible only for exact reviewed Trash; saved data outside this path is preserved."}else if root.kind==Kind::DesktopEntry{"Read-only launcher metadata. Use the operating system software manager or the normal vendor uninstaller for the package."}else{"Read-only: not a current-user-owned eligible regular application, or its name/flags/links are protected. Use its normal OS/vendor uninstaller."}.to_owned();
            inventory.applications.push(Application {
                name,
                path: path.clone(),
                kind: root.kind,
                executable,
                can_trash,
                limitation,
                root: root.clone(),
                reviewed: metadata,
                reviewed_path: path,
            });
            progress(Progress {
                done: count as u64,
                total: 0,
                message: "Reading bounded local application metadata; no app was executed".into(),
            });
        }
    }
    inventory
        .warnings
        .extend(backend.warnings().into_iter().take(64));
    inventory
        .applications
        .sort_by_key(|application| (application.name.to_lowercase(), application.path.clone()));
    Ok(inventory)
}
fn trash_with(
    reviewed: &[Application],
    selected: &[PathBuf],
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    backend: &mut impl Backend,
) -> CoreResult<usize> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid(
            "confirm the exact reviewed application paths before Trash",
        ));
    }
    if selected.is_empty() || selected.len() > MAX_SELECTED || reviewed.len() > MAX_ENTRIES {
        return Err(invalid(
            "select between 1 and 256 applications from a bounded fresh inventory",
        ));
    }
    let mut seen = HashSet::new();
    let mut checked = Vec::new();
    for path in selected {
        cancel.check()?;
        if !seen.insert(path) {
            return Err(invalid("duplicate application selection"));
        }
        let matches = reviewed
            .iter()
            .filter(|app| app.reviewed_path == *path)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(invalid(
                "application was not in the exact reviewed inventory",
            ));
        }
        let app = matches[0];
        let current = backend.metadata(path)?;
        if app.path != app.reviewed_path
            || app.kind != app.root.kind
            || current != app.reviewed
            || !eligible(&app.root, path, &current, backend.current_user())
        {
            return Err(invalid(
                "application ownership, identity, links or eligibility changed; rescan and review before Trash",
            ));
        }
        checked.push(app);
    }
    for (index, app) in checked.iter().enumerate() {
        cancel.check()?;
        let current = backend.metadata(&app.reviewed_path)?;
        if current != app.reviewed
            || !eligible(
                &app.root,
                &app.reviewed_path,
                &current,
                backend.current_user(),
            )
        {
            return Err(invalid(
                "application changed during review; later Trash actions were stopped",
            ));
        }
        backend.trash(&app.reviewed_path, &app.reviewed)?;
        progress(Progress {
            done: index as u64 + 1,
            total: checked.len() as u64,
            message: "Moved one exact reviewed app to Trash; saved data outside it was untouched"
                .into(),
        });
    }
    Ok(checked.len())
}

#[derive(Default)]
struct SystemBackend {
    warnings: Vec<String>,
}
impl Backend for SystemBackend {
    fn list(&mut self, root: &Root, cancel: &CancelToken) -> CoreResult<Vec<(PathBuf, Metadata)>> {
        cancel.check()?;
        match fs::symlink_metadata(&root.path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        check_local(&root.path)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&root.path)?.take(MAX_ENTRIES) {
            cancel.check()?;
            match entry {
                Ok(entry) => match self.metadata(&entry.path()) {
                    Ok(metadata) => entries.push((entry.path(), metadata)),
                    Err(error) => {
                        if self.warnings.len() < 64 {
                            self.warnings.push(format!(
                                "An item in {} was not inspected: {error}",
                                root.path.display()
                            ));
                        }
                    }
                },
                Err(error) => {
                    if self.warnings.len() < 64 {
                        self.warnings.push(format!(
                            "An item in {} could not be listed: {error}",
                            root.path.display()
                        ));
                    }
                }
            }
        }
        if entries.len() == MAX_ENTRIES {
            self.warnings.push(format!(
                "{} reached the per-location inventory limit.",
                root.path.display()
            ));
        }
        Ok(entries)
    }
    fn metadata(&mut self, path: &Path) -> CoreResult<Metadata> {
        read_metadata(path)
    }
    fn desktop(&mut self, path: &Path) -> CoreResult<String> {
        check_local(path)?;
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > MAX_DESKTOP_BYTES as u64 {
            return Err(invalid("desktop metadata is not a bounded regular file"));
        }
        let mut text = String::new();
        file.take(MAX_DESKTOP_BYTES as u64 + 1)
            .read_to_string(&mut text)?;
        if text.len() > MAX_DESKTOP_BYTES {
            return Err(invalid("desktop metadata exceeds 64 KiB"));
        }
        Ok(text)
    }
    fn current_user(&self) -> u32 {
        #[cfg(unix)]
        {
            unsafe { libc::geteuid() }
        }
        #[cfg(not(unix))]
        {
            0
        }
    }
    fn trash(&mut self, path: &Path, expected: &Metadata) -> CoreResult<()> {
        if self.metadata(path)? != *expected {
            return Err(invalid("application changed immediately before Trash"));
        }
        #[cfg(target_os = "macos")]
        if path.parent() == Some(Path::new("/Applications")) {
            let root = Root {
                path: PathBuf::from("/Applications"),
                kind: Kind::MacBundle,
            };
            if !eligible(&root, path, expected, self.current_user()) {
                return Err(invalid("protected macOS application"));
            }
            // Narrow, user-approved application policy exception. Generic fs_safe still
            // rejects /Applications; only this sealed, freshly checked bundle can reach here.
            return trash::delete(path)
                .map_err(|error| CoreError::Io(std::io::Error::other(error.to_string())));
        }
        crate::fs_safe::move_to_trash(&[path.to_owned()])?;
        Ok(())
    }
    fn warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}
fn check_local(path: &Path) -> CoreResult<()> {
    if !path.is_absolute() {
        return Err(invalid("application locations must be absolute"));
    }
    crate::fs_safe::reject_symlink_components(path)?;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| invalid("invalid application path"))?;
        let mut info: libc::statfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statfs(name.as_ptr(), &mut info) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        #[cfg(target_os = "linux")]
        let local = matches!(
            info.f_type as u64,
            0xef53
                | 0x58465342
                | 0x9123683e
                | 0x01021994
                | 0x794c7630
                | 0x2fc12fc1
                | 0xf2f52010
                | 0x52654973
        );
        #[cfg(target_os = "macos")]
        let local = info.f_flags & libc::MNT_LOCAL as u32 != 0;
        if !local {
            return Err(invalid(
                "remote or unrecognized application filesystems are not inspected",
            ));
        }
    }
    Ok(())
}
#[cfg(unix)]
fn read_metadata(path: &Path) -> CoreResult<Metadata> {
    use std::os::unix::fs::MetadataExt;
    use std::time::UNIX_EPOCH;
    check_local(path)?;
    let metadata = fs::symlink_metadata(path)?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| invalid("unsupported application timestamp"))?;
    #[cfg(target_os = "macos")]
    let protected = std::os::darwin::fs::MetadataExt::st_flags(&metadata) != 0;
    #[cfg(not(target_os = "macos"))]
    let protected = false;
    Ok(Metadata {
        identity: Identity {
            device: metadata.dev(),
            inode: metadata.ino(),
            len: metadata.len(),
            modified_secs: modified.as_secs(),
            modified_nanos: modified.subsec_nanos(),
        },
        owner: metadata.uid(),
        links: metadata.nlink(),
        directory: metadata.is_dir(),
        regular: metadata.is_file(),
        link: crate::fs_safe::is_link(&metadata),
        protected,
    })
}
#[cfg(not(unix))]
fn read_metadata(_path: &Path) -> CoreResult<Metadata> {
    Err(CoreError::Unsupported("non-Windows application metadata"))
}
#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn roots(platform: Platform, home: &Path) -> Vec<Root> {
    match platform {
        Platform::Macos => vec![
            Root {
                path: PathBuf::from("/Applications"),
                kind: Kind::MacBundle,
            },
            Root {
                path: home.join("Applications"),
                kind: Kind::MacBundle,
            },
        ],
        Platform::Linux => vec![
            Root {
                path: PathBuf::from("/usr/share/applications"),
                kind: Kind::DesktopEntry,
            },
            Root {
                path: PathBuf::from("/usr/local/share/applications"),
                kind: Kind::DesktopEntry,
            },
            Root {
                path: home.join(".local/share/applications"),
                kind: Kind::DesktopEntry,
            },
            Root {
                path: home.join("Applications"),
                kind: Kind::AppImage,
            },
        ],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    struct Fake {
        entries: BTreeMap<PathBuf, Metadata>,
        texts: BTreeMap<PathBuf, String>,
        effects: Vec<PathBuf>,
    }
    fn home() -> PathBuf {
        std::env::temp_dir().join("oxbox-fake-application-user")
    }
    fn meta(id: u64, directory: bool) -> Metadata {
        Metadata {
            identity: Identity {
                device: 1,
                inode: id,
                len: 10,
                modified_secs: 1,
                modified_nanos: 0,
            },
            owner: 1000,
            links: 1,
            directory,
            regular: !directory,
            link: false,
            protected: false,
        }
    }
    impl Fake {
        fn new() -> Self {
            Self {
                entries: BTreeMap::new(),
                texts: BTreeMap::new(),
                effects: vec![],
            }
        }
        fn add(&mut self, path: PathBuf, metadata: Metadata) {
            self.entries.insert(path, metadata);
        }
    }
    impl Backend for Fake {
        fn list(&mut self, root: &Root, _: &CancelToken) -> CoreResult<Vec<(PathBuf, Metadata)>> {
            Ok(self
                .entries
                .iter()
                .filter(|(path, _)| path.parent() == Some(root.path.as_path()))
                .map(|(path, meta)| (path.clone(), meta.clone()))
                .collect())
        }
        fn metadata(&mut self, path: &Path) -> CoreResult<Metadata> {
            self.entries
                .get(path)
                .cloned()
                .ok_or_else(|| invalid("fake path changed"))
        }
        fn desktop(&mut self, path: &Path) -> CoreResult<String> {
            Ok(self.texts.get(path).cloned().unwrap_or_default())
        }
        fn current_user(&self) -> u32 {
            1000
        }
        fn trash(&mut self, path: &Path, expected: &Metadata) -> CoreResult<()> {
            assert_eq!(self.entries.get(path), Some(expected));
            self.effects.push(path.to_owned());
            self.entries.remove(path);
            Ok(())
        }
    }
    #[test]
    fn linux_inventory_reads_name_exec_as_inert_text_and_only_portable_images_are_removable() {
        let mut fake = Fake::new();
        let desktop = home().join(".local/share/applications/generated.desktop");
        fake.add(desktop.clone(), meta(1, false));
        fake.texts.insert(
            desktop,
            "[Desktop Entry]\nName=Generated App\nExec=browser https://example.invalid %u\n".into(),
        );
        let portable = home().join("Applications/Generated.AppImage");
        fake.add(portable.clone(), meta(2, false));
        let inventory = scan_with(
            &roots(Platform::Linux, &home()),
            &CancelToken::default(),
            &mut |_| {},
            &mut fake,
        )
        .unwrap();
        assert_eq!(inventory.applications.len(), 2);
        let entry = inventory
            .applications
            .iter()
            .find(|app| app.kind == Kind::DesktopEntry)
            .unwrap();
        assert_eq!(entry.name, "Generated App");
        assert_eq!(
            entry.executable.as_deref(),
            Some("browser https://example.invalid %u")
        );
        assert!(!entry.can_trash);
        assert!(
            inventory
                .applications
                .iter()
                .find(|app| app.path == portable)
                .unwrap()
                .can_trash
        );
        assert!(fake.effects.is_empty());
    }
    #[test]
    fn mac_inventory_never_offers_root_owned_apple_named_protected_or_linked_bundles() {
        let mut fake = Fake::new();
        let mut root_owned = meta(1, true);
        root_owned.owner = 0;
        fake.add(PathBuf::from("/Applications/RootOwned.app"), root_owned);
        fake.add(PathBuf::from("/Applications/Safari.app"), meta(2, true));
        let mut link = meta(3, true);
        link.link = true;
        fake.add(home().join("Applications/Linked.app"), link);
        let mut protected = meta(4, true);
        protected.protected = true;
        fake.add(home().join("Applications/Protected.app"), protected);
        fake.add(home().join("Applications/Generated.app"), meta(5, true));
        let inventory = scan_with(
            &roots(Platform::Macos, &home()),
            &CancelToken::default(),
            &mut |_| {},
            &mut fake,
        )
        .unwrap();
        let eligible = inventory
            .applications
            .iter()
            .filter(|app| app.can_trash)
            .collect::<Vec<_>>();
        assert_eq!(eligible.len(), 1);
        assert_eq!(eligible[0].name, "Generated");
        assert!(
            inventory
                .applications
                .iter()
                .all(|app| !app.path.starts_with("/System/Applications"))
        );
        assert!(fake.effects.is_empty());
    }
    #[test]
    fn stale_renamed_unreviewed_and_unconfirmed_selections_never_reach_trash() {
        let mut fake = Fake::new();
        let selected = home().join("Applications/Generated.AppImage");
        fake.add(selected.clone(), meta(1, false));
        let inventory = scan_with(
            &roots(Platform::Linux, &home()),
            &CancelToken::default(),
            &mut |_| {},
            &mut fake,
        )
        .unwrap();
        assert!(
            trash_with(
                &inventory.applications,
                std::slice::from_ref(&selected),
                false,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        assert!(
            trash_with(
                &inventory.applications,
                &[home().join("elsewhere/App.AppImage")],
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        fake.entries.get_mut(&selected).unwrap().identity.inode = 99;
        assert!(
            trash_with(
                &inventory.applications,
                &[selected],
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        assert!(fake.effects.is_empty());
    }
    #[test]
    fn exact_eligible_review_trashes_only_the_selected_fixture_and_cancelled_call_does_nothing() {
        let mut fake = Fake::new();
        let selected = home().join("Applications/Generated.AppImage");
        let other = home().join("Applications/Keep.AppImage");
        fake.add(selected.clone(), meta(1, false));
        fake.add(other.clone(), meta(2, false));
        let inventory = scan_with(
            &roots(Platform::Linux, &home()),
            &CancelToken::default(),
            &mut |_| {},
            &mut fake,
        )
        .unwrap();
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            trash_with(
                &inventory.applications,
                std::slice::from_ref(&selected),
                true,
                &cancel,
                &mut |_| {},
                &mut fake
            )
            .unwrap_err()
            .code(),
            "cancelled"
        );
        assert_eq!(
            trash_with(
                &inventory.applications,
                std::slice::from_ref(&selected),
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .unwrap(),
            1
        );
        assert_eq!(fake.effects, vec![selected]);
        assert!(fake.entries.contains_key(&other));
    }
    #[test]
    fn roots_are_fixed_and_exclude_system_applications_and_recursive_discovery() {
        let mac = roots(Platform::Macos, &home());
        assert_eq!(mac.len(), 2);
        assert!(
            mac.iter()
                .all(|root| root.path != Path::new("/System/Applications"))
        );
        assert_eq!(roots(Platform::Linux, &home()).len(), 4);
    }
}
