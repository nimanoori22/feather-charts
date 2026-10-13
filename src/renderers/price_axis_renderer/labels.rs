use super::*;
pub(super) fn prepare(
    m: &PriceAxisMeasurement,
    bounds: AxisBounds,
    ratio: PixelRatio,
    pane: bool,
) -> Vec<PriceAxisCommand> {
    if bounds.width <= 0. || bounds.height <= 0. {
        return vec![];
    }
    let snapshot = &m.snapshot;
    let o = &snapshot.renderer_options;
    let mut labels = snapshot.labels.clone();
    if snapshot.align_labels {
        crate::renderers::price_axis_labels::align_labels(
            &mut labels,
            snapshot.label_center.unwrap_or(bounds.height / 2.),
            bounds.height,
            f64::from(o.font_size + o.padding_top + o.padding_bottom),
        );
    }
    let right_aligned = (snapshot.side == PriceAxisSide::Left) != pane;
    let hratio = f64::from(ratio.horizontal);
    let vratio = f64::from(ratio.vertical);
    let mut commands = vec![];
    for (i, l) in labels.iter().enumerate() {
        let text = if pane { &l.pane_text } else { &l.axis.text };
        if text.is_empty() {
            continue;
        }
        let metric = if pane {
            &m.pane_metrics[i]
        } else {
            &m.label_metrics[i]
        };
        let tick = if pane { 0. } else { f64::from(o.tick_length) };
        let border = if pane { f64::from(o.border_size) } else { 0. };
        let bitmap_border = if border > 0. {
            (border * hratio).floor().max(1.)
        } else {
            0.
        };
        let tick_height = vratio.floor().max(1.);
        let mut height = source_round(
            f64::from(o.font_size + o.padding_top + o.padding_bottom + l.additional_padding)
                * vratio,
        );
        if height % 2. != tick_height % 2. {
            height += 1.;
        }
        let middle = source_round(l.render_coordinate * vratio) - (vratio * 0.5).floor();
        let top = (middle + tick_height / 2. - height / 2.).floor() / vratio;
        let width = source_round(
            (f64::from(o.border_size + o.padding_inner + o.padding_outer)
                + f64::from(metric.width).ceil()
                + tick)
                * hratio,
        ) / hratio;
        let inside = if right_aligned {
            bounds.width - bitmap_border / hratio
        } else {
            bitmap_border / hratio
        };
        commands.push(PriceAxisCommand::RoundedRectangle {
            rect: AxisRect {
                origin: Point {
                    x: if right_aligned {
                        inside - width
                    } else {
                        inside
                    },
                    y: top,
                },
                size: AxisBounds {
                    width,
                    height: height / vratio,
                },
            },
            radii: if right_aligned {
                [2., 0., 0., 2.]
            } else {
                [0., 2., 2., 0.]
            },
            color: l.axis.background.clone(),
        });
        if !pane && l.tick_visible.unwrap_or(snapshot.ticks_visible) {
            commands.push(PriceAxisCommand::Rectangle {
                rect: AxisRect {
                    origin: Point {
                        x: if right_aligned {
                            inside - source_round(tick * hratio) / hratio
                        } else {
                            inside
                        },
                        y: middle / vratio,
                    },
                    size: AxisBounds {
                        width: source_round(tick * hratio) / hratio,
                        height: tick_height / vratio,
                    },
                },
                color: l.axis.color.clone(),
            });
        }
        if pane && border > 0. {
            commands.push(PriceAxisCommand::Rectangle {
                rect: AxisRect {
                    origin: Point {
                        x: if right_aligned {
                            bounds.width - bitmap_border / hratio
                        } else {
                            0.
                        },
                        y: top,
                    },
                    size: AxisBounds {
                        width: bitmap_border / hratio,
                        height: height / vratio,
                    },
                },
                color: o.pane_background_color.clone(),
            });
        }
        let inside_media = if right_aligned {
            bounds.width - border
        } else {
            border
        };
        commands.push(PriceAxisCommand::Text {
            label: text.clone(),
            position: Point {
                x: if right_aligned {
                    inside_media - tick - f64::from(o.padding_inner) - border
                } else {
                    inside_media + tick + f64::from(o.padding_inner)
                },
                y: top
                    + height / vratio / 2.
                    + f64::from(
                        metric.actual_bounding_box_ascent.unwrap_or_default()
                            - metric.actual_bounding_box_descent.unwrap_or_default(),
                    ) / 2.,
            },
            alignment: if right_aligned {
                AxisTextAlignment::Right
            } else {
                AxisTextAlignment::Left
            },
            font_size: o.font_size,
            font_family: o.font_family.clone(),
            color: l.axis.color.clone(),
        });
    }
    commands
}
