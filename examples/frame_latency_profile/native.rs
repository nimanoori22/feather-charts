#[path = "presentation.rs"]
mod presentation;
use feather_charts::{
    api::{Chart, ChartState},
    model::{
        data_consumer::{BuiltInSeriesDataItem, HistogramData, HistogramDataItem},
        data_layer::SeriesId,
        horz_scale_behavior_time::types::UtcTimestamp,
        pane::PriceScalePosition,
        series_options::SeriesOptionsMap,
        time_data::LogicalRange,
    },
};
use iced::{
    Color, Point, Rectangle, Size, Theme,
    advanced::{
        graphics::{
            self,
            compositor::{self, Compositor as _},
            geometry::Renderer as _,
        },
        renderer::Renderer as _,
    },
    mouse,
    widget::canvas::{Event, Program},
};
use presentation::Presentation;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

type Compositor = <iced::Renderer as compositor::Default>::Compositor;
type Surface = <Compositor as compositor::Compositor>::Surface;
const PHASES: [&str; 4] = ["append", "zoom", "pan", "price"];
const PER_PHASE: usize = 120;
const TOTAL: usize = PER_PHASE * PHASES.len();
const PERIOD: Duration = Duration::from_nanos(16_666_667);

#[derive(Debug)]
struct EmptyPrimitive;
impl feather_charts::api::DrawingPrimitive for EmptyPrimitive {
    fn prepare(
        &self,
        _: &feather_charts::api::PrimitiveContext<'_>,
    ) -> feather_charts::api::PreparedPrimitive {
        Default::default()
    }
}

