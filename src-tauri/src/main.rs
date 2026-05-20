#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn normalize_ime_env_for_wayland() {
    let is_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);

    if !is_wayland {
        return;
    }

    let gtk_im = std::env::var("GTK_IM_MODULE")
        .ok()
        .map(|value| value.to_ascii_lowercase());

    if matches!(gtk_im.as_deref(), Some("fcitx") | Some("ibus")) {
        // Keep the app on native Wayland and avoid forcing a legacy GTK IM module.
        unsafe {
            std::env::remove_var("GTK_IM_MODULE");
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    normalize_ime_env_for_wayland();
    ohmywu_lib::run()
}
