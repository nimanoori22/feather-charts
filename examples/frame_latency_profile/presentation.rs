use std::sync::Arc;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::{Backend, ObjectId},
    protocol::{wl_registry, wl_surface::WlSurface},
};
use wayland_protocols::wp::presentation_time::client::{
    wp_presentation::{self, WpPresentation},
    wp_presentation_feedback::{self, WpPresentationFeedback},
};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle},
    window::Window,
};

#[derive(Debug)]
pub struct Feedback {
    pub frame: u64,
    pub presented_ns: Option<u64>,
    pub refresh_ns: u32,
    pub flags: u32,
}
#[derive(Default)]
struct State {
    presentation: Option<WpPresentation>,
    clock: Option<u32>,
    feedback: Vec<Feedback>,
}

// The last field keeps the foreign display/surface alive until all guest objects drop.
pub struct Presentation {
    queue: EventQueue<State>,
    state: State,
    surface: WlSurface,
    connection: Connection,
    _window: Arc<Window>,
}
impl Presentation {
    pub fn new(window: Arc<Window>) -> Self {
        let RawDisplayHandle::Wayland(display) = window.display_handle().unwrap().as_raw() else {
            panic!(
                "Wayland presentation feedback is required; no presentation estimate is substituted"
            );
        };
        let RawWindowHandle::Wayland(surface) = window.window_handle().unwrap().as_raw() else {
            panic!("Wayland surface is required");
        };
        // Winit uses the system backend. The retained Arc<Window> owns its WlDisplay.
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        // This pointer belongs to the same retained window/display, with wl_surface type.
        let id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }
                .unwrap();
        let surface = WlSurface::from_id(&connection, id).unwrap();
        let mut queue = connection.new_event_queue();
        let mut state = State::default();
        connection.display().get_registry(&queue.handle(), ());
        queue.roundtrip(&mut state).unwrap();
        queue.roundtrip(&mut state).unwrap();
        assert!(
            state.presentation.is_some(),
            "wp_presentation is unavailable"
        );
        assert!(state.clock.is_some(), "presentation clock is unavailable");
        Self {
            queue,
            state,
            surface,
            connection,
            _window: window,
        }
    }
    pub fn clock_id(&self) -> u32 {
        self.state.clock.unwrap()
    }
    pub fn now_ns(&self) -> u64 {
        clock_ns(self.clock_id() as libc::clockid_t)
    }
    pub fn request(&self, frame: u64) {
        self.state.presentation.as_ref().unwrap().feedback(
            &self.surface,
            &self.queue.handle(),
            frame,
        );
        self.connection.flush().unwrap();
    }
    pub fn drain(&mut self) -> Vec<Feedback> {
        self.queue.dispatch_pending(&mut self.state).unwrap();
        std::mem::take(&mut self.state.feedback)
    }
}

pub fn thread_cpu_ns() -> u64 {
    clock_ns(libc::CLOCK_THREAD_CPUTIME_ID)
}

fn clock_ns(id: libc::clockid_t) -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Either the compositor-advertised clock or the native thread CPU clock.
    assert_eq!(unsafe { libc::clock_gettime(id, &mut time) }, 0);
    assert!(time.tv_sec >= 0 && (0..1_000_000_000).contains(&time.tv_nsec));
    (time.tv_sec as u64).checked_mul(1_000_000_000).unwrap() + time.tv_nsec as u64
}
impl Drop for Presentation {
    fn drop(&mut self) {
        self.state.presentation.take().unwrap().destroy();
        let _ = self.connection.flush();
    }
}
impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
            && interface == "wp_presentation"
        {
            state.presentation = Some(registry.bind(name, 1, qh, ()));
        }
    }
}
impl Dispatch<WpPresentation, ()> for State {
    fn event(
        state: &mut Self,
        _: &WpPresentation,
        event: wp_presentation::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_presentation::Event::ClockId { clk_id } = event {
            state.clock = Some(clk_id);
        }
    }
}
impl Dispatch<WpPresentationFeedback, u64> for State {
    fn event(
        state: &mut Self,
        _: &WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        frame: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wp_presentation_feedback::Event::Presented {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
                refresh,
                flags,
                ..
            } => {
                let seconds = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
                state.feedback.push(Feedback {
                    frame: *frame,
                    presented_ns: Some(
                        seconds.checked_mul(1_000_000_000).unwrap() + u64::from(tv_nsec),
                    ),
                    refresh_ns: refresh,
                    flags: u32::from(flags),
                });
            }
            wp_presentation_feedback::Event::Discarded => state.feedback.push(Feedback {
                frame: *frame,
                presented_ns: None,
                refresh_ns: 0,
                flags: 0,
            }),
            _ => {}
        }
    }
}
