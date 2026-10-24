// SPDX-License-Identifier: GPL-3.0-or-later
//
// The app is Wayland only: without a Wayland display it says so instead of trying X11.

/// Whether the session has a Wayland display for the window.
pub fn has_wayland() -> bool {
    let named = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    is_wayland(named("WAYLAND_DISPLAY"), named("WAYLAND_SOCKET"))
}

/// winit's rule, with an empty variable counting as unset.
fn is_wayland(wayland_display: bool, wayland_socket: bool) -> bool {
    wayland_display || wayland_socket
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_needs_a_wayland_display() {
        assert!(is_wayland(true, false));
        assert!(is_wayland(false, true));
        assert!(is_wayland(true, true));
        // `DISPLAY` alone (X11) isn't enough.
        assert!(!is_wayland(false, false));
    }
}
