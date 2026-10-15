//! Diagnostic-only hooks in the actual iced_winit runtime, not a replacement host.
#[path = "../frame_latency_profile/presentation.rs"]
mod presentation;
use presentation::Presentation;
use std::{cell::RefCell, sync::Arc};
use winit::{event::WindowEvent, window::Window};
thread_local! {
    static PROBE: RefCell<Option<(Presentation, u64)>> = const { RefCell::new(None) };
}
pub fn drain() {
    PROBE.with(|probe| {
        if let Some((probe, _)) = &mut *probe.borrow_mut() {
            for feedback in probe.drain() {
                iced_debug::diagnostic_record(
                    if feedback.presented_ns.is_some() {
                        "presented"
                    } else {
                        "discarded"
                    },
                    feedback.frame,
                    feedback.presented_ns.unwrap_or(0),
                    feedback.refresh_ns as u64,
                );
            }
        }
    });
}
pub fn request(window: Arc<Window>) {
    PROBE.with(|state| {
        let mut state = state.borrow_mut();
        let (probe, frame) = state.get_or_insert_with(|| {
            let probe = Presentation::new(window.clone());
            assert_eq!(
                probe.clock_id(),
                libc::CLOCK_MONOTONIC as u32,
                "unmatched presentation clock"
            );
            iced_debug::diagnostic_record("clock_id", 0, probe.clock_id() as u64, 0);
            (probe, 0)
        });
        let size = window.inner_size();
        iced_debug::diagnostic_record(
            "physical_size",
            *frame,
            size.width as u64,
            size.height as u64,
        );
        iced_debug::diagnostic_record(
            "scale",
            *frame,
            window.scale_factor().to_bits(),
            window
                .current_monitor()
                .and_then(|m| m.refresh_rate_millihertz())
                .unwrap_or(0) as u64,
        );
        iced_debug::diagnostic_record("submit", *frame, 0, 0);
        probe.request(*frame);
        *frame += 1;
        if std::env::var_os("FEATHER_RUNTIME_SMOKE").is_some() && *frame == 6 {
            iced_debug::diagnostic_flush();
            std::process::exit(0);
        }
    });
}
pub fn input(event: &WindowEvent) {
    drain();
    let stage = match event {
        WindowEvent::CursorMoved { .. } => "input_move",
        WindowEvent::MouseInput {
            state: winit::event::ElementState::Pressed,
            ..
        } => "input_down",
        WindowEvent::MouseInput { .. } => "input_up",
        WindowEvent::MouseWheel { .. } => "input_wheel",
        WindowEvent::Resized(_) => "resize",
        WindowEvent::ScaleFactorChanged { .. } => "rescale",
        WindowEvent::Focused(_) => "focus",
        WindowEvent::CloseRequested => "close",
        _ => return,
    };
    let (a, b) = match event {
        WindowEvent::CursorMoved { position, .. } => (position.x.to_bits(), position.y.to_bits()),
        WindowEvent::Resized(size) => (size.width as u64, size.height as u64),
        _ => (0, 0),
    };
    iced_debug::diagnostic_record(stage, 0, a, b);
    if matches!(event, WindowEvent::CloseRequested) {
        // Release our window owner before Iced waits for native window destruction.
        PROBE.with(|probe| {
            probe.borrow_mut().take();
        });
        iced_debug::diagnostic_flush();
    }
}
