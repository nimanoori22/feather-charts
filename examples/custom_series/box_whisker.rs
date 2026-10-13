//! Native implementation of the upstream box-whisker example, not engine code.
use feather_charts::model::{
    data_consumer::TimedData,
    horz_scale_behavior_time::types::{Time, TimePoint, UtcTimestamp},
    icustom_series::*,
};
use iced::{
    Color, Point, Size,
    widget::canvas::{Frame, Path},
};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Datum {
    pub time: Time,
    pub quartiles: [f64; 5],
    pub outliers: Vec<f64>,
}
impl TimedData for Datum {
    type Item = Time;
    fn time(&self) -> &Time {
        &self.time
    }
    fn time_mut(&mut self) -> &mut Time {
        &mut self.time
    }
}
impl CustomData for Datum {}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub colors: [Color; 4],
}
impl Default for Options {
    fn default() -> Self {
        Self {
            colors: [
                Color::from_rgb8(106, 27, 154),
                Color::from_rgb8(103, 58, 183),
                Color::from_rgb8(233, 30, 99),
                Color::from_rgb8(149, 152, 161),
            ],
        }
    }
}

/// Bitmap geometry, also used by the source-reference fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub color: usize,
    pub circle: bool,
    pub coordinates: [f64; 4],
}
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}
fn line(position: f64, ratio: f64, width: f64) -> (f64, f64) {
    let width = js_round(width * ratio);
    (js_round(position * ratio) - (width * 0.5).floor(), width)
}
fn between(first: f64, second: f64, ratio: f64) -> (f64, f64) {
    let first = js_round(first * ratio);
    let second = js_round(second * ratio);
    (first.min(second), (first - second).abs() + 1.)
}
pub fn candle_width(spacing: f64, ratio: f64) -> f64 {
    let coefficient = 1. - 0.2 * (spacing.max(4.) - 4.).atan() / (std::f64::consts::PI * 0.5);
    let mut width = if (2.5..=4.).contains(&spacing) {
        (3. * ratio).floor()
    } else {
        (spacing * coefficient * ratio)
            .floor()
            .min((spacing * ratio).floor())
            .max(ratio.floor())
    };
    if width >= 2. && (ratio.floor() as i64 % 2 != width as i64 % 2) {
        width -= 1.;
    }
    width
}
pub fn geometry(
    x: f64,
    quartiles: [f64; 5],
    outliers: &[f64],
    spacing: f64,
    ratio: f64,
) -> Vec<Shape> {
    if !ratio.is_finite()
        || ratio <= 0.
        || !x.is_finite()
        || !spacing.is_finite()
        || quartiles.iter().any(|value| !value.is_finite())
    {
        return vec![];
    }
    let body = candle_width(spacing, 1.);
    let median = spacing.floor().max(body);
    let extreme = candle_width(spacing * 0.5, 1.);
    let radius = body.min(4.);
    let grid = ratio.floor().max(1.) / ratio;
    let mut result = vec![];
    if radius > 2. {
        for &y in outliers.iter().filter(|y| y.is_finite()) {
            // The plugin uses a bitmap radius, even though candle widths are media widths.
            result.push(Shape {
                color: 3,
                circle: true,
                coordinates: [js_round(x * ratio), js_round(y * ratio), radius, 0.],
            });
        }
    }
    let mut rect = |color, horizontal: (f64, f64), vertical: (f64, f64)| {
        result.push(Shape {
            color,
            circle: false,
            coordinates: [horizontal.0, vertical.0, horizontal.1, vertical.1],
        });
    };
    let whisker = line(x, ratio, grid);
    rect(0, whisker, between(quartiles[0], quartiles[1], ratio));
    rect(0, whisker, between(quartiles[3], quartiles[4], ratio));
    rect(0, line(x, ratio, extreme), line(quartiles[4], ratio, grid));
    rect(0, line(x, ratio, extreme), line(quartiles[0], ratio, grid));
    rect(
        1,
        line(x, ratio, body),
        between(quartiles[1], quartiles[2], ratio),
    );
    rect(
        2,
        line(x, ratio, body),
        between(quartiles[2], quartiles[3], ratio),
    );
    rect(0, line(x, ratio, median), line(quartiles[2], ratio, grid));
    result
}

#[derive(Clone, Default)]
pub struct Renderer {
    pub data: Option<Rc<PaneRendererCustomData<Datum>>>,
    options: Options,
}
impl CustomSeriesPaneRenderer<Frame> for Renderer {
    type HitTestData = ();
    fn draw(
        &self,
        frame: &mut Frame,
        convert: &PriceToCoordinateConverter<'_>,
        _: bool,
        _: Option<&()>,
    ) {
        let Some(data) = &self.data else {
            return;
        };
        let Some(range) = &data.visible_range else {
            return;
        };
        let ratio = data.pixel_ratio;
        for bar in &data.bars[range.from..range.to] {
            let mut ys = [0.; 5];
            let mut valid = true;
            for (y, price) in ys.iter_mut().zip(bar.original_data.quartiles) {
                if let Some(coordinate) = convert(price.into()) {
                    *y = coordinate.value();
                } else {
                    valid = false;
                }
            }
            if !valid {
                continue;
            }
            let outliers: Vec<_> = bar
                .original_data
                .outliers
                .iter()
                .filter_map(|price| convert((*price).into()).map(|y| y.value()))
                .collect();
            for shape in geometry(bar.x.value(), ys, &outliers, data.bar_spacing, ratio) {
                let [x, y, width, height] = shape.coordinates.map(|value| (value / ratio) as f32);
                let color = self.options.colors[shape.color];
                if shape.circle {
                    frame.fill(&Path::circle(Point::new(x, y), width), color);
                } else {
                    frame.fill_rectangle(Point::new(x, y), Size::new(width, height), color);
                }
            }
        }
    }
}
#[derive(Default)]
pub struct View {
    renderer: Renderer,
}
impl CustomSeriesPaneView<Frame> for View {
    type Item = Time;
    type InternalTime = TimePoint;
    type Data = Datum;
    type Metadata = ();
    type Options = Options;
    type Renderer = Renderer;
    fn renderer(&self) -> &Renderer {
        &self.renderer
    }
    fn update(&mut self, data: PaneRendererCustomData<Datum>, options: &Options) {
        self.renderer.data = Some(Rc::new(data));
        self.renderer.options = *options;
    }
    fn price_value_builder(&self, data: &Datum) -> CustomSeriesPricePlotValues {
        [data.quartiles[4], data.quartiles[0], data.quartiles[2]]
            .map(Into::into)
            .to_vec()
    }
    fn is_whitespace(&self, data: &CustomSeriesDataItem<Datum, Time>) -> bool {
        matches!(data, CustomSeriesDataItem::Whitespace(_))
    }
    fn default_options(&self) -> Options {
        Options::default()
    }
}
pub fn sample(index: usize) -> CustomSeriesDataItem<Datum, Time> {
    let center = 100. + (index as f64 * 0.13).sin() * 8.;
    CustomSeriesDataItem::Data(Datum {
        time: UtcTimestamp::new(1_700_000_000. + index as f64 * 60.).into(),
        quartiles: [center - 5., center - 2., center, center + 3., center + 6.],
        outliers: if index.is_multiple_of(7) {
            vec![center - 9., center + 10.]
        } else {
            vec![]
        },
    })
}