fn row(
    index: usize,
    fixed: bool,
    revision: usize,
) -> BuiltInSeriesDataItem<feather_charts::model::horz_scale_behavior_time::types::Time> {
    BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
        time: UtcTimestamp::new(1_700_000_000. + index as f64 * 60.).into(),
        value: if fixed {
            100. + revision as f64 * 0.0001
        } else {
            100. + (index as f64 * 0.03).sin() * 10.
        },
        color: None,
        custom_values: None,
    }))
}
struct Sample {
    frame: usize,
    started_ns: u64,
    update_ns: u64,
    prepare_ns: u64,
    geometry_ns: u64,
    present_call_ns: u64,
    present_cpu_ns: u64,
    work_cpu_ns: u64,
    work_ns: u64,
    presented_ns: Option<u64>,
    resolved: bool,
    discarded: bool,
    refresh_ns: u32,
    flags: u32,
    skipped: usize,
}
struct Running {
    probe: Presentation,
    compositor: Compositor,
    renderer: iced::Renderer,
    surface: Surface,
    window: Arc<Window>,
    chart: Chart,
    state: ChartState,
    owner: SeriesId,
    samples: Vec<Sample>,
    next: Instant,
    began: Instant,
    rows: usize,
    viewport: graphics::Viewport,
    size_changed: bool,
    feedback_deadline: Option<Instant>,
    diagnostic_draw: String,
    capture_dir: Option<PathBuf>,
}
impl Running {
    fn new(
        event_loop: &ActiveEventLoop,
        backend: &str,
        rows: usize,
        narrow: bool,
        diagnostic_draw: &str,
    ) -> Self {
        let dimensions = if narrow { (420., 320.) } else { (917., 870.8) };
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Feather Charts — frame acceptance")
                        .with_inner_size(LogicalSize::new(dimensions.0, dimensions.1))
                        .with_resizable(false),
                )
                .unwrap(),
        );
        let mut compositor = iced::futures::executor::block_on(Compositor::with_backend(
            graphics::Settings::default(),
            window.clone(),
            window.clone(),
            graphics::Shell::headless(),
            Some(backend),
        ))
        .unwrap();
        let renderer = compositor.create_renderer();
        assert!(
            matches!(
                (&renderer, backend),
                (iced::Renderer::Primary(_), "wgpu") | (iced::Renderer::Secondary(_), "tiny-skia")
            ),
            "renderer fallback invalidates the measurement"
        );
        let physical = window.inner_size();
        let surface = compositor.create_surface(window.clone(), physical.width, physical.height);
        let viewport = graphics::Viewport::with_physical_size(
            Size::new(physical.width, physical.height),
            window.scale_factor() as f32,
        );
        let probe = Presentation::new(window.clone());
        let mut chart = Chart::default();
        let mut owner = None;
        for source in 0_usize..16 {
            let mut options =
                feather_charts::model::series_options::HistogramSeriesOptions::default();
            if diagnostic_draw.starts_with("capture") {
                options.style.color = format!(
                    "rgba({},{},{},{})",
                    30 + source * 11,
                    200 - source * 7,
                    60 + source * 9,
                    if source.is_multiple_of(3) { 0.4 } else { 1.0 }
                );
            }
            let id = chart
                .add_series(
                    SeriesOptionsMap::Histogram(options),
                    0,
                    PriceScalePosition::Right,
                )
                .unwrap();
            chart
                .set_data(
                    id,
                    (0..rows)
                        .map(|index| {
                            let mut data = row(index, index + 1 == rows, 0);
                            if diagnostic_draw.starts_with("capture")
                                && let BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(
                                    ref mut data,
                                )) = data
                            {
                                data.value += (source % 5) as f64 * 1.5;
                            }
                            data
                        })
                        .collect(),
                )
                .unwrap();
            owner.get_or_insert(id);
        }
        chart.set_logical_range(LogicalRange {
            from: 20.0.into(),
            to: 100.0.into(),
        });
        chart
            .prepare(
                viewport.logical_size(),
                viewport.scale_factor(),
                Instant::now(),
            )
            .unwrap();
        Self {
            probe,
            compositor,
            renderer,
            surface,
            window,
            chart,
            state: ChartState::default(),
            owner: owner.unwrap(),
            samples: Vec::with_capacity(TOTAL),
            next: Instant::now(),
            began: Instant::now(),
            rows,
            viewport,
            size_changed: false,
            feedback_deadline: None,
            diagnostic_draw: "full".into(),
            capture_dir: None,
        }
    }
    fn input(&mut self, event: mouse::Event, position: Point) {
        if let Some(action) = self.chart.program(std::convert::identity).update(
            &mut self.state,
            &Event::Mouse(event),
            Rectangle::with_size(self.viewport.logical_size()),
            mouse::Cursor::Available(position),
        ) && let Some(message) = action.into_inner().0
        {
            self.chart.update(message).unwrap();
        }
    }
    fn scripted_input(&mut self, frame: usize) {
        let phase = frame / PER_PHASE;
        let step = frame % PER_PHASE;
        let plot = self.chart.frame().layout.plot;
        let center = Point::new(
            (plot.origin.x + plot.size.width * 0.5) as f32,
            (plot.origin.y + plot.size.height * 0.5) as f32,
        );
        match phase {
            1 | 3 => {
                let position = if phase == 3 {
                    let axis = self.chart.frame().layout.right_axis;
                    Point::new((axis.origin.x + axis.size.width * 0.5) as f32, center.y)
                } else {
                    center
                };
                self.input(
                    mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Lines {
                            x: 0.,
                            y: if step.is_multiple_of(2) { 0.05 } else { -0.05 },
                        },
                    },
                    position,
                );
            }
            2 => {
                let position = Point::new(
                    center.x + (step as f32 * 0.1).sin() * 20.,
                    center.y + (step as f32 * 0.1).sin() * 10.,
                );
                self.input(mouse::Event::CursorMoved { position }, position);
                if step == 0 {
                    self.input(mouse::Event::ButtonPressed(mouse::Button::Left), position);
                }
                if step + 1 == PER_PHASE {
                    self.input(mouse::Event::ButtonReleased(mouse::Button::Left), position);
                }
            }
            _ => {}
        }
    }
    fn redraw(&mut self) {
        if self.samples.len() == TOTAL || Instant::now() < self.next {
            return;
        }
        let frame = self.samples.len();
        let started_ns = self.probe.now_ns();
        let started_cpu = presentation::thread_cpu_ns();
        let started = Instant::now();
        self.scripted_input(frame);
        self.chart
            .update_data(self.owner, row(self.rows + frame, true, frame), false)
            .unwrap();
        let updated = Instant::now();
        self.chart
            .prepare(
                self.viewport.logical_size(),
                self.viewport.scale_factor(),
                updated,
            )
            .unwrap();
        let prepared = Instant::now();
        let bounds = Rectangle::with_size(self.viewport.logical_size());
        self.renderer.reset(bounds);
        let geometry =
            if ["full", "capture", "capture-reference"].contains(&self.diagnostic_draw.as_str()) {
                self.chart.program(std::convert::identity).draw(
                    &self.state,
                    &self.renderer,
                    &Theme::Dark,
                    bounds,
                    mouse::Cursor::Unavailable,
                )
            } else {
                let mut pane = self.chart.frame().clone();
                match self.diagnostic_draw.as_str() {
                    "no-rectangles" => {
                        pane.plot.candles.clear();
                        for layer in &mut pane.plot.series_layers {
                            layer.candles.clear();
                        }
                    }
                    "no-axes" => {
                        pane.prices = Default::default();
                        pane.pane_labels = Default::default();
                        pane.time = Default::default();
                        pane.left_corner = Default::default();
                        pane.right_corner = Default::default();
                    }
                    "no-strokes" => {
                        pane.plot.grid.clear();
                        pane.plot.lines.clear();
                        pane.plot.gradient_lines.clear();
                        pane.plot.price_lines.clear();
                        pane.plot.crosshair.clear();
                        for layer in &mut pane.plot.series_layers {
                            layer.lines.clear();
                            layer.gradient_lines.clear();
                        }
                    }
                    _ => unreachable!(),
                }
                let mut canvas = iced::widget::canvas::Frame::new(&self.renderer, bounds.size());
                pane.draw(&mut canvas);
                vec![canvas.into_geometry()]
            };
        for geometry in geometry {
            self.renderer.draw_geometry(geometry);
        }
        // Force visible damage even when a historical viewport's formatted labels stay unchanged.
        self.renderer.fill_quad(
            iced::advanced::renderer::Quad {
                bounds: Rectangle::new(Point::ORIGIN, Size::new(4., 4.)),
                ..Default::default()
            },
            Color::from_rgb8(frame as u8, (frame >> 8) as u8, 127),
        );
        let drawn = Instant::now();
        if frame % PER_PHASE == 20
            && let Some(directory) = &self.capture_dir
        {
            let rgba = self
                .compositor
                .screenshot(&mut self.renderer, &self.viewport, Color::WHITE);
            fs::write(directory.join(format!("frame-{frame}.rgba")), rgba).unwrap();
        }
        let present_cpu = presentation::thread_cpu_ns();
        let probe = &self.probe;
        let window = &self.window;
        self.compositor
            .present(
                &mut self.renderer,
                &mut self.surface,
                &self.viewport,
                Color::WHITE,
                || {
                    probe.request(frame as u64);
                    window.pre_present_notify();
                },
            )
            .unwrap();
        let submitted = Instant::now();
        let finished_cpu = presentation::thread_cpu_ns();
        let work_ns = self.probe.now_ns().checked_sub(started_ns).unwrap();
        self.next += PERIOD;
        let mut skipped = 0;
        while self.next <= submitted {
            self.next += PERIOD;
            skipped += 1;
        }
        self.samples.push(Sample {
            frame,
            started_ns,
            update_ns: (updated - started).as_nanos() as u64,
            prepare_ns: (prepared - updated).as_nanos() as u64,
            geometry_ns: (drawn - prepared).as_nanos() as u64,
            present_call_ns: (submitted - drawn).as_nanos() as u64,
            present_cpu_ns: finished_cpu - present_cpu,
            work_cpu_ns: finished_cpu - started_cpu,
            work_ns,
            presented_ns: None,
            resolved: false,
            discarded: false,
            refresh_ns: 0,
            flags: 0,
            skipped,
        });
        if self.samples.len() == TOTAL {
            self.feedback_deadline = Some(Instant::now() + Duration::from_secs(3));
        }
    }
    fn drain(&mut self) {
        for feedback in self.probe.drain() {
            let sample = self
                .samples
                .get_mut(feedback.frame as usize)
                .expect("unknown frame feedback");
            assert!(!sample.resolved, "duplicate feedback");
            sample.resolved = true;
            sample.discarded = feedback.presented_ns.is_none();
            sample.presented_ns = feedback.presented_ns;
            sample.refresh_ns = feedback.refresh_ns;
            sample.flags = feedback.flags;
        }
    }
    fn save(&self, output: &PathBuf, backend: &str) {
        fs::create_dir_all(output).unwrap();
        let mut metadata = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("metadata.txt"))
            .unwrap();
        writeln!(metadata, "requested_backend={backend}\nbackend={:?}\nrows={}\nsources=16\nframes_per_phase=120\nwarmups_per_phase=20\nclock_id={}\nphysical_width={}\nphysical_height={}\nscale={}\nsize_changed={}\nmonitor_refresh_millihertz={:?}",
            self.compositor.information(), self.rows, self.probe.clock_id(), self.viewport.physical_width(), self.viewport.physical_height(), self.viewport.scale_factor(), self.size_changed,
            self.window.current_monitor().and_then(|monitor| monitor.refresh_rate_millihertz())).unwrap();
        writeln!(metadata, "diagnostic_draw={}", self.diagnostic_draw).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("frames.csv"))
            .unwrap();
        writeln!(file,"frame,phase,warmup,started_ns,update_ns,prepare_ns,geometry_ns,present_call_ns,present_cpu_ns,work_cpu_ns,work_ns,presented_ns,discarded,resolved,refresh_ns,flags,skipped_deadlines").unwrap();
        for s in &self.samples {
            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                s.frame,
                PHASES[s.frame / PER_PHASE],
                s.frame % PER_PHASE < 20,
                s.started_ns,
                s.update_ns,
                s.prepare_ns,
                s.geometry_ns,
                s.present_call_ns,
                s.present_cpu_ns,
                s.work_cpu_ns,
                s.work_ns,
                s.presented_ns.map_or_else(String::new, |n| n.to_string()),
                s.discarded,
                s.resolved,
                s.refresh_ns,
                s.flags,
                s.skipped
            )
            .unwrap();
        }
    }
}
struct App {
    running: Option<Running>,
    backend: String,
    rows: usize,
    narrow: bool,
    output: PathBuf,
    complete: bool,
    diagnostic_draw: String,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_none() {
            self.running = Some(Running::new(
                event_loop,
                &self.backend,
                self.rows,
                self.narrow,
                &self.diagnostic_draw,
            ));
            self.running.as_mut().unwrap().diagnostic_draw = self.diagnostic_draw.clone();
            if self.diagnostic_draw.starts_with("capture") {
                fs::create_dir(&self.output).unwrap();
                let running = self.running.as_mut().unwrap();
                running.capture_dir = Some(self.output.clone());
                if self.diagnostic_draw == "capture-reference" {
                    running.chart.add_pane_primitive(0, EmptyPrimitive).unwrap();
                }
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        match event {
            WindowEvent::RedrawRequested => running.redraw(),
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                let next = graphics::Viewport::with_physical_size(
                    Size::new(size.width, size.height),
                    running.window.scale_factor() as f32,
                );
                if next.physical_size() != running.viewport.physical_size()
                    || next.scale_factor() != running.viewport.scale_factor()
                {
                    running.size_changed |= running.samples.len() >= 20;
                    running.compositor.configure_surface(
                        &mut running.surface,
                        size.width,
                        size.height,
                    );
                    running.viewport = next;
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                running.size_changed |= running.samples.len() >= 20;
            }
            WindowEvent::CloseRequested => {
                running.drain();
                running.save(&self.output, &self.backend);
                event_loop.exit();
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(running) = &mut self.running else {
            return;
        };
        running.drain();
        let now = Instant::now();
        if running.samples.len() == TOTAL && running.samples.iter().all(|sample| sample.resolved) {
            running.save(&self.output, &self.backend);
            self.complete = true;
            event_loop.exit();
            return;
        }
        if running
            .feedback_deadline
            .is_some_and(|deadline| now >= deadline)
            || now - running.began > Duration::from_secs(60)
        {
            running.save(&self.output, &self.backend);
            event_loop.exit();
            return;
        }
        if now >= running.next && running.samples.len() < TOTAL {
            running.window.request_redraw();
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let deadline = if running.samples.len() == TOTAL {
            now + PERIOD
        } else {
            running.next
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
    }
}
pub fn run() {
    let args: Vec<_> = std::env::args().collect();
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .map(|at| args.get(at + 1).expect("missing argument"))
    };
    let backend = value("--backend")
        .map_or("tiny-skia", String::as_str)
        .to_owned();
    assert!(["tiny-skia", "wgpu"].contains(&backend.as_str()));
    let rows = value("--rows").map_or(100000, |v| v.parse().unwrap());
    assert!([1200, 100000].contains(&rows));
    let output = PathBuf::from(value("--output").expect("--output directory is required"));
    let diagnostic_draw = value("--diagnostic-draw")
        .map_or("full", String::as_str)
        .to_owned();
    assert!(
        [
            "full",
            "no-rectangles",
            "no-axes",
            "no-strokes",
            "capture",
            "capture-reference"
        ]
        .contains(&diagnostic_draw.as_str())
    );
    assert!(!output.exists(), "use a fresh output directory");
    let mut app = App {
        running: None,
        backend,
        rows,
        narrow: args.iter().any(|a| a == "--narrow"),
        output,
        complete: false,
        diagnostic_draw,
    };
    EventLoop::new().unwrap().run_app(&mut app).unwrap();
    assert!(
        app.complete,
        "native profile incomplete; retained available frame feedback"
    );
}
