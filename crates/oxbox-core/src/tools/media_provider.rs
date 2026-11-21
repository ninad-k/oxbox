//! Offline external media providers, safe outputs and managed recording processes.
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
const MAX_OUTPUT: u64 = 20 * 1024 * 1024 * 1024;
fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn lock<T>(mutex: &Mutex<T>) -> CoreResult<std::sync::MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| invalid("media process state is unavailable"))
}

pub struct TemporaryFolder {
    path: PathBuf,
    files: Vec<PathBuf>,
    keep: bool,
}
impl TemporaryFolder {
    pub fn new(parent: &Path) -> CoreResult<Self> {
        let parent = parent.canonicalize()?;
        if !parent.is_dir() {
            return Err(invalid("choose an existing output folder"));
        }
        for _ in 0..16 {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = parent.join(format!(
                ".oxbox-media-{}-{now}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
                    }
                    return Ok(Self {
                        path,
                        files: Vec::new(),
                        keep: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(invalid("could not create a private media staging folder"))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn file(&mut self, name: &str) -> CoreResult<PathBuf> {
        if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
            return Err(invalid("invalid private media filename"));
        }
        let path = self.path.join(name);
        self.files.push(path.clone());
        Ok(path)
    }
    fn preserve(&mut self) {
        self.keep = true;
    }
}
impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        if !self.keep {
            for path in &self.files {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_dir(&self.path);
        }
    }
}

pub struct OutputPlan {
    destination: PathBuf,
    folder: TemporaryFolder,
    staging: PathBuf,
}
impl OutputPlan {
    pub fn new(output: &Path, extension: &str) -> CoreResult<Self> {
        crate::fs_safe::validate_output_filename(
            output
                .file_name()
                .ok_or_else(|| invalid("choose an output filename"))?,
        )?;
        if extension.is_empty()
            || extension.len() > 8
            || !extension.bytes().all(|value| value.is_ascii_alphanumeric())
        {
            return Err(invalid("invalid output format"));
        }
        if !output
            .extension()
            .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case(extension))
        {
            return Err(invalid(format!("choose a .{extension} output filename")));
        }
        let parent = output
            .parent()
            .ok_or_else(|| invalid("choose an output filename"))?
            .canonicalize()?;
        let destination = parent.join(
            output
                .file_name()
                .ok_or_else(|| invalid("choose an output filename"))?,
        );
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(invalid("the output already exists; choose a new filename"));
        }
        let mut folder = TemporaryFolder::new(&parent)?;
        let staging = folder.file(&format!("payload.{extension}"))?;
        Ok(Self {
            destination,
            folder,
            staging,
        })
    }
    pub fn staging_file(&self) -> &Path {
        &self.staging
    }
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    pub fn preserve(&mut self) {
        self.folder.preserve();
    }
    pub fn publish(&self) -> CoreResult<PathBuf> {
        self.publish_with_cancel(&CancelToken::default())
    }
    pub fn publish_with_cancel(&self, cancel: &CancelToken) -> CoreResult<PathBuf> {
        cancel.check()?;
        let metadata = fs::symlink_metadata(&self.staging)?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_OUTPUT {
            return Err(invalid(
                "the provider output is empty, invalid, or exceeds 20 GiB",
            ));
        }
        // Hard-link creation is atomic and fails if the destination already exists.
        if fs::hard_link(&self.staging, &self.destination).is_ok() {
            return Ok(self.destination.clone());
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.destination)?;
        let mut source = std::fs::File::open(&self.staging)?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            cancel.check()?;
            let count = source.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            output.write_all(&buffer[..count])?;
        }
        output.sync_all()?;
        Ok(self.destination.clone())
    }
}

