//! Fixed-batch diagnostic: kind rows width height backend mode output [DPR].
use feather_charts::{
    api::{Chart, ChartMessage, ChartState},
    model::{
        data_consumer::*, horz_scale_behavior_time::types::UtcTimestamp, pane::PriceScalePosition,
        series_options::*, time_scale_options::HorzScaleOptionsPatch,
    },
};
use iced::{
    advanced::{
        graphics::geometry::Renderer as _,
        renderer::{Headless, Renderer as _},
    },
    widget::canvas::Program,
};
use std::{
    cell::Cell,
    io::Write,
    rc::Rc,
    time::{Duration, Instant},
};
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert!(
        (7..=8).contains(&args.len()),
        "kind rows width height backend mode output [DPR]"
    );
    let rows = args[1].parse::<usize>().unwrap();
    let size = iced::Size::new(args[2].parse().unwrap(), args[3].parse().unwrap());
    let ratio = args.get(7).map_or(1., |v| v.parse::<f32>().unwrap());
    let mode = match args[5].as_str() {
        "disabled" => LastPriceAnimationMode::Disabled,
        "continuous" => LastPriceAnimationMode::Continuous,
        _ => panic!("mode"),
    };
    let mut options = match args[0].as_str() {
        "line" => {
            let mut o = LineSeriesOptions::default();
            o.style.last_price_animation = mode;
            SeriesOptionsMap::Line(o)
        }
        "area" => {
            let mut o = AreaSeriesOptions::default();
            o.style.last_price_animation = mode;
            SeriesOptionsMap::Area(o)
        }
        "baseline" => {
            let mut o = BaselineSeriesOptions::default();
            o.style.last_price_animation = mode;
            SeriesOptionsMap::Baseline(o)
        }
        _ => panic!("kind"),
    };
    let calls = Rc::new(Cell::new(0));
    let captured = calls.clone();
    let common = match &mut options {
        SeriesOptionsMap::Line(o) => &mut o.common,
        SeriesOptionsMap::Area(o) => &mut o.common,
        SeriesOptionsMap::Baseline(o) => &mut o.common,
        _ => unreachable!(),
    };
    common.price_format = PriceFormat::Custom(PriceFormatCustom {
        formatter: Rc::new(move |p| {
            captured.set(captured.get() + 1);
            format!("{:.2}", p.value())
        }),
        tickmarks_formatter: None,
        min_move: 0.01,
        base: Some(100.),
    });
    let mut chart = Chart::default();
    let id = chart
        .add_series(options, 0, PriceScalePosition::Right)
        .unwrap();
    chart
        .set_data(
            id,
            (0..rows)
                .map(|i| {
                    let time = UtcTimestamp::new(1_700_000_000. + i as f64 * 60.).into();
                    let value = 100. + (i as f64 * 0.1).sin() * 8.;
                    match args[0].as_str() {
                        "line" => BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                            time,
                            value,
                            color: None,
                            custom_values: None,
                        })),
                        "area" => BuiltInSeriesDataItem::Area(AreaDataItem::Data(AreaData {
                            time,
                            value,
                            line_color: None,
                            top_color: None,
                            bottom_color: None,
                            custom_values: None,
                        })),
                        _ => {
                            BuiltInSeriesDataItem::Baseline(BaselineDataItem::Data(BaselineData {
                                time,
                                value,
                                top_line_color: None,
                                bottom_line_color: None,
                                top_fill_color1: None,
                                top_fill_color2: None,
                                bottom_fill_color1: None,
                                bottom_fill_color2: None,
                                custom_values: None,
                            }))
                        }
                    }
                })
                .collect(),
        )
        .unwrap();
    chart.apply_time_scale_options(HorzScaleOptionsPatch {
        enable_conflation: Some(true),
        min_bar_spacing: Some(0.001),
        ..Default::default()
    });
    chart.fit_content();
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(12.),
        Some(&args[4]),
    ))
    .expect("requested backend available");
    assert!(
        matches!(
            (&renderer, args[4].as_str()),
            (iced::Renderer::Primary(_), "wgpu") | (iced::Renderer::Secondary(_), "tiny-skia")
        ),
        "unexpected backend fallback"
    );
    let origin = Instant::now();
    chart.prepare(size, ratio, origin).unwrap();
    let state = ChartState::default();
    let bounds = iced::Rectangle::with_size(size);
    let mut csv = std::fs::File::create(&args[6]).unwrap();
    writeln!(
        csv,
        "sample,prepare_ns,geometry_ns,raster_ns,formatter_calls,base_retained"
    )
    .unwrap();
    for sample in 0..100 {
        let now = origin + Duration::from_micros(sample * 32500);
        calls.set(0);
        let base = chart.frame().plot.lines.as_ptr();
        let fills = chart.frame().plot.fills.as_ptr();
        let start = Instant::now();
        chart.update(ChartMessage::Tick(now)).unwrap();
        let prepare = start.elapsed().as_nanos();
        let retained =
            base == chart.frame().plot.lines.as_ptr() && fills == chart.frame().plot.fills.as_ptr();
        renderer.reset(bounds);
        let start = Instant::now();
        for g in chart.program(std::convert::identity).draw(
            &state,
            &renderer,
            &iced::Theme::Dark,
            bounds,
            iced::mouse::Cursor::Unavailable,
        ) {
            renderer.draw_geometry(g);
        }
        let geometry = start.elapsed().as_nanos();
        let start = Instant::now();
        let rgba = renderer.screenshot(
            iced::Size::new((size.width * ratio) as u32, (size.height * ratio) as u32),
            ratio,
            iced::Color::BLACK,
        );
        let raster = start.elapsed().as_nanos();
        if sample >= 20 {
            assert_eq!(calls.get(), 0, "animation tick prepared axes");
            assert!(retained, "animation tick rebuilt base geometry");
            writeln!(
                csv,
                "{sample},{prepare},{geometry},{raster},{},{retained}",
                calls.get()
            )
            .unwrap();
        }
        if sample == 30 {
            std::fs::write(format!("{}.rgba", args[6]), rgba).unwrap();
        }
    }
    // Source stage at 325ms: deterministic frozen-frame captures, independent of setup duration.
    let mut frame = chart.frame().clone();
    for a in &mut frame.plot.last_price_animations {
        a.radius = 7.;
        a.fill_color.a = a.color.a * 0.125;
        a.stroke_color.a = a.color.a * 0.6;
    }
    renderer.reset(bounds);
    let mut canvas = iced::widget::canvas::Frame::new(&renderer, size);
    frame.draw(&mut canvas);
    renderer.draw_geometry(canvas.into_geometry());
    let physical = iced::Size::new((size.width * ratio) as u32, (size.height * ratio) as u32);
    let a = renderer.screenshot(physical, ratio, iced::Color::BLACK);
    renderer.reset(bounds);
    let mut canvas = iced::widget::canvas::Frame::new(&renderer, size);
    frame.draw(&mut canvas);
    renderer.draw_geometry(canvas.into_geometry());
    let b = renderer.screenshot(physical, ratio, iced::Color::BLACK);
    assert!(a == b, "retained frozen frame pixels changed");
    std::fs::write(format!("{}.fixed.rgba", args[6]), a).unwrap();
}
