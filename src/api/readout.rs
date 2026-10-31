//! Optional host overlay, following Lightweight Charts' legend examples.
use super::{Chart, ChartBehavior, ChartMessage};
use crate::model::{
    plot_data::PlotRowLike, plot_list::MismatchDirection, series_data::SeriesPlotRow,
};

impl<B: ChartBehavior, M: Clone + 'static> Chart<B, M> {
    /// Current crosshair sample per visible built-in series, or latest values
    /// when the crosshair is absent. Missing samples show a dash, never stale data.
    pub fn crosshair_readout(&self) -> Vec<String> {
        let selected = self.model.crosshair().position().map(|p| p.index);
        self.series_ids()
            .filter_map(|id| {
                let series = self.model.series(id)?;
                if !series.visible() {
                    return None;
                }
                let row = match selected {
                    Some(index) => series.bars().search(index, MismatchDirection::None),
                    None => series.bars().last(),
                };
                if row.is_none() && selected.is_none() {
                    return None;
                }
                let title = &series.common_options().title;
                let title = if title.is_empty() {
                    format!("Series {}", id.value())
                } else {
                    title.clone()
                };
                let value = row.map_or_else(
                    || "—".into(),
                    |row| {
                        let v = row.values();
                        let formatter = series.formatter();
                        if matches!(row, SeriesPlotRow::Candlestick(_) | SeriesPlotRow::Bar(_)) {
                            format!(
                                "O: {}  H: {}  L: {}  C: {}",
                                formatter.format(v[0]),
                                formatter.format(v[1]),
                                formatter.format(v[2]),
                                formatter.format(v[3])
                            )
                        } else {
                            formatter.format(v[3])
                        }
                    },
                );
                Some(format!("{title}  {value}"))
            })
            .collect()
    }

    /// Ordinary chart with a passive readout inside the plot's top-left corner.
    /// The overlay does not capture pointer input or change chart layout.
    /// Chart-only screenshot exports continue to contain engine geometry only.
    pub fn element_with_readout(&self) -> iced::Element<'_, ChartMessage> {
        use iced::{
            Color, Fill, Padding,
            widget::{column, container, stack, text},
        };
        let lines = self.crosshair_readout();
        if lines.is_empty() {
            return self.element();
        }
        let plot = self.frame().layout.plot;
        let color = self
            .frame()
            .plot
            .axes
            .as_ref()
            .and_then(|a| {
                self.color_parser()
                    .rendering_rgba(&a.time.renderer_options.text_color)
                    .ok()
            })
            .map(|[r, g, b, a]| Color::from_rgba(r, g, b, a))
            .unwrap_or(Color::BLACK);
        let background = Color {
            a: 0.9,
            ..self.frame().plot.background
        };
        let readout =
            container(column(lines.into_iter().map(|line| text(line).size(12).into())).spacing(2))
                .padding(6)
                .max_width((plot.size.width - 16.).max(0.) as f32)
                .max_height((plot.size.height - 16.).max(0.) as f32)
                .clip(true)
                .style(move |_| container::Style {
                    text_color: Some(color),
                    background: Some(background.into()),
                    ..Default::default()
                });
        stack![
            self.element(),
            container(readout)
                .padding(
                    Padding::ZERO
                        .top(plot.origin.y as f32 + 8.)
                        .left(plot.origin.x as f32 + 8.)
                )
                .width(Fill)
                .height(Fill)
        ]
        .clip(true)
        .into()
    }
}
