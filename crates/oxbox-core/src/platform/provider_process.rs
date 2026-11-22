//! Owned provider process trees. Containment is established before user code runs.
use crate::CoreResult;
use std::{
    io,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus},
};

pub(super) struct ManagedChild {
    inner: Child,
    #[cfg(windows)]
    job: Option<windows::Job>,
    #[cfg(unix)]
    group: Option<libc::pid_t>,
    #[cfg(unix)]
    status: Option<ExitStatus>,
}
impl ManagedChild {
    pub(super) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.inner.stdout.take()
    }
    pub(super) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.inner.stderr.take()
    }
    pub(super) fn stdin_mut(&mut self) -> Option<&mut ChildStdin> {
        self.inner.stdin.as_mut()
    }
    pub(super) fn spawn(command: &mut Command) -> CoreResult<Self> {
        #[cfg(windows)]
        {
            use std::os::windows::{io::AsRawHandle, process::CommandExt};
            use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
            let job = windows::Job::new().map_err(containment_error)?;
            // std's spawn_with_attributes is unstable in Rust 1.95. A suspended
            // primary thread cannot run provider code or spawn descendants before
            // assignment. Every failure drops the guard and kills/reaps the child.
            command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
            let child = Self {
                inner: command.spawn()?,
                job: Some(job),
            };
            let job = child.job.as_ref().ok_or_else(|| {
                containment_error(io::Error::other("provider job is unavailable"))
            })?;
            job.assign(child.inner.as_raw_handle())
                .map_err(containment_error)?;
            windows::resume_owned_primary(child.inner.id()).map_err(containment_error)?;
            Ok(child)
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // setpgid runs between fork and exec, before provider code can run.
            command.process_group(0);
            let mut inner = command.spawn()?;
            let group = match libc::pid_t::try_from(inner.id()) {
                Ok(group) if group > 0 => group,
                _ => {
                    let _ = inner.kill();
                    let _ = inner.wait();
                    return Err(
                        io::Error::other("provider process identity is out of range").into(),
                    );
                }
            };
            Ok(Self {
                inner,
                group: Some(group),
                status: None,
            })
        }
    }
    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        #[cfg(unix)]
        {
            if let Some(status) = self.status {
                return Ok(Some(status));
            }
            // Keep the leader waitable until its group has been signalled. Reaping
            // it first would release its PID/PGID and permit an unrelated group
            // to reuse that numeric identity before kill(-pgid).
            // SAFETY: siginfo_t is a C value whose zero representation is valid;
            // POSIX requires si_pid zeroed to detect a WNOHANG no-event result.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            loop {
                // SAFETY: this is our live child; typed output storage is valid.
                let result = unsafe {
                    libc::waitid(
                        libc::P_PID,
                        self.inner.id() as libc::id_t,
                        &mut info,
                        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                    )
                };
                if result == 0 {
                    break;
                }
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
            // SAFETY: waitid initialized siginfo_t; si_pid is the documented field
            // used to distinguish WNOHANG's no-event result from a child exit.
            if unsafe { info.si_pid() } == 0 {
                return Ok(None);
            }
            self.stop_group()?;
            let status = self.inner.wait()?;
            self.status = Some(status);
            Ok(Some(status))
        }
        #[cfg(windows)]
        {
            let result = self.inner.try_wait()?;
            if result.is_some() {
                // A successful wrapper may leave descendants holding stdout/stderr.
                // Close the tree before the caller joins any pipe-reading threads.
                self.close_tree();
            }
            Ok(result)
        }
    }
    fn close_tree(&mut self) {
        #[cfg(windows)]
        drop(self.job.take());
        #[cfg(unix)]
        let _ = self.stop_group();
    }
    #[cfg(unix)]
    fn stop_group(&mut self) -> io::Result<()> {
        if let Some(group) = self.group.take() {
            // SAFETY: group is positive and belongs to our unreaped child. No
            // caller can access Child::wait directly through this wrapper.
            while unsafe { libc::kill(-group, libc::SIGKILL) } != 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
                break;
            }
        }
        Ok(())
    }
    pub(super) fn terminate(&mut self) {
        self.close_tree();
        if !self.inner.try_wait().is_ok_and(|status| status.is_some()) {
            let _ = self.inner.kill();
        }
        let _ = self.inner.wait();
    }
}
#[cfg(windows)]
fn containment_error(error: io::Error) -> crate::CoreError {
    crate::CoreError::InvalidInput(format!(
        "the provider could not be contained before execution: {error}"
    ))
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(windows)]
mod windows {
    use std::{
        io,
        mem::{size_of, zeroed},
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle},
        ptr::null,
    };
    use windows_sys::Win32::{
        Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            Threading::{
                GetProcessIdOfThread, OpenThread, ResumeThread, THREAD_QUERY_LIMITED_INFORMATION,
                THREAD_SUSPEND_RESUME,
            },
        },
    };
    pub(super) struct Job(OwnedHandle);
    impl Job {
        pub(super) fn new() -> io::Result<Self> {
            // SAFETY: anonymous, non-inheritable job; no borrowed pointers persist.
            let handle = unsafe { CreateJobObjectW(null(), null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: this call returned a new owned kernel handle.
            let owned = unsafe { OwnedHandle::from_raw_handle(handle) };
            // SAFETY: all-zero limit fields are valid before setting LimitFlags.
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            // SAFETY: the typed struct and handle remain alive throughout the call.
            if unsafe {
                SetInformationJobObject(
                    owned.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    std::ptr::from_ref(&limits).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(owned))
        }
        pub(super) fn assign(&self, child: RawHandle) -> io::Result<()> {
            // SAFETY: both handles are live; child is still CREATE_SUSPENDED.
            if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child) } == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
    pub(super) fn resume_owned_primary(process: u32) -> io::Result<()> {
        // SAFETY: snapshot creation has no borrowed arguments.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful snapshot is a newly owned handle.
        let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };
        // SAFETY: THREADENTRY32 accepts zeroed fields plus the documented size.
        let mut entry: THREADENTRY32 = unsafe { zeroed() };
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        let mut thread = None;
        // SAFETY: snapshot and output storage are valid for each enumeration call.
        let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } != 0;
        while found {
            if entry.th32OwnerProcessID == process && thread.replace(entry.th32ThreadID).is_some() {
                return Err(io::Error::other(
                    "suspended provider has an ambiguous primary thread",
                ));
            }
            found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } != 0;
        }
        if io::Error::last_os_error().raw_os_error() != Some(ERROR_NO_MORE_FILES as i32) {
            return Err(io::Error::last_os_error());
        }
        let thread = thread
            .ok_or_else(|| io::Error::other("suspended provider primary thread was not found"))?;
        // SAFETY: no inherited handle; identity is verified again before resuming.
        let handle = unsafe {
            OpenThread(
                THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                0,
                thread,
            )
        };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: OpenThread returned a newly owned handle.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        if unsafe { GetProcessIdOfThread(handle.as_raw_handle()) } != process {
            return Err(io::Error::other(
                "provider thread identity changed before resume",
            ));
        }
        // SAFETY: this uniquely identified thread belongs to our suspended child,
        // which is already a member of our non-breakaway kill-on-close job.
        if unsafe { ResumeThread(handle.as_raw_handle()) } != 1 {
            return Err(io::Error::other(
                "provider primary thread was not exclusively suspended",
            ));
        }
        Ok(())
    }
}
