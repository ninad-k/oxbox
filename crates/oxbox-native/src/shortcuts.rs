//! Native keyboard bindings consume only the intended event modifiers.
use eframe::egui::{Context, Event, Key, Modifiers};

pub fn consume(ctx: &Context, modifiers: Modifiers, key: Key) -> bool {
    ctx.input_mut(|input| {
        let mut consumed = false;
        input.events.retain(|event| {
            let matches = matches!(event, Event::Key {
                key: event_key,
                pressed: true,
                modifiers: event_modifiers,
                ..
            } if *event_key == key && event_modifiers.matches_exact(modifiers));
            consumed |= matches;
            !matches
        });
        consumed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, RawInput};

    fn key(modifiers: Modifiers, key: Key) -> Event {
        Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    fn run(mut events: Vec<Event>, current: Modifiers, check: impl FnOnce(&Context)) {
        let ctx = Context::default();
        let mut check = Some(check);
        events.push(Event::ModifiersChanged(current));
        let mut output = ctx.run_ui(
            RawInput {
                events,
                ..Default::default()
            },
            |_| check.take().unwrap()(&ctx),
        );
        output.textures_delta.clear();
    }
    fn key_count(ctx: &Context) -> usize {
        ctx.input(|input| {
            input
                .events
                .iter()
                .filter(|event| matches!(event, Event::Key { .. }))
                .count()
        })
    }
    #[test]
    fn rejects_extra_modifiers_for_quit_favorites_and_timer_keys() {
        for (required, extra, key) in [
            (Modifiers::CTRL, Modifiers::SHIFT, Key::Q),
            (Modifiers::CTRL, Modifiers::ALT, Key::Q),
            (Modifiers::ALT, Modifiers::SHIFT, Key::Num1),
            (Modifiers::ALT, Modifiers::CTRL, Key::Num1),
            (Modifiers::NONE, Modifiers::ALT, Key::Space),
            (Modifiers::NONE, Modifiers::SHIFT, Key::R),
        ] {
            run(
                vec![self::key(required | extra, key)],
                required | extra,
                |ctx| {
                    assert!(!consume(ctx, required, key), "unexpected modified {key:?}");
                    assert_eq!(key_count(ctx), 1);
                },
            );
        }
    }
    #[test]
    fn uses_each_event_modifiers_instead_of_the_end_of_frame_state() {
        run(vec![key(Modifiers::CTRL, Key::K)], Modifiers::NONE, |ctx| {
            assert!(consume(ctx, Modifiers::CTRL, Key::K));
            assert!(!consume(ctx, Modifiers::CTRL, Key::K));
        });
        run(
            vec![key(Modifiers::CTRL | Modifiers::SHIFT, Key::Q)],
            Modifiers::CTRL,
            |ctx| assert!(!consume(ctx, Modifiers::CTRL, Key::Q)),
        );
    }
    #[test]
    fn consumes_only_exact_matches_and_preserves_other_events() {
        run(
            vec![
                key(Modifiers::CTRL | Modifiers::SHIFT, Key::Q),
                key(Modifiers::CTRL, Key::Q),
                key(Modifiers::NONE, Key::Escape),
            ],
            Modifiers::NONE,
            |ctx| {
                assert!(consume(ctx, Modifiers::CTRL, Key::Q));
                assert_eq!(key_count(ctx), 2);
                assert!(consume(ctx, Modifiers::NONE, Key::Escape));
                assert_eq!(key_count(ctx), 1);
            },
        );
    }
    #[test]
    fn accepts_platform_command_alias_without_accepting_other_physical_modifiers() {
        for modifiers in [Modifiers::CTRL, Modifiers::MAC_CMD] {
            run(
                vec![key(modifiers | Modifiers::COMMAND, Key::K)],
                modifiers | Modifiers::COMMAND,
                |ctx| assert!(consume(ctx, modifiers, Key::K)),
            );
            run(
                vec![key(modifiers | Modifiers::COMMAND | Modifiers::ALT, Key::Q)],
                modifiers | Modifiers::COMMAND | Modifiers::ALT,
                |ctx| assert!(!consume(ctx, modifiers, Key::Q)),
            );
        }
    }
}
