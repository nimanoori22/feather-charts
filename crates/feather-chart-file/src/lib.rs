//! Version 1 self-contained numeric chart documents. No GUI dependencies.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug)]
pub struct DocumentError(pub String);
impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DocumentError {}
fn err(message: impl Into<String>) -> DocumentError {
    DocumentError(message.into())
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartDocument {
    pub format: String,
    pub version: u32,
    pub title: String,
    pub x_axis: XAxis,
    pub y_axis: Axis,
    pub series: Vec<Series>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub label: String,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub precision: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XAxis {
    pub label: String,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub precision: u8,
    pub spacing: Spacing,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    Linear,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesKind {
    Line,
    Histogram,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Series {
    pub name: String,
    pub kind: SeriesKind,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "read_bar_width"
    )]
    pub bar_width: Option<f64>,
    pub points: Vec<[f64; 2]>,
}
fn read_bar_width<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<f64>, D::Error> {
    f64::deserialize(input).map(Some)
}
fn text(value: &str, field: &str, limit: usize) -> Result<(), DocumentError> {
    if value.len() > limit {
        return Err(err(format!("{field}: exceeds {limit} bytes")));
    }
    Ok(())
}
/// Rejects extents whose subtraction, padding or reciprocal cannot be represented.
pub fn validate_extent(min: f64, max: f64) -> bool {
    let span = max - min;
    let padding = if span == 0. {
        (min.abs() * 0.05).max(1.)
    } else {
        span * 0.1
    };
    min.is_finite()
        && max.is_finite()
        && span.is_finite()
        && (span == 0. || (span > 0. && (1. / span).is_finite()))
        && (min - padding).is_finite()
        && (max + padding).is_finite()
}
impl ChartDocument {
    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.format != "feather-chart" {
            return Err(err("format: expected feather-chart"));
        }
        if self.version != 1 {
            return Err(err(format!("version: unsupported {}", self.version)));
        }
        text(&self.title, "title", 1024)?;
        for (label, unit, precision, field) in [
            (
                &self.x_axis.label,
                &self.x_axis.unit,
                self.x_axis.precision,
                "x_axis",
            ),
            (
                &self.y_axis.label,
                &self.y_axis.unit,
                self.y_axis.precision,
                "y_axis",
            ),
        ] {
            text(label, field, 1024)?;
            if let Some(unit) = unit {
                text(unit, field, 1024)?;
            }
            if precision > 12 {
                return Err(err(format!("{field}.precision: must be 0..12")));
            }
        }
        if self.series.is_empty() || self.series.len() > 32 {
            return Err(err("series: require 1..32 series"));
        }
        if self.metadata.len() > 64 {
            return Err(err("metadata: exceeds 64 entries"));
        }
        for (key, value) in &self.metadata {
            text(key, "metadata key", 4096)?;
            text(value, "metadata value", 4096)?;
        }
        let mut count = 0;
        let mut bounds = [f64::INFINITY, f64::NEG_INFINITY, 0_f64, 0_f64];
        for (i, s) in self.series.iter().enumerate() {
            let field = format!("series[{i}]");
            text(&s.name, &format!("{field}.name"), 1024)?;
            if s.name.trim().is_empty() {
                return Err(err(format!("{field}.name: must not be empty")));
            }
            if let Some(c) = &s.color
                && (c.len() != 7
                    || !c.starts_with('#')
                    || !c[1..].bytes().all(|c| c.is_ascii_hexdigit()))
            {
                return Err(err(format!("{field}.color: expected #RRGGBB")));
            }
            let half = match (s.kind, s.bar_width) {
                (SeriesKind::Line, None) => 0.,
                (SeriesKind::Histogram, Some(w)) if w.is_finite() && w > 0. && w / 2. > 0. => {
                    w / 2.
                }
                _ => {
                    return Err(err(format!(
                        "{field}.bar_width: required positive finite width for histogram; forbidden for line"
                    )));
                }
            };
            count += s.points.len();
            if count > 250_000 {
                return Err(err("series.points: exceeds 250000 total points"));
            }
            let mut previous = None;
            for (j, &[x, y]) in s.points.iter().enumerate() {
                if !x.is_finite() || !y.is_finite() || previous.is_some_and(|p| x <= p) {
                    return Err(err(format!(
                        "{field}.points[{j}]: require finite numbers and strictly increasing x"
                    )));
                }
                if !((x - half).is_finite() && (x + half).is_finite())
                    || (half > 0. && (x - half == x || x + half == x))
                {
                    return Err(err(format!(
                        "{field}.points[{j}]: unrepresentable histogram edges"
                    )));
                }
                previous = Some(x);
                bounds[0] = bounds[0].min(x - half);
                bounds[1] = bounds[1].max(x + half);
                bounds[2] = bounds[2].min(y);
                bounds[3] = bounds[3].max(y);
            }
        }
        if count > 0
            && (!validate_extent(bounds[0], bounds[1]) || !validate_extent(bounds[2], bounds[3]))
        {
            return Err(err("series.points: unrepresentable numeric extent"));
        }
        Ok(())
    }
}
pub fn read_document(input: impl Read) -> Result<ChartDocument, DocumentError> {
    let mut bytes = Vec::new();
    input
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| err(format!("read: {e}")))?;
    if bytes.len() > MAX_BYTES {
        return Err(err("document exceeds 32 MiB"));
    }
    let doc: ChartDocument =
        serde_json::from_slice(&bytes).map_err(|e| err(format!("JSON: {e}")))?;
    doc.validate()?;
    Ok(doc)
}
pub fn write_document(
    mut output: impl Write,
    document: &ChartDocument,
) -> Result<(), DocumentError> {
    document.validate()?;
    let bytes = serde_json::to_vec_pretty(document).map_err(|e| err(format!("JSON: {e}")))?;
    if bytes.len() > MAX_BYTES {
        return Err(err("document exceeds 32 MiB"));
    }
    output
        .write_all(&bytes)
        .map_err(|e| err(format!("write: {e}")))
}
