use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SeriesKind {
    Line,
    Candlesticks,
    Histogram,
    PriceVolume,
}
impl SeriesKind {
    pub(super) fn candlesticks(self) -> bool {
        matches!(self, Self::Candlesticks | Self::PriceVolume)
    }
    pub(super) fn histogram(self) -> bool {
        self == Self::Histogram
    }
    pub(super) fn volume(self) -> bool {
        self == Self::PriceVolume
    }
}

pub(super) fn options(kind: LineType, style: LineStyle, markers: bool) -> SeriesOptionsMap {
    SeriesOptionsMap::Line(SeriesOptions {
        common: SeriesOptionsCommon {
            last_value_visible: false,
            title: "Demo".into(),
            price_scale_id: None,
            series_last_value_mode: None,
            visible: true,
            hit_test_tolerance: 3.,
            price_line_visible: false,
            price_line_source: PriceLineSource::LastBar,
            price_line_width: LineWidth::One,
            price_line_color: String::new(),
            price_line_style: LineStyle::Solid,
            price_format: PriceFormat::BuiltIn(PriceFormatBuiltIn {
                kind: PriceFormatBuiltInType::Price,
                precision: 2,
                min_move: 0.01,
                base: Some(100.),
            }),
            base_line_visible: false,
            base_line_color: String::new(),
            base_line_width: LineWidth::One,
            base_line_style: LineStyle::Solid,
            autoscale_info_provider: None,
            conflation_threshold_factor: None,
        },
        style: LineStyleOptions {
            color: "#2196f3".into(),
            line_style: style,
            line_width: LineWidth::Three,
            line_type: kind,
            line_visible: true,
            point_markers_visible: markers,
            point_markers_radius: None,
            crosshair_marker_visible: true,
            crosshair_marker_radius: 4.,
            crosshair_marker_border_color: String::new(),
            crosshair_marker_background_color: String::new(),
            crosshair_marker_border_width: 2.,
            last_price_animation: LastPriceAnimationMode::Disabled,
        },
    })
}
pub(super) fn bar(
    config: &DisplayConfig,
    index: usize,
    adjustment: f64,
) -> BuiltInSeriesDataItem<Time> {
    let value = config.value(index, adjustment);
    BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
        time: Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.)),
        value,
        color: Some(
            if (index / 20).is_multiple_of(2) {
                "#2196f3"
            } else {
                "#f59e0b"
            }
            .into(),
        ),
        custom_values: None,
    }))
}
impl Demo {
    pub(super) fn history_bar(&self, index: i64, volume: bool) -> BuiltInSeriesDataItem<Time> {
        let mut item = if volume {
            self.volume_bar(index.unsigned_abs() as usize, 0.)
        } else {
            self.bar(index.unsigned_abs() as usize, 0.)
        };
        let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
        match &mut item {
            BuiltInSeriesDataItem::Line(LineDataItem::Data(row)) => row.time = time,
            BuiltInSeriesDataItem::Line(LineDataItem::Whitespace(row)) => row.time = time,
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(row)) => row.time = time,
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Whitespace(row)) => {
                row.time = time
            }
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(row)) => row.time = time,
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Whitespace(row)) => row.time = time,
            _ => unreachable!("demo supports line, candle and histogram rows"),
        }
        item
    }
    pub(super) fn volume_bar(&self, index: usize, adjustment: f64) -> BuiltInSeriesDataItem<Time> {
        let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
        BuiltInSeriesDataItem::Histogram(if index % 17 == 5 {
            HistogramDataItem::Whitespace(WhitespaceData {
                time,
                custom_values: None,
            })
        } else {
            HistogramDataItem::Data(HistogramData {
                time,
                value: 10000. + (index % 11) as f64 * 2300. + adjustment * 1000.,
                color: Some(if index % 3 == 1 { "#ef5350" } else { "#26a69a" }.into()),
                custom_values: None,
            })
        })
    }
    pub(super) fn sync_volume(&mut self) {
        if !self.series_kind.volume() {
            return;
        }
        if self.registered {
            if !self.volume_registered {
                let pane = self.model.add_pane();
                let SeriesOptionsMap::Line(mut line) = options(self.kind, self.style, false) else {
                    unreachable!()
                };
                line.common.price_format = PriceFormat::BuiltIn(PriceFormatBuiltIn {
                    kind: PriceFormatBuiltInType::Volume,
                    precision: 0,
                    min_move: 1.,
                    base: None,
                });
                self.model
                    .register_series(
                        VOLUME_ID,
                        SeriesType::Histogram,
                        SeriesOptionsMap::Histogram(SeriesOptions {
                            common: line.common,
                            style: HistogramStyleOptions {
                                color: "#26a69a".into(),
                                base: 0.,
                            },
                        }),
                        pane,
                        PriceScalePosition::Right,
                    )
                    .unwrap();
                self.model.set_pane_stretch_factor(0, 2.).unwrap();
                self.model.set_pane_stretch_factor(pane, 1.).unwrap();
                self.volume_registered = true;
            }
            let data = (self.history_start..self.count as i64)
                .map(|i| self.history_bar(i, true))
                .collect();
            let update = self
                .layer
                .set_series_data(VOLUME_ID, SeriesType::Histogram, data)
                .unwrap();
            self.model.apply_data_update(update).unwrap();
        } else if self.volume_registered {
            let update = self.layer.remove_series(VOLUME_ID).unwrap();
            self.model.remove_series(VOLUME_ID, update).unwrap();
            self.volume_registered = false;
        }
    }
    pub(super) fn series_type(&self) -> SeriesType {
        if self.series_kind.histogram() {
            SeriesType::Histogram
        } else if self.series_kind.candlesticks() {
            SeriesType::Candlestick
        } else {
            SeriesType::Line
        }
    }
    pub(super) fn series_options(&self) -> SeriesOptionsMap {
        let line = options(self.kind, self.style, self.markers);
        if self.series_kind.histogram() {
            let SeriesOptionsMap::Line(line) = line else {
                unreachable!()
            };
            return SeriesOptionsMap::Histogram(SeriesOptions {
                common: line.common,
                style: HistogramStyleOptions {
                    color: "#2196f3".into(),
                    base: 0.,
                },
            });
        }
        if !self.series_kind.candlesticks() {
            return line;
        }
        let SeriesOptionsMap::Line(line) = line else {
            unreachable!()
        };
        let (up, down) = if self.markers {
            ("rgba(38,166,154,0.35)", "rgba(239,83,80,0.35)")
        } else {
            ("#26a69a", "#ef5350")
        };
        SeriesOptionsMap::Candlestick(SeriesOptions {
            common: line.common,
            style: CandlestickStyleOptions {
                up_color: up.into(),
                down_color: down.into(),
                wick_visible: self.kind == LineType::Simple,
                border_visible: self.style == LineStyle::Solid,
                border_color: "#378658".into(),
                border_up_color: up.into(),
                border_down_color: down.into(),
                wick_color: "#737375".into(),
                wick_up_color: up.into(),
                wick_down_color: down.into(),
            },
        })
    }
    pub(super) fn bar(&self, index: usize, adjustment: f64) -> BuiltInSeriesDataItem<Time> {
        if self.series_kind.histogram() {
            let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
            return BuiltInSeriesDataItem::Histogram(if index % 13 == 5 {
                HistogramDataItem::Whitespace(WhitespaceData {
                    time,
                    custom_values: None,
                })
            } else {
                HistogramDataItem::Data(HistogramData {
                    time,
                    value: self.config.value(index, adjustment),
                    color: index.is_multiple_of(19).then(|| "#f59e0b".into()),
                    custom_values: None,
                })
            });
        }
        if !self.series_kind.candlesticks() {
            return bar(&self.config, index, adjustment);
        }
        let time = Time::from(UtcTimestamp::new(1_700_000_000. + index as f64 * 60.));
        if index % 13 == 5 {
            return BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Whitespace(
                WhitespaceData {
                    time,
                    custom_values: None,
                },
            ));
        }
        let close = self.config.value(index, adjustment);
        let span = 2. * self.config.multiplier;
        let open = close
            + match index % 3 {
                0 => -span,
                1 => span,
                _ => 0.,
            };
        BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
            time,
            open,
            high: open.max(close) + span,
            low: open.min(close) - span,
            close,
            color: index.is_multiple_of(19).then(|| "#2196f3".into()),
            border_color: index.is_multiple_of(19).then(|| "#f59e0b".into()),
            wick_color: index.is_multiple_of(19).then(|| "#ddd".into()),
            custom_values: None,
        }))
    }
}
