# Phase 5: presentation, OS controls, and scheduled reminders

These tasks promote the corresponding roadmap entries into reviewable features. Implementation is present;
manual OS checks remain separate from automated logic validation.

| Tool | Implemented behavior | Platform and safety |
|---|---|---|
| Presentation Mode | Independent system/display awake request; optional desktop-icon hide; opt-in auto-start with an additional display; manual stop suppresses auto-restart until disconnect | Awake requests are created/dropped on the same worker thread. Desktop visibility restores its original state. Existing Do Not Sleep requests remain separate. Linux users leave icon hiding off. Quit joins the worker and restores owned state. |
| Do Not Disturb | Opens notification settings on Windows, System Settings on macOS, and explains the manual control | State is explicitly unknown. No undocumented registry mutation, private Focus API, or fabricated toggle success. Linux users use their desktop's controls. |
| Airplane Mode | Opens Windows Airplane mode settings | Hardware and OS decide whether radios can be switched. Oxbox neither reads nor changes radio state and never disconnects automatically. |
| Alarm | One-time, daily, weekday alarms; explicit IANA zone; explicit first/second DST-fold occurrence; snooze, dismiss, enable/disable, confirmed removal | Native scheduler runs with a hidden launcher. Persist before notification; missed alarms coalesce into one delivery. One-time DST gaps are rejected; recurring gap dates are skipped. Notifications respect OS permissions and DND. |
| Date Countdown | Named future/past dates, explicit time zone, signed remaining time, optional once-only notification, confirmed removal | Native notification timing; saved records survive restart. Past dates can be tracked without generating stale notifications when added. |

Storage uses the `roadmapTime` key in the existing local `settings.json` store. Each list is capped at 100 items.
Malformed reminder storage is preserved and reported; editing is blocked until the file is backed up/repaired.
No feature calls a remote service. Disabling radios and changing DND remain deliberate user actions in OS controls.

Presentation auto-start counts connected displays; it does not identify their physical connection or infer that a
particular display is a projector. It is off by default. A change to manually configured DND is not restored by Oxbox.

## Verification

- Fake-clock tests cover alarm delivery, snooze, dismissal, weekday rollover, and DST gaps/folds.
- Countdown tests cover positive/negative remaining time and once-only delivery.
- Injected presentation backend tests cover partial-start rollback, exact visibility restoration, and monitor auto-start suppression.
- OS-control tests assert documented Windows URI selection and unknown/manual state.
- UI unit tests cover signed countdown formatting; `pnpm check` validates all views.
- Pending manual checks: Windows presentation start/stop and external-display connect/disconnect, hidden-window alarm delivery,
  native settings pages, restart persistence, and macOS/Linux behavior. No OS effects are invoked by automated tests.

## Source references

- [Microsoft: Launch Windows Settings](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings)
  documents `ms-settings:notifications`, `ms-settings:quiethours`, and `ms-settings:network-airplanemode`.
- [chrono TimeZone](https://docs.rs/chrono/latest/chrono/trait.TimeZone.html) and
  [LocalResult](https://docs.rs/chrono/latest/chrono/offset/enum.LocalResult.html) define local-time gaps and ambiguous instants.
- [Microsoft: SetThreadExecutionState](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate)
  describes thread-specific awake requirements; Oxbox keeps each guard on its owning thread.
