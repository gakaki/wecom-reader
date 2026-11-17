//! Runs the same Rust root on the machine's native desktop backend.
//! `nsh run` / `nsh package` use this for macOS, Windows and Linux;
//! `cargo run --bin gpui` uses the GPUI Kit host instead.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<wecom_reader::App>()),
        platform::AppOptions {
            title: "WeCom Reader".to_owned(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: "com.gakaki.linuxdo".to_owned(),
            size: (1180, 760),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn main() {}
