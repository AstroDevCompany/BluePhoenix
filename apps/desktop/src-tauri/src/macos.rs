#![cfg(target_os = "macos")]

use objc2_app_kit::{NSWindow, NSWindowButton};
use objc2_foundation::NSPoint;
use tauri::{WebviewWindow, WindowEvent};

/// Must match `--titlebar-h` in `apps/desktop/src/styles/app.css`.
const TITLEBAR_HEIGHT: f64 = 40.0;
const TRAFFIC_LIGHT_X: f64 = 16.0;

pub fn install(window: &WebviewWindow) {
    center(window);
    let win = window.clone();
    window.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
        ) {
            return;
        }
        center(&win);
    });
}

fn center(window: &WebviewWindow) {
    let Ok(ptr) = window.ns_window() else {
        return;
    };
    if ptr.is_null() {
        return;
    }
    unsafe { apply(&*(ptr as *const NSWindow)) };
}

unsafe fn apply(ns_window: &NSWindow) {
    let Some(close) = ns_window.standardWindowButton(NSWindowButton::CloseButton) else {
        return;
    };
    let Some(miniaturize) = ns_window.standardWindowButton(NSWindowButton::MiniaturizeButton)
    else {
        return;
    };
    let Some(zoom) = ns_window.standardWindowButton(NSWindowButton::ZoomButton) else {
        return;
    };
    let Some(titlebar_view) = close.superview() else {
        return;
    };
    let Some(container) = titlebar_view.superview() else {
        return;
    };

    let button_height = close.frame().size.height;
    let gap = miniaturize.frame().origin.x - close.frame().origin.x;

    let mut container_frame = container.frame();
    container_frame.size.height = TITLEBAR_HEIGHT;
    container_frame.origin.y = ns_window.frame().size.height - TITLEBAR_HEIGHT;
    container.setFrame(container_frame);

    let mut titlebar_frame = titlebar_view.frame();
    titlebar_frame.size.height = TITLEBAR_HEIGHT;
    titlebar_frame.origin.y = 0.0;
    titlebar_view.setFrame(titlebar_frame);

    let y = ((TITLEBAR_HEIGHT - button_height) / 2.0).max(0.0);
    for (index, button) in [close, miniaturize, zoom].iter().enumerate() {
        button.setFrameOrigin(NSPoint::new(TRAFFIC_LIGHT_X + index as f64 * gap, y));
    }
}
