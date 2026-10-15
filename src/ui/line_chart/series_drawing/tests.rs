use super::super::{CanvasRectangle, CanvasSeriesGeometry, FrameLayout};
use super::{PlotSnapshot, SeriesDrawPass, SeriesPassKind};
use crate::{
    model::data_layer::SeriesId,
    renderers::{grid_renderer::PixelRatio, line_renderer::LineDrawCommands},
    ui::hit_testing::{HitGeometry, PreparedSeriesHit},
};
use iced::{Color, Point, Rectangle, Size};

fn rect(color: Color, height: f32) -> CanvasRectangle {
    CanvasRectangle {
        origin: Point::new(10.25, 20.5),
        size: Size::new(5., height),
        color,
    }
}
fn plot() -> PlotSnapshot {
    let mut plot = PlotSnapshot::from_commands(
        &FrameLayout {
            pane: 0,
            size: Size::new(100., 100.),
            pixel_ratio: PixelRatio {
                horizontal: 1.,
                vertical: 1.,
            },
            background: "white".into(),
        },
        vec![],
        LineDrawCommands::default(),
    )
    .unwrap();
    plot.series_id = Some(SeriesId::new(1));
    plot.candles = vec![rect(Color::from_rgb(1., 0., 0.), 100_000.)];
    plot.series_layers = [
        (2, Color::from_rgb(0., 1., 0.)),
        (3, Color::from_rgb(0., 0., 1.)),
    ]
    .map(|(id, color)| CanvasSeriesGeometry {
        series_id: Some(SeriesId::new(id)),
        candles: vec![rect(color, 100_000.)],
        ..Default::default()
    })
    .to_vec();
    plot
}

#[test]
fn opaque_different_color_can_cover_only_the_visible_part_of_an_offscreen_bar() {
    let bounds = Rectangle::with_size(Size::new(100., 100.));
    let earlier = rect(Color::from_rgba(1., 0., 0., 0.4), 1_000_000.);
    let later = rect(Color::from_rgb(0., 0., 1.), 79.5);
    let original = (earlier.origin, earlier.size, earlier.color);
    assert!(earlier.covered_by(&later, bounds));
    assert_eq!((earlier.origin, earlier.size, earlier.color), original);
    assert_ne!(
        earlier.color, later.color,
        "occlusion is geometric, not color or source identity deduplication"
    );
}

#[test]
fn partial_coverage_and_translucent_later_rectangles_must_not_cull() {
    let bounds = Rectangle::with_size(Size::new(100., 100.));
    let earlier = rect(Color::from_rgb(1., 0., 0.), 1_000_000.);
    for alpha in [0., 0.5, 0.999] {
        assert!(!earlier.covered_by(
            &rect(Color::from_rgba(0., 0., 1., alpha), 1_000_000.),
            bounds
        ));
    }
    let mut narrower = rect(Color::BLACK, 100_000.);
    narrower.size.width = 4.75;
    assert!(!earlier.covered_by(&narrower, bounds));
    let mut lower = rect(Color::BLACK, 100_000.);
    lower.origin.y += 0.25;
    assert!(!earlier.covered_by(&lower, bounds));
    assert!(!earlier.covered_by(&rect(Color::BLACK, 79.25), bounds));
}

#[test]
fn occluder_follows_actual_hover_raised_draw_order_and_preserves_source_geometry() {
    for filled in [false, true] {
        let mut plot = plot();
        plot.hovered_series = Some(SeriesId::new(2));
        plot.hovered_series_on_top = true;
        plot.hit_series.push(PreparedSeriesHit {
            series: SeriesId::new(2),
            filled,
            tolerance: 0.,
            geometry: HitGeometry::Range {
                items: vec![],
                spacing: 5.,
            },
        });
        let passes: Vec<_> = plot.series_draw_passes().collect();
        let normal_or_foreground: Vec<_> = passes
            .iter()
            .copied()
            .filter(|pass| pass.kind != SeriesPassKind::Background)
            .collect();
        assert_eq!(
            normal_or_foreground
                .iter()
                .map(|pass| pass.layer)
                .collect::<Vec<_>>(),
            vec![0, 2, 1]
        );
        assert_eq!(
            plot.occluding_candles(normal_or_foreground[0]).unwrap()[0].color,
            plot.series_layers[1].candles[0].color
        );
        assert_eq!(
            plot.occluding_candles(normal_or_foreground[1]).unwrap()[0].color,
            plot.series_layers[0].candles[0].color
        );
        assert!(plot.occluding_candles(normal_or_foreground[2]).is_none());
        if filled {
            assert!(
                passes
                    .iter()
                    .any(|pass| pass.layer == 1 && pass.kind == SeriesPassKind::Background)
            );
            assert_eq!(normal_or_foreground[2].kind, SeriesPassKind::Foreground);
        }
        assert_eq!(plot.series_id, Some(SeriesId::new(1)));
        assert_eq!(
            plot.series_layers
                .iter()
                .map(|layer| layer.series_id)
                .collect::<Vec<_>>(),
            vec![Some(SeriesId::new(2)), Some(SeriesId::new(3))]
        );
        assert_eq!(plot.candles.len(), 1);
        assert!(
            plot.series_layers
                .iter()
                .all(|layer| layer.candles.len() == 1)
        );
    }
}

#[test]
fn empty_immediate_next_pass_does_not_search_past_an_intervening_source() {
    let mut plot = plot();
    plot.series_layers[0].candles.clear();
    let first = SeriesDrawPass {
        layer: 0,
        kind: SeriesPassKind::Normal,
    };
    let next = plot
        .occluding_candles(first)
        .expect("the next source remains an explicit candidate");
    assert!(next.is_empty(), "must not skip to the opaque third source");
    assert_eq!(plot.series_layers[1].candles.len(), 1);
    assert!(
        plot.occluding_candles(SeriesDrawPass {
            layer: 2,
            kind: SeriesPassKind::Normal
        })
        .is_none()
    );
}
