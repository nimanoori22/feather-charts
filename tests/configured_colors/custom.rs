use super::*;
use feather_charts::{
    api::*,
    model::{
        bar::BarPrice,
        data_consumer::TimedData,
        horz_scale_behavior_time::types::{Time, TimePoint, UtcTimestamp},
        icustom_series::*,
        pane::PriceScalePosition,
        series_options::CustomSeriesOptions,
    },
};
use iced::widget::canvas::Frame;
#[derive(Clone, Debug, PartialEq)]
struct Datum {
    time: Time,
    color: String,
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
impl CustomData for Datum {
    fn color(&self) -> Option<&str> {
        Some(&self.color)
    }
}
#[derive(Clone)]
struct Renderer {
    color: iced::Color,
}
impl CustomSeriesPaneRenderer<Frame> for Renderer {
    type HitTestData = ();
    fn draw(&self, f: &mut Frame, _: &PriceToCoordinateConverter<'_>, _: bool, _: Option<&()>) {
        f.fill_rectangle(
            iced::Point::new(10., 10.),
            iced::Size::new(20., 20.),
            self.color,
        );
    }
}
struct View {
    renderer: Renderer,
    updates: Rc<Cell<usize>>,
}
impl CustomSeriesPaneView<Frame> for View {
    type Item = Time;
    type InternalTime = TimePoint;
    type Data = Datum;
    type Metadata = ();
    type Options = String;
    type Renderer = Renderer;
    fn renderer(&self) -> &Renderer {
        &self.renderer
    }
    fn default_options(&self) -> String {
        DARK.into()
    }
    fn update(&mut self, _: PaneRendererCustomData<Datum>, _: &String) {
        panic!("configured hook must be used")
    }
    fn validate_options(
        &self,
        options: &String,
        mut colors: ColorParser,
    ) -> Result<(), feather_charts::model::colors::ColorParseError> {
        colors.parse_color(options).map(|_| ())
    }
    fn update_with_color_parser(
        &mut self,
        _: PaneRendererCustomData<Datum>,
        options: &String,
        mut colors: ColorParser,
    ) {
        let [r, g, b, a] = colors.rendering_rgba(options).unwrap();
        self.renderer.color = iced::Color::from_rgba(r, g, b, a);
        self.updates.set(self.updates.get() + 1);
    }
    fn price_value_builder(&self, _: &Datum) -> CustomSeriesPricePlotValues {
        vec![BarPrice::new(3.)]
    }
    fn is_whitespace(&self, d: &CustomSeriesDataItem<Datum, Time>) -> bool {
        matches!(d, CustomSeriesDataItem::Whitespace(_))
    }
}
#[test]
fn custom_rows_and_renderer_options_use_policy_with_atomic_validation() {
    let (mut c, calls) = configured_chart();
    let updates = Rc::new(Cell::new(0));
    let mut opts = CustomSeriesOptions::default();
    opts.style.color = DARK.into();
    let handle = c
        .add_custom_series(
            View {
                renderer: Renderer {
                    color: iced::Color::BLACK,
                },
                updates: updates.clone(),
            },
            opts,
            0,
            PriceScalePosition::Right,
        )
        .unwrap();
    let rows = vec![CustomSeriesDataItem::Data(Datum {
        time: UtcTimestamp::new(1700000000.).into(),
        color: LIGHT.into(),
    })];
    c.set_custom_data(&handle, rows).unwrap();
    c.fit_content();
    c.prepare(iced::Size::new(400., 300.), 1.25, std::time::Instant::now())
        .unwrap();
    assert!(updates.get() > 0);
    assert_eq!(calls.get(), 2);
    assert_eq!(handle.data()[0].color, LIGHT);
    let before = handle.data();
    assert!(
        c.set_custom_data(
            &handle,
            vec![CustomSeriesDataItem::Data(Datum {
                time: UtcTimestamp::new(1700000000.).into(),
                color: "invalid".into()
            })]
        )
        .is_err()
    );
    assert_eq!(handle.data(), before);
    assert!(
        c.replace_custom_view_options(&handle, "invalid".into())
            .is_err()
    );
    handle
        .with_view_options(&c, |o| assert_eq!(o, DARK))
        .unwrap();
    c.replace_custom_view_options(&handle, LIGHT.into())
        .unwrap();
    c.prepare(iced::Size::new(400., 300.), 1.25, std::time::Instant::now())
        .unwrap();
}
#[derive(Debug)]
struct Primitive {
    color: String,
    attached: Rc<Cell<usize>>,
}
impl DrawingPrimitive for Primitive {
    fn validate_colors(
        &self,
        mut p: ColorParser,
    ) -> Result<(), feather_charts::model::colors::ColorParseError> {
        p.parse_color(&self.color).map(|_| ())
    }
    fn attached(&self, _: PrimitiveOwner) {
        self.attached.set(self.attached.get() + 1)
    }
    fn prepare(&self, ctx: &PrimitiveContext<'_>) -> PreparedPrimitive {
        let mut colors = ctx.color_parser();
        assert!(colors.parse_color(&self.color).is_ok());
        PreparedPrimitive {
            price_labels: vec![PrimitiveAxisLabel::new(
                40.,
                "custom",
                self.color.clone(),
                LIGHT,
            )],
            time_labels: vec![PrimitiveAxisLabel::new(
                50.,
                "custom",
                self.color.clone(),
                LIGHT,
            )],
            ..Default::default()
        }
    }
}
#[test]
fn primitive_declared_colors_fail_before_attach_or_replace_and_labels_use_policy() {
    let (mut c, _) = configured_chart();
    let id = c
        .add_series(options(0, DARK), 0, PriceScalePosition::Right)
        .unwrap();
    c.set_data(id, (0..3).map(|i| row(0, i, DARK)).collect())
        .unwrap();
    c.fit_content();
    let attached = Rc::new(Cell::new(0));
    assert!(
        c.add_series_primitive(
            id,
            Primitive {
                color: "invalid".into(),
                attached: attached.clone()
            }
        )
        .is_err()
    );
    assert_eq!(attached.get(), 0);
    let primitive = c
        .add_series_primitive(
            id,
            Primitive {
                color: DARK.into(),
                attached: attached.clone(),
            },
        )
        .unwrap();
    assert_eq!(attached.get(), 1);
    assert!(
        c.replace_primitive(
            primitive,
            Primitive {
                color: "invalid".into(),
                attached: attached.clone()
            }
        )
        .is_err()
    );
    assert_eq!(attached.get(), 1);
    assert_eq!(
        c.primitive_owner(primitive).unwrap(),
        PrimitiveOwner::Series(id)
    );
    c.prepare(iced::Size::new(400., 300.), 1.25, std::time::Instant::now())
        .unwrap();
}
