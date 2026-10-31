use feather_chart_file::{ChartDocument, SeriesKind};
use feather_charts::{
    api::*,
    model::{data_consumer::*, pane::PriceScalePosition, series_options::*},
};
pub struct DocumentChart {
    pub document: ChartDocument,
    pub chart: OptionsChart,
}
const COLORS: [&str; 6] = [
    "#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9",
];
impl DocumentChart {
    pub fn new(document: ChartDocument) -> Result<Self, String> {
        document.validate().map_err(|e| e.to_string())?;
        let mut chart =
            OptionsChart::new_numeric(document.x_axis.precision).map_err(|e| e.to_string())?;
        for (i, s) in document.series.iter().enumerate() {
            let color = s
                .color
                .clone()
                .unwrap_or_else(|| COLORS[i % COLORS.len()].into());
            let common = SeriesOptionsCommon {
                title: s.name.clone(),
                price_line_visible: false,
                last_value_visible: false,
                price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
                    kind: PriceFormatBuiltInType::Price,
                    precision: document.y_axis.precision.into(),
                    min_move: 10_f64.powi(-i32::from(document.y_axis.precision)),
                    base: None,
                }),
                ..Default::default()
            };
            let options = match s.kind {
                SeriesKind::Line => {
                    let mut o = LineSeriesOptions {
                        common,
                        ..Default::default()
                    };
                    o.style.color = color.clone();
                    SeriesOptionsMap::Line(o)
                }
                SeriesKind::Histogram => {
                    let mut o = HistogramSeriesOptions {
                        common,
                        ..Default::default()
                    };
                    o.style.color = color.clone();
                    SeriesOptionsMap::Histogram(o)
                }
            };
            let id = chart
                .add_series(options, 0, PriceScalePosition::Right)
                .map_err(|e| e.to_string())?;
            if let Some(width) = s.bar_width {
                chart
                    .set_numeric_histogram_width(id, width)
                    .map_err(|e| e.to_string())?;
            }
            let data = s
                .points
                .iter()
                .map(|&[x, y]| {
                    let time = HorizontalPrice::new(x).expect("validated numeric key");
                    match s.kind {
                        SeriesKind::Line => {
                            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                                time,
                                value: y,
                                color: None,
                                custom_values: None,
                            }))
                        }
                        SeriesKind::Histogram => BuiltInSeriesDataItem::Histogram(
                            HistogramDataItem::Data(HistogramData {
                                time,
                                value: y,
                                color: None,
                                custom_values: None,
                            }),
                        ),
                    }
                })
                .collect();
            chart.set_data(id, data).map_err(|e| e.to_string())?;
        }
        chart.fit_content();
        Ok(Self { document, chart })
    }
    pub fn empty(&self) -> bool {
        self.document.series.iter().all(|s| s.points.is_empty())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_document_prepares_and_invalid_candidates_are_rejected() {
        let mut document = feather_chart_file::read_document(
            include_bytes!("../fixtures/benchmark.fchart").as_slice(),
        )
        .unwrap();
        for series in &mut document.series {
            series.points.clear();
        }
        let mut empty = DocumentChart::new(document.clone()).unwrap();
        assert!(empty.empty());
        empty
            .chart
            .prepare(iced::Size::new(320., 300.), 1.5, std::time::Instant::now())
            .unwrap();
        assert!(empty.chart.frame().plot.hit_series.is_empty());
        document.version = 2;
        assert!(DocumentChart::new(document).is_err());
    }
    #[test]
    fn adapter_keeps_original_keys_and_prepares_numeric_geometry() {
        let d = feather_chart_file::read_document(
            include_bytes!("../fixtures/benchmark.fchart").as_slice(),
        )
        .unwrap();
        let mut c = DocumentChart::new(d.clone()).unwrap();
        c.chart
            .prepare(iced::Size::new(800., 500.), 1., std::time::Instant::now())
            .unwrap();
        assert_eq!(c.chart.series_ids().count(), 2);
        assert_eq!(c.document, d);
        assert_eq!(c.chart.frame().plot.hit_series.len(), 2);
        for (prepared, authored) in c.chart.frame().plot.hit_series.iter().zip(&d.series) {
            let rows = c.chart.series_data(prepared.series).unwrap();
            let points: Vec<_> = rows
                .into_iter()
                .map(|row| match row {
                    BuiltInSeriesDataItem::Line(LineDataItem::Data(d)) => [d.time.value(), d.value],
                    BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(d)) => {
                        [d.time.value(), d.value]
                    }
                    _ => panic!("unexpected adapter series kind"),
                })
                .collect();
            assert_eq!(points, authored.points);
        }

        let x = [1., 2., 100.].map(|v| c.chart.numeric_to_coordinate(v).unwrap());
        assert!(((x[2] - x[1]) / (x[1] - x[0]) - 98.).abs() < 1e-8);
    }
}