pub fn find_executable(name: &str) -> CoreResult<PathBuf> {
    if !["ffmpeg", "ffprobe", "tesseract"].contains(&name) {
        return Err(invalid("unknown local media provider"));
    }
    let executable = if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    for folder in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        if folder.as_os_str().is_empty() {
            continue;
        }
        let candidate = folder.join(&executable);
        if candidate.is_file() {
            return Ok(candidate.canonicalize()?);
        }
    }
    Err(invalid(format!(
        "{name} is not installed or is not in PATH. Install a maintained local provider yourself; Oxbox does not download or bundle it."
    )))
}
fn command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command.env_remove("FFREPORT");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
/// A failed adapter/thread setup must never leave a capture process detached.
struct ManagedChild(Child);
impl std::ops::Deref for ManagedChild {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}
impl std::ops::DerefMut for ManagedChild {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        if !self.0.try_wait().is_ok_and(|status| status.is_some()) {
            terminate(&mut self.0);
        }
    }
}
struct Captured {
    reader: JoinHandle<std::io::Result<Vec<u8>>>,
    overflow: Arc<AtomicBool>,
}
fn capture(reader: impl Read + Send + 'static, limit: usize, tail: bool) -> Captured {
    let overflow = Arc::new(AtomicBool::new(false));
    let flag = overflow.clone();
    let handle = std::thread::spawn(move || {
        let mut reader = reader;
        let mut stored = VecDeque::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            for byte in &buffer[..count] {
                if stored.len() == limit {
                    if tail {
                        stored.pop_front();
                    } else {
                        flag.store(true, Ordering::Release);
                        continue;
                    }
                }
                stored.push_back(*byte);
            }
        }
        Ok(stored.into_iter().collect())
    });
    Captured {
        reader: handle,
        overflow,
    }
}
fn finish_capture(capture: Captured) -> CoreResult<Vec<u8>> {
    capture
        .reader
        .join()
        .map_err(|_| invalid("provider output reader failed"))?
        .map_err(Into::into)
}
fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
pub fn run_text(
    executable: &Path,
    args: &[String],
    cancel: &CancelToken,
    timeout: Duration,
    max_text: usize,
) -> CoreResult<String> {
    cancel.check()?;
    let mut child = ManagedChild(
        command(executable)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
    );
    let stdout = capture(
        child
            .stdout
            .take()
            .ok_or_else(|| invalid("provider stdout is unavailable"))?,
        max_text,
        false,
    );
    let stderr = capture(
        child
            .stderr
            .take()
            .ok_or_else(|| invalid("provider stderr is unavailable"))?,
        65536,
        true,
    );
    let start = Instant::now();
    let mut failure = None;
    let status = loop {
        if let Err(error) = cancel.check() {
            failure = Some(error);
            terminate(&mut child);
            break None;
        }
        if start.elapsed() > timeout || stdout.overflow.load(Ordering::Acquire) {
            failure = Some(invalid("provider time or text-output limit exceeded"));
            terminate(&mut child);
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(error) => {
                failure = Some(error.into());
                terminate(&mut child);
                break None;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text_exceeded = stdout.overflow.clone();
    let stdout = finish_capture(stdout)?;
    let stderr = finish_capture(stderr)?;
    if text_exceeded.load(Ordering::Acquire) {
        return Err(invalid("provider text-output limit exceeded"));
    }
    if let Some(error) = failure {
        return Err(error);
    }
    if !status.is_some_and(|status| status.success()) {
        return Err(invalid(format!(
            "local provider failed: {}",
            String::from_utf8_lossy(&stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub error: Option<String>,
}
pub fn provider_status(name: &str) -> ProviderStatus {
    match find_executable(name) {
        Ok(path) => {
            let version = run_text(
                &path,
                &[if name == "tesseract" {
                    "--version"
                } else {
                    "-version"
                }
                .into()],
                &CancelToken::default(),
                Duration::from_secs(5),
                65536,
            );
            let (version, error) = match version {
                Ok(text) => (text.lines().next().map(str::to_owned), None),
                Err(error) => (None, Some(error.to_string())),
            };
            ProviderStatus {
                name: name.into(),
                path: Some(path),
                version,
                error,
            }
        }
        Err(error) => ProviderStatus {
            name: name.into(),
            path: None,
            version: None,
            error: Some(error.to_string()),
        },
    }
}
pub fn file_input(path: &Path) -> CoreResult<Vec<String>> {
    let path = path.canonicalize()?;
    if !path.is_file() {
        return Err(invalid("choose a regular media file"));
    }
    Ok(vec![
        "-format_whitelist".into(),
        "matroska,webm,mov,mp4,m4a,3gp,3g2,mj2,avi,wav,mp3,flac,ogg,aac,gif,png_pipe,jpeg_pipe,webp_pipe,mpeg,mpegvideo,mpegts,asf,flv".into(),
        "-protocol_whitelist".into(),
        "file,pipe".into(),
        "-i".into(),
        path.to_string_lossy().into_owned(),
    ])
}
fn spawn_ffmpeg(args: &[String], plan: &OutputPlan, recording: bool) -> CoreResult<ManagedChild> {
    let executable = find_executable("ffmpeg")?;
    let mut process = command(&executable);
    process.args(["-hide_banner", "-loglevel", "error", "-nostats", "-n"]);
    if !recording {
        process.arg("-nostdin");
    }
    process
        .args(args)
        .arg(plan.staging_file())
        .stdin(if recording {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    Ok(ManagedChild(process.spawn()?))
}
pub fn ffmpeg_job(
    args: &[String],
    output: &Path,
    extension: &str,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<PathBuf> {
    cancel.check()?;
    let plan = OutputPlan::new(output, extension)?;
    let mut child = spawn_ffmpeg(args, &plan, false)?;
    let stderr = capture(
        child
            .stderr
            .take()
            .ok_or_else(|| invalid("FFmpeg stderr is unavailable"))?,
        65536,
        true,
    );
    let started = Instant::now();
    let mut failure = None;
    let status = loop {
        if let Err(error) = cancel.check() {
            failure = Some(error);
            terminate(&mut child);
            break None;
        }
        if started.elapsed() > Duration::from_secs(7200)
            || fs::metadata(plan.staging_file()).is_ok_and(|metadata| metadata.len() > MAX_OUTPUT)
        {
            failure = Some(invalid(
                "media job exceeded the two-hour or 20 GiB output limit",
            ));
            terminate(&mut child);
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(error) => {
                failure = Some(error.into());
                terminate(&mut child);
                break None;
            }
        }
        progress(Progress {
            done: 0,
            total: 0,
            message: format!("Local FFmpeg is working · {}s", started.elapsed().as_secs()),
        });
        std::thread::sleep(Duration::from_millis(100));
    };
    let log = finish_capture(stderr)?;
    if let Some(error) = failure {
        return Err(error);
    }
    if !status.is_some_and(|status| status.success()) {
        return Err(invalid(format!(
            "FFmpeg failed: {}",
            String::from_utf8_lossy(&log)
        )));
    }
    let path = plan.publish_with_cancel(cancel)?;
    progress(Progress {
        done: 1,
        total: 1,
        message: "Media file saved".into(),
    });
    Ok(path)
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub active: bool,
    pub id: String,
    pub kind: String,
    pub output: Option<PathBuf>,
    pub started_at: u64,
    pub error: Option<String>,
}
fn recording_finished(
    status: &Mutex<RecordingStatus>,
    plan: &mut OutputPlan,
    result: CoreResult<PathBuf>,
) {
    // Recovery data belongs to the user even when status reporting is unhealthy.
    if result.is_err() && plan.staging_file().is_file() {
        plan.preserve();
    }
    if let Ok(mut state) = status.lock() {
        state.active = false;
        match result {
            Ok(path) => state.output = Some(path),
            Err(error) => {
                if plan.staging_file().is_file() {
                    let partial = plan.staging_file().to_owned();
                    state.error = Some(format!(
                        "{error}. Partial recording kept at {}",
                        partial.display()
                    ));
                } else {
                    state.error = Some(error.to_string());
                }
            }
        }
    }
}
struct RecordingWorker {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}
#[derive(Default)]
pub struct Recorder {
    worker: Mutex<Option<RecordingWorker>>,
    status: Arc<Mutex<RecordingStatus>>,
    stopping: AtomicBool,
}
impl Recorder {
    pub fn status(&self) -> CoreResult<RecordingStatus> {
        Ok(lock(&self.status)?.clone())
    }
    pub fn start(
        &self,
        kind: &str,
        args: Vec<String>,
        output: &Path,
        extension: &str,
    ) -> CoreResult<RecordingStatus> {
        let mut worker = lock(&self.worker)?;
        if self.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        if worker
            .as_ref()
            .is_some_and(|worker| !worker.thread.is_finished())
        {
            return Err(invalid(
                "another recording is already active; stop it first",
            ));
        }
        if let Some(previous) = worker.take() {
            let _ = previous.thread.join();
        }
        let maximum = args
            .windows(2)
            .find(|pair| pair[0] == "-t")
            .and_then(|pair| pair[1].parse::<u64>().ok())
            .filter(|seconds| (1..=86_400).contains(seconds))
            .ok_or_else(|| {
                invalid("recording requires a validated 1-second to 24-hour duration")
            })?;
        let mut plan = OutputPlan::new(output, extension)?;
        let mut child = spawn_ffmpeg(&args, &plan, true)?;
        if self.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        let stderr = capture(
            child
                .stderr
                .take()
                .ok_or_else(|| invalid("FFmpeg stderr is unavailable"))?,
            65536,
            true,
        );
        let id = format!("recording-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let initial = RecordingStatus {
            active: true,
            id: id.clone(),
            kind: kind.into(),
            output: Some(plan.destination().to_owned()),
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error: None,
        };
        *lock(&self.status)? = initial.clone();
        let status = self.status.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = stop.clone();
        let thread = std::thread::Builder::new()
            .name("oxbox-recording".into())
            .spawn(move || {
                let mut requested_stop = None;
                let mut failure = None;
                let began = Instant::now();
                let exit = loop {
                    // A stalled device may never advance FFmpeg's media timestamps.
                    // Bound wall time too, allowing a short device startup grace period.
                    if (stop_worker.load(Ordering::Acquire)
                        || began.elapsed() > Duration::from_secs(maximum + 10))
                        && requested_stop.is_none()
                    {
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(b"q\n");
                            let _ = stdin.flush();
                        }
                        requested_stop = Some(Instant::now());
                    }
                    if requested_stop.is_some_and(|start| start.elapsed() > Duration::from_secs(5))
                    {
                        failure = Some(
                            "The provider did not stop gracefully; it was terminated.".to_string(),
                        );
                        terminate(&mut child);
                        break None;
                    }
                    if fs::metadata(plan.staging_file())
                        .is_ok_and(|metadata| metadata.len() > MAX_OUTPUT)
                    {
                        failure = Some("Recording exceeded the 20 GiB limit.".into());
                        terminate(&mut child);
                        break None;
                    }
                    match child.try_wait() {
                        Ok(Some(exit)) => break Some(exit),
                        Ok(None) => {}
                        Err(error) => {
                            failure = Some(error.to_string());
                            terminate(&mut child);
                            break None;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(100));
                };
                let log = finish_capture(stderr).unwrap_or_default();
                let result = if let Some(error) = failure {
                    Err(invalid(error))
                } else if !exit.is_some_and(|exit| exit.success()) {
                    Err(invalid(format!(
                        "Recording provider failed: {}",
                        String::from_utf8_lossy(&log)
                    )))
                } else {
                    plan.publish()
                };
                recording_finished(&status, &mut plan, result);
            });
        let thread = match thread {
            Ok(thread) => thread,
            Err(error) => {
                let mut state = lock(&self.status)?;
                state.active = false;
                state.error = Some(error.to_string());
                return Err(error.into());
            }
        };
        *worker = Some(RecordingWorker { stop, thread });
        Ok(initial)
    }
    pub fn stop(&self, id: &str) -> CoreResult<RecordingStatus> {
        let mut worker = lock(&self.worker)?;
        let current = self.status()?;
        if current.active && current.id != id {
            return Err(invalid("this recording is no longer active"));
        }
        if let Some(worker) = worker.take() {
            worker.stop.store(true, Ordering::Release);
            worker
                .thread
                .join()
                .map_err(|_| invalid("recording worker failed"))?;
        }
        self.status()
    }
    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::Release);
        // Startup and shutdown serialize on the worker slot. Never use a status ID
        // here: an in-flight start can publish a newer ID before this lock is held.
        // Cleanup must also work when the separate status/worker mutex was poisoned.
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(worker) = worker {
            worker.stop.store(true, Ordering::Release);
            let _ = worker.thread.join();
        }
    }
}
impl Drop for Recorder {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_recording_survives_unavailable_status_reporting() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("recording.wav");
        let mut plan = OutputPlan::new(&output, "wav").unwrap();
        let partial = plan.staging_file().to_owned();
        std::fs::write(&partial, b"recoverable recording fixture").unwrap();
        let status = Arc::new(Mutex::new(RecordingStatus::default()));
        let poison = status.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poison.lock().unwrap();
            panic!("simulated unavailable status");
        })
        .join();
        recording_finished(&status, &mut plan, Err(invalid("simulated provider error")));
        drop(plan);
        assert_eq!(
            std::fs::read(&partial).unwrap(),
            b"recoverable recording fixture"
        );
    }
    #[cfg(windows)]
    #[test]
    fn output_plan_rejects_ntfs_streams_without_starting_a_provider() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("existing.bin");
        std::fs::write(&base, b"unchanged fixture").unwrap();
        assert!(OutputPlan::new(&dir.path().join("existing.bin:output.mp4"), "mp4").is_err());
        assert_eq!(std::fs::read(base).unwrap(), b"unchanged fixture");
    }
    #[test]
    fn shutdown_stops_worker_even_when_status_is_unavailable() {
        let recorder = Recorder::default();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let thread = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        *recorder.worker.lock().unwrap() = Some(RecordingWorker {
            stop: stop.clone(),
            thread,
        });
        let status = recorder.status.clone();
        let _ = std::thread::spawn(move || {
            let _guard = status.lock().unwrap();
            panic!("simulated unavailable status");
        })
        .join();
        recorder.shutdown();
        let was_stopped = stop.load(Ordering::Acquire);
        // Always release the synthetic worker, including during the expected red stage.
        stop.store(true, Ordering::Release);
        if let Some(worker) = recorder.worker.lock().unwrap().take() {
            worker.thread.join().unwrap();
        }
        assert!(
            was_stopped,
            "shutdown must stop the worker independently of the status snapshot"
        );
    }
    #[test]
    fn output_created_after_preflight_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("raced.wav");
        let plan = OutputPlan::new(&output, "wav").unwrap();
        std::fs::write(plan.staging_file(), b"provider output").unwrap();
        std::fs::write(&output, b"user file created after preflight").unwrap();
        assert!(plan.publish().is_err());
        assert_eq!(
            std::fs::read(output).unwrap(),
            b"user file created after preflight"
        );
    }
    #[test]
    fn shutdown_prevents_future_recording_without_starting_a_provider() {
        let recorder = Recorder::default();
        recorder.shutdown();
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("must-not-record.wav");
        assert_eq!(
            recorder
                .start("microphone", Vec::new(), &output, "wav")
                .unwrap_err()
                .code(),
            "cancelled"
        );
        assert!(!output.exists());
        assert!(!recorder.status().unwrap().active);
    }
    #[test]
    fn output_preflight_never_replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output.mp4");
        std::fs::write(&output, b"keep").unwrap();
        assert!(OutputPlan::new(&output, "mp4").is_err());
        assert_eq!(std::fs::read(output).unwrap(), b"keep");
    }
    #[test]
    fn completed_provider_output_publishes_exclusively() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("new.wav");
        let plan = OutputPlan::new(&output, "wav").unwrap();
        std::fs::write(plan.staging_file(), b"WAV fixture").unwrap();
        assert_eq!(
            plan.publish().unwrap().canonicalize().unwrap(),
            output.canonicalize().unwrap()
        );
        assert_eq!(std::fs::read(output).unwrap(), b"WAV fixture");
    }
}
