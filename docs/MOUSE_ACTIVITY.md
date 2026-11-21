# Mouse Activity

Mouse Activity is an optional Windows tool for gentle pointer activity during an explicitly requested session.
Open the tool, choose an idle delay and movement interval, optionally set a duration, then select **Start**.
It starts stopped each time Oxbox launches. It has no schedule, login activation or saved running state.

The tool waits for the full idle delay after Start or real keyboard/mouse input. While idle, it sends a small
pointer-only movement at the selected interval. It alternates direction to avoid drifting across the screen.
Real input suspends subsequent movements and restarts the idle delay. Its own synthetic events do not count
as real input. It never clicks, types or scrolls. It does not request administrator rights or bypass protected
desktops. Held input or unavailable desktop/input access prevents movement.

**Waiting for idle** means the session is enabled but is not moving the pointer. **Paused by input** means
new input restarted the idle delay. **Simulating while idle**
means the idle threshold was met. Stop is available in the tool, the launcher banner and the tray menu.
The optional duration uses a monotonic clock and expires even if the launcher is hidden. Stop, expiry,
native errors and Quit end the session and release its input observers. macOS and Linux are explicitly
unsupported in this MVP.

Stop invalidates earlier Start requests even if their background work has not begun. It waits for the
session's worker to finish. A later explicit Start is a new request; a delayed older Stop cannot terminate
that newer session. The tray records Stop at the native click callback before asynchronous scheduling.
Launcher actions first reserve their request through a synchronous IPC command, so a deferred asynchronous
Start keeps its original place in that order. Tokens are bounded, single-use, action-bound strings; stale,
unknown, replayed or malformed tokens cannot change the running session.

Mouse Activity observes only event timing in memory. It does not retain key contents, text, input history
or pointer trails and makes no network requests. Input detection and injection remain subject to Windows
desktop/session permissions; an error stops the session rather than escalating privileges. This tool does
not guarantee a particular sleep, lock-screen or application presence policy.

Windows low-level hooks provide the immediate input signal. A separate session-input timer conservatively
restarts waiting on uncertain external activity if a hook is delayed or removed. The tool excludes its own
tagged synthetic movement from idle timing; accessibility/remote-control activity may conservatively pause
the session. Input can race with a movement already submitted to Windows; no stale pointer position is
restored afterward.

Native API references: [low-level input hook requirements](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc),
[absolute virtual-desktop coordinates](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-mouseinput),
and [SendInput permissions and return value](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput).

## Contributor checks

The module's automated tests use fake input and clock backends. They must never move the test runner's
pointer. Before claiming Windows native acceptance, coordinate a disposable session with the tester and
record the OS, idle/interval settings and results: full idle delay, real keyboard/mouse interruption,
duration expiry, held input, multi-monitor edges, banner/tray Stop, Quit and restart stopped. Check unsupported
platform labeling separately. See [the task specification](../tasks/mouse-activity.md).

The focused suite includes 17 tests on Windows: idle/interval/expiry logic, bounded motion/event flags,
fake backend cleanup and errors, queued request ordering and token validation. These automated results do not establish
native input or pointer-movement acceptance.
