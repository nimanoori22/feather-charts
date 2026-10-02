//! Extension contracts for user-defined series and their pane renderers.

use crate::model::{
    bar::BarPrice,
    coordinate::Coordinate,
    data_consumer::TimedData,
    time_data::{TimePointIndex, ValueRange},
};

/// A whitespace item for a custom series.
///
/// `Metadata` is intentionally generic: the chart engine ignores it while a
/// custom-series author may retain arbitrary application-specific values.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomSeriesWhitespaceData<Item, Metadata = ()> {
    pub time: Item,
    pub custom_values: Option<Metadata>,
}

impl<Item, Metadata> TimedData for CustomSeriesWhitespaceData<Item, Metadata> {
    type Item = Item;

    fn time(&self) -> &Self::Item {
        &self.time
    }

    fn time_mut(&mut self) -> &mut Self::Item {
        &mut self.time
    }
}

/// Data supplied by a custom-series author.
///
/// Implement this on an application's own data type. The type can carry any
/// additional fields required by its pane renderer.
pub trait CustomData: TimedData + Clone {
    fn color(&self) -> Option<&str> {
        None
    }
}

/// A custom-series input item, either a value-bearing datum or whitespace.
#[derive(Clone, Debug, PartialEq)]
pub enum CustomSeriesDataItem<Data, Item, Metadata = ()> {
    Data(Data),
    Whitespace(CustomSeriesWhitespaceData<Item, Metadata>),
}

impl<Data, Item, Metadata> TimedData for CustomSeriesDataItem<Data, Item, Metadata>
where
    Data: TimedData<Item = Item>,
{
    type Item = Item;

    fn time(&self) -> &Self::Item {
        match self {
            Self::Data(data) => data.time(),
            Self::Whitespace(data) => data.time(),
        }
    }

    fn time_mut(&mut self) -> &mut Self::Item {
        match self {
            Self::Data(data) => data.time_mut(),
            Self::Whitespace(data) => data.time_mut(),
        }
    }
}

/// One custom datum prepared with horizontal position and style information.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomBarItemData<Data> {
    pub x: Coordinate,
    pub time: TimePointIndex,
    pub original_data: Data,
    pub bar_color: String,
}

/// Data supplied to a custom pane view before its renderer draws.
#[derive(Clone, Debug, PartialEq)]
pub struct PaneRendererCustomData<Data> {
    pub bars: Vec<CustomBarItemData<Data>>,
    pub bar_spacing: f64,
    pub visible_range: Option<ValueRange<TimePointIndex>>,
    pub conflation_factor: f64,
}

/// Converts a price into a vertical chart coordinate.
pub type PriceToCoordinateConverter<'a> = dyn Fn(BarPrice) -> Option<Coordinate> + 'a;

/// Classification used to prioritize overlapping custom-series hits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CustomSeriesHitTestType {
    Point,
    Line,
    Range,
    #[default]
    Custom,
}

/// Result returned by a custom renderer hit test.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomSeriesHitTestResult<HitTestData> {
    pub distance: f64,
    pub object_id: Option<String>,
    pub hit_type: CustomSeriesHitTestType,
    pub cursor_style: Option<String>,
    pub hit_test_data: Option<HitTestData>,
}

/// Backend-neutral renderer used by a custom series.
///
/// `Target` is supplied by the chart's rendering adapter; an Iced adapter can
/// choose the native drawing context it exposes without emulating FancyCanvas.
pub trait CustomSeriesPaneRenderer<Target> {
    type HitTestData;

    fn draw(
        &self,
        target: &mut Target,
        price_to_coordinate: &PriceToCoordinateConverter<'_>,
        is_hovered: bool,
        hit_test_data: Option<&Self::HitTestData>,
    );

    fn hit_test(
        &self,
        _x: Coordinate,
        _y: Coordinate,
        _price_to_coordinate: &PriceToCoordinateConverter<'_>,
    ) -> Option<CustomSeriesHitTestResult<Self::HitTestData>> {
        None
    }
}

/// Values used for autoscaling and the series' current-price display.
pub type CustomSeriesPricePlotValues = Vec<BarPrice>;

/// The user-facing context passed to a custom conflation operation.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomConflationContext<Data, Item, InternalTime> {
    pub data: Data,
    pub index: TimePointIndex,
    pub original_time: Item,
    pub time: InternalTime,
    pub price_values: CustomSeriesPricePlotValues,
}

/// A reducer that combines exactly two custom data contexts.
pub type CustomConflationReducer<Data, Item, InternalTime> = Box<
    dyn Fn(
        &CustomConflationContext<Data, Item, InternalTime>,
        &CustomConflationContext<Data, Item, InternalTime>,
    ) -> Data,
>;

/// Defines a custom series' rendering, autoscale values, whitespace behavior,
/// default options, and optional conflation.
pub trait CustomSeriesPaneView<Target> {
    type Item: Clone;
    type InternalTime: Clone;
    type Data: CustomData<Item = Self::Item>;
    type Metadata;
    type Options;
    type Renderer: CustomSeriesPaneRenderer<Target>;

