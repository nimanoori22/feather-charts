//! Lazy grid render-data construction from time- and price-scale mark snapshots.

use crate::{
    model::{
        grid::{GridOptions, UpdateType},
        ihorz_scale_behavior::TimeMark,
        price_scale::PriceMark,
    },
    renderers::grid_renderer::{GridMark, GridRenderer, GridRendererData},
};

#[derive(Clone, Debug)]
pub struct GridPaneView {
    invalidated: bool,
    renderer: GridRenderer,
    refresh_generation: u64,
}

impl Default for GridPaneView {
    fn default() -> Self {
        Self::new()
    }
}

impl GridPaneView {
    pub fn new() -> Self {
        Self {
            invalidated: true,
            renderer: GridRenderer::default(),
            refresh_generation: 0,
        }
    }

    pub fn update(&mut self, _kind: UpdateType) {
        self.invalidated = true;
    }

    pub fn is_invalidated(&self) -> bool {
        self.invalidated
    }

    pub fn refresh_generation(&self) -> u64 {
        self.refresh_generation
    }

    pub fn refresh(
        &mut self,
        options: &GridOptions,
        price_marks: &[PriceMark],
        time_marks: &[TimeMark],
    ) {
        if !self.invalidated {
            return;
        }
        self.renderer.set_data(Some(GridRendererData {
            vertical_lines_visible: options.vert_lines.visible,
            vertical_line_color: options.vert_lines.color.clone(),
            vertical_line_style: options.vert_lines.style,
            time_marks: time_marks
                .iter()
                .map(|mark| GridMark {
                    coordinate: mark.coordinate.into(),
                })
                .collect(),
            horizontal_lines_visible: options.horz_lines.visible,
            horizontal_line_color: options.horz_lines.color.clone(),
            horizontal_line_style: options.horz_lines.style,
            price_marks: price_marks.to_vec(),
        }));
        self.invalidated = false;
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
    }

    pub fn renderer(&self) -> &GridRenderer {
        &self.renderer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{coordinate::Coordinate, price_scale::PriceMark},
        renderers::draw_line::LineStyle,
    };

    fn time_mark(coordinate: f64) -> TimeMark {
        TimeMark {
            coordinate,
            label: String::new(),
            weight: crate::model::time_data::TickMarkWeightValue::new(0),
            need_align_coordinate: false,
        }
    }

    #[test]
    fn lazily_builds_a_snapshot_with_only_mark_coordinates() {
        let mut view = GridPaneView::new();
        let options = GridOptions::default();
        let prices = [PriceMark {
            coord: Coordinate::new(20.0),
            label: "ignored by grid".into(),
            logical: 4.0,
        }];
        let times = [time_mark(10.0)];
        view.refresh(&options, &prices, &times);
        let data = view.renderer().data().unwrap();
        assert_eq!(
            data.time_marks,
            vec![GridMark {
                coordinate: 10.0.into()
            }]
        );
        assert_eq!(data.price_marks, prices);
        assert_eq!(view.refresh_generation(), 1);

        view.refresh(&options, &[], &[]);
        assert_eq!(view.refresh_generation(), 1);
        view.update(UpdateType::Data);
        view.refresh(&options, &[], &[]);
        assert_eq!(view.refresh_generation(), 2);
        assert!(view.renderer().data().unwrap().time_marks.is_empty());
        assert_eq!(LineStyle::Solid, options.vert_lines.style);
    }
}
