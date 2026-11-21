//! One module per tool. The module name is the tool id in `snake_case`
//! (tool id `verify-checksum` → module `checksum` is fine too; keep it obvious).
//!
//! Reference implementations to copy when adding a tool:
//! - [`text_transform`]: a pure, synchronous tool (no I/O).
//! - [`checksum`]: a file tool that streams data and runs on a blocking thread.

pub mod airplane_mode;
pub mod alarm;
pub mod archive;
pub mod barcode_reader;
pub mod break_time;
pub mod checksum;
pub mod clean_drive;
pub mod clipboard_history;
pub mod close_apps;
pub mod color_picker;
pub mod convert_media;
pub mod cpu_temperature;
pub mod currency;
pub mod date_countdown;
pub mod do_not_disturb;
pub mod duplicates;
pub mod eject_volumes;
pub mod encrypt;
pub mod energy_saver;
pub mod focus_window;
pub mod hidden_files;
pub mod hide_desktop;
pub mod keep_awake;
pub mod launch_apps;
pub mod lock;
pub mod make_gif;
pub mod media_provider;
pub mod mouse_activity;
pub mod mute_microphone;
pub mod network;
pub mod ocr;
pub mod presentation_mode;
pub mod qr;
pub mod record_audio;
pub mod record_screen;
pub mod resize;
pub mod resolution;
pub mod screenshot;
pub mod show_desktop;
pub mod sleep_timer;
pub mod system_monitor;
pub mod take_photo;
pub mod take_video;
pub mod text_transform;
pub mod timer_stopwatch;
pub mod uninstall;
pub mod unit_convert;
pub mod window_manager;
pub mod window_top;
pub mod world_time;