    fn renderer(&self) -> &Self::Renderer;
    fn update(&mut self, data: PaneRendererCustomData<Self::Data>, series_options: &Self::Options);
    fn price_value_builder(&self, data: &Self::Data) -> CustomSeriesPricePlotValues;
    fn is_whitespace(
        &self,
        data: &CustomSeriesDataItem<Self::Data, Self::Item, Self::Metadata>,
    ) -> bool;
    fn default_options(&self) -> Self::Options;

    /// Explicit cleanup hook for resources outside normal Rust ownership.
    fn destroy(&mut self) {}

    /// Combines two adjacent data points when time-scale conflation is active.
    fn conflate(
        &self,
        _first: &CustomConflationContext<Self::Data, Self::Item, Self::InternalTime>,
        _second: &CustomConflationContext<Self::Data, Self::Item, Self::InternalTime>,
    ) -> Option<Self::Data> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{
        bar::BarPrice, coordinate::Coordinate, data_consumer::TimedData, time_data::TimePointIndex,
    };

    use super::{
        CustomConflationContext, CustomData, CustomSeriesDataItem, CustomSeriesPaneRenderer,
        CustomSeriesPaneView, CustomSeriesPricePlotValues, CustomSeriesWhitespaceData,
        PaneRendererCustomData, PriceToCoordinateConverter,
    };

    #[derive(Clone, Debug, PartialEq)]
    struct Datum {
        time: u32,
        value: f64,
        color: Option<String>,
    }

    impl TimedData for Datum {
        type Item = u32;

        fn time(&self) -> &Self::Item {
            &self.time
        }

        fn time_mut(&mut self) -> &mut Self::Item {
            &mut self.time
        }
    }

    impl CustomData for Datum {
        fn color(&self) -> Option<&str> {
            self.color.as_deref()
        }
    }

    #[derive(Default)]
    struct Renderer;

    impl CustomSeriesPaneRenderer<Vec<String>> for Renderer {
        type HitTestData = usize;

        fn draw(
            &self,
            target: &mut Vec<String>,
            _price_to_coordinate: &PriceToCoordinateConverter<'_>,
            is_hovered: bool,
            hit_test_data: Option<&Self::HitTestData>,
        ) {
            target.push(format!("{is_hovered}:{:?}", hit_test_data.copied()));
        }
    }

    struct View {
        renderer: Renderer,
        updates: usize,
    }

    impl CustomSeriesPaneView<Vec<String>> for View {
        type Item = u32;
        type InternalTime = u64;
        type Data = Datum;
        type Metadata = String;
        type Options = String;
        type Renderer = Renderer;

        fn renderer(&self) -> &Self::Renderer {
            &self.renderer
        }

        fn update(&mut self, _data: PaneRendererCustomData<Self::Data>, _options: &Self::Options) {
            self.updates += 1;
        }

        fn price_value_builder(&self, data: &Self::Data) -> CustomSeriesPricePlotValues {
            vec![data.value.into()]
        }

        fn is_whitespace(
            &self,
            data: &CustomSeriesDataItem<Self::Data, Self::Item, Self::Metadata>,
        ) -> bool {
            matches!(data, CustomSeriesDataItem::Whitespace(_))
        }

        fn default_options(&self) -> Self::Options {
            "default".to_owned()
        }

        fn conflate(
            &self,
            first: &CustomConflationContext<Self::Data, Self::Item, Self::InternalTime>,
            second: &CustomConflationContext<Self::Data, Self::Item, Self::InternalTime>,
        ) -> Option<Self::Data> {
            Some(Datum {
                time: second.data.time,
                value: first.data.value + second.data.value,
                color: None,
            })
        }
    }

    #[test]
    fn custom_view_contract_covers_rendering_whitespace_and_conflation() {
        let mut view = View {
            renderer: Renderer,
            updates: 0,
        };
        let datum = Datum {
            time: 1,
            value: 2.5,
            color: Some("blue".to_owned()),
        };
        let whitespace = CustomSeriesDataItem::Whitespace(CustomSeriesWhitespaceData {
            time: 2,
            custom_values: Some("note".to_owned()),
        });
        let context = CustomConflationContext {
            data: datum.clone(),
            index: TimePointIndex::new(1.0),
            original_time: 1,
            time: 1_u64,
            price_values: vec![datum.value.into()],
        };
        let next = CustomConflationContext {
            data: Datum {
                time: 2,
                value: 3.0,
                color: None,
            },
            index: TimePointIndex::new(2.0),
            original_time: 2,
            time: 2_u64,
            price_values: vec![3.0.into()],
        };

        assert_eq!(view.price_value_builder(&datum)[0].value(), 2.5);
        assert_eq!(datum.color(), Some("blue"));
        assert!(view.is_whitespace(&whitespace));
        assert_eq!(view.conflate(&context, &next).unwrap().value, 5.5);

        view.update(
            PaneRendererCustomData {
                bars: vec![],
                bar_spacing: 6.0,
                visible_range: None,
                conflation_factor: 1.0,
            },
            &view.default_options(),
        );
        assert_eq!(view.updates, 1);

        let mut target = vec![];
        let converter = |price: BarPrice| Some(Coordinate::new(price.value()));
        view.renderer()
            .draw(&mut target, &converter, true, Some(&7));
        assert_eq!(target, ["true:Some(7)"]);
    }
}
