# Contributor architecture guide

The launcher is a SvelteKit SPA embedded by Tauri 2. `crates/oxbox-core` owns computation, filesystem rules,
native platform behavior, and testable scheduler models. It has no Tauri dependency. `src-tauri` adapts core
results to serializable IPC, manages app-lifetime services, notifications, tray controls, and capture windows.

## Tool path

1. A `ToolDef` in `src/lib/registry.ts` supplies the stable ID, search terms, platform list, and lazy view import.
2. The Svelte view uses typed API wrappers, shared controls, and CSS tokens.
3. `src/lib/api.ts` is the public facade; `file-api.ts`, `system-api.ts`, and `media-api.ts` group related types
   and wrappers. `ipc.ts` owns transport and `ToolError`, avoiding module cycles.
4. The handler registered in `src-tauri/src/lib.rs` performs boundary validation and delegates to core.
   `commands.rs` holds the original adapters; `app_commands.rs`, `file_commands.rs`, `system_commands.rs`,
   and `media_commands.rs` keep the larger catalogue readable. Promoted roadmap tools use `roadmap_*_commands.rs`
   and matching typed API groups; `currency_commands.rs` adapts the isolated rate cache.
5. Core returns `CoreResult<T>` and stable error codes. Disk/CPU work runs on Tauri's blocking pool.

Tool views never call `invoke` directly. Add wrappers to the appropriate group and expose them through `api`.
Use a `requestId` or disposed flag to ignore results after input changes or unmounting. Polling views stop their
timers on unmount and avoid overlapping native calls.

## Background ownership

| Owner | Lifetime and cleanup |
|---|---|
| `SystemTools` | App-managed; scheduler worker checks sleep deadlines and break reminders, owns power state; joins on Quit |
| Core awake controller | Dedicated worker creates and drops thread-affine awake guard on the same thread |
| `MediaState` | App-managed clipboard service; starts paused, polls while enabled; joins and clears memory on Quit |
| `AreaCaptureState` | Frozen monitor frames and overlay windows; selection/cancel/error closes overlays; Quit cleans up |
| `JobState` | Active cancellation tokens, bounded early-cancel registry; Quit cancels outstanding jobs |
| `clocks.svelte.ts` | Launcher-lifetime monotonic countdown/stopwatch service, survives tool navigation |
| Settings store | Local `settings.json`; preferences, shortcut, and breakTime use distinct keys |
| Roadmap system state | Window/topmost/minimize ownership, focus overlays and reversible display modes |
| Roadmap time state | Persisted alarms/countdowns; independent presentation leases and display auto-start |
| Roadmap media state | One idle-by-default recording process; explicit start, stop/finalization and Quit cleanup |
| Currency cache | Separate `currency.json`, serialized access, manual-save rollback, default-offline opt-in |

## Long-running jobs

The UI calls `createJob(toolId, onProgress)` before invoking a job command. It installs an event listener first
and passes a fresh job ID. The Tauri runner registers a `CancelToken`, throttles progress to at most ten events
per second, and unregisters on completion/error. Events are correlated by `jobId`. Cancellation requested before
registration is remembered in a bounded queue. At most eight jobs run concurrently.

Core receives a cloneable token and `&mut dyn FnMut(Progress)`, without knowing about Tauri. Check the token in
each traversal and streaming loop. Do not emit user contents or secrets in progress messages. UI listeners are
removed in `finally`; unmounting cancels the associated job.

## Security boundaries

File outputs use exclusive creation. Archive destinations are new folders; entries are validated independently
of host path syntax. Cleanup accepts an internal category allow-list, rechecks paths, and enforces the reviewed
file/byte budget. Root, home, system, and symlink/reparse paths are protected by the shared filesystem guard.
Outputs may remain partially complete after cancellation where removing them could hide useful diagnostic
evidence; originals are preserved.

All shell commands use an executable and argument array. Native Windows calls are contained in platform modules
or the tool's native adapter, with safety comments. A missing OS capability returns an error or an unavailable
registry state. Runtime features are offline by default. The sole optional network exception is the fixed ECB
currency download after explicit opt-in ([issue #1](https://github.com/ninad-k/oxbox/issues/1)); local amounts and
user content are never uploaded. Development downloads and CI are separate from runtime behavior.

See the [ADRs](adr/), [testing guide](TESTING.md), and [tool recipe](../AGENTS.md) before extending the project.
