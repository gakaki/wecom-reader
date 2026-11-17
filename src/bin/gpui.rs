//! GPUI Kit mode: the pure-Rust NSH app runs inside a GPUI Fast window.
//! This is the desktop host requested by the project; no TSX entry exists.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use nsh_platform_gpui::{AppOptions, AppSource, Root, run_app, set_ui_sets};

fn main() -> std::process::ExitCode {
    env_logger::init();
    set_ui_sets(vec![nsh_base::registry(), wecom_reader::registry()]);

    run_app(
        AppSource::Rust(Root::of::<wecom_reader::App>()),
        AppOptions {
            title: "WeCom Reader · GPUI".to_owned(),
            app_id: "com.gakaki.linuxdo.gpui".to_owned(),
            size: (1180, 760),
            ..Default::default()
        },
    )
}
