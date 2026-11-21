# Mouse Activity (`mouse-activity`)

Status: source implemented; Windows native acceptance pending.

The user authorized an original idle-aware mouse utility on 2026-11-21. Only the general concept of an
idle-aware activity utility is used; no external project implementation, name, icon or UI is copied.

## Behavior

- Windows MVP. macOS/Linux return an explicit Unsupported error. No new dependencies, service, elevation or network.
- Starts stopped after every launch. Starting is an explicit user action; no saved active state or autostart.
- Wait a full configurable idle period (30–3600 seconds) after Start or genuine input. Move by one bounded
  pixel only while idle, at a configurable interval (5–300 seconds). Alternate direction to avoid drift.
- Observe mouse and keyboard input without recording keys, button contents or positions in logs/storage.
  Own synthetic input must not restart genuine-idle timing. Genuine input suspends future simulated moves.
- Skip movement while input is held, the input desktop is inaccessible, or the cursor position changes
  during a movement decision. Never click, type, scroll, restore an old cursor position or request privileges.
- Optional duration cap: 1–1440 minutes, based on a monotonic clock. No cap means explicit Stop/Quit.
- Show Stopped, Waiting for idle, Paused by input, and Simulating while idle states, with visible Stop controls in the tool,
  launcher banner and tray. Errors stop the session; quitting cancels and joins the worker before returning.

## Architecture and acceptance

Core owns validation, scheduling, fake backend tests and the Windows backend. A thin Tauri module exposes
Start/Stop/Status; the view uses a typed API and shared design tokens. Hooks and native resources exist only
during a requested session and are released on Stop, expiry, error and Quit.

Automated tests must use a fake clock/input backend and never move the developer's pointer. Cover full idle
delay, interval limits, synthetic-input exclusion, genuine input suspension, bounded motion at screen edges,
duration expiry, input-read/injection failures, Stop acknowledgement and Quit registration races.
The same core request service used by commands is tested with reordered Start/Stop requests and a fake
initialization barrier: acknowledged Stop rejects older queued starts, a delayed older Stop leaves a newer
explicit session alone, and Stop during initialization cleans up the stale worker before acknowledgement.
Synchronous IPC reservations precede asynchronous operation dispatch. Tokens travel as strings to avoid
JavaScript integer rounding and are validated as issued, single-use and action-bound before any mutation;
only 64 unclaimed reservations are retained. Unknown/future/replayed/wrong-action tokens are regression-tested.

Windows manual acceptance remains separate: explicit Start, real keyboard/mouse interruption, visible status
while the launcher is hidden, timed stop, tray Stop, Quit/restart stopped, multi-monitor edges, held buttons,
locked/elevated desktop refusal. Coordinate pointer-moving checks with the user; do not run them silently.
