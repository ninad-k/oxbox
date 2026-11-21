# ADR 0002: Cancellable jobs and explicit background ownership

- Status: Accepted
- Date: 2026-11-21

## Decision

Use core-owned `CancelToken` and `Progress` types, with a Tauri-managed job registry and a typed UI helper.
The frontend chooses a fresh ID and subscribes before invoking. Events include the ID and are throttled to
10 Hz. Cancellation arriving before worker registration is retained in a bounded queue. Limit active work to
eight jobs. Unregister both token and event listener when work finishes.

Background power, clipboard, and capture services have explicit initialize/shutdown hooks. Awake guards that
require thread affinity are created and dropped on their own worker thread. App Quit cancels jobs, closes
capture overlays, stops the clipboard watcher, cancels sleep actions, releases awake guards, and restores icons.

## Consequences

Core logic stays testable without a desktop host. UI navigation cannot silently terminate persistent timers or
leave stray polling loops. Cancellation is cooperative and may leave completed outputs or a new partial
extraction folder; it does not roll back a completed OS action.
