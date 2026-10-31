//! Reproducible file -> chart -> renderer acceptance and 100k-point smoke probe.
#[path = "../src/chart.rs"]
mod chart;
use feather_charts::api::*;
use std::{path::PathBuf, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: verify INPUT.fchart OUTPUT.ppm [--wgpu] [--100k]".into());
    }
    let start = Instant::now();
    let mut document = feather_chart_file::read_document(std::fs::File::open(&args[0])?)?;
    if args.iter().any(|a| a == "--100k") {
        document.series.truncate(1);
        document.series[0].bar_width = None;
        document.series[0].kind = feather_chart_file::SeriesKind::Line;
        document.series[0].points = (0..100000)
            .map(|i| [i as f64, (i as f64 / 100.).sin()])
            .collect();
    }
    let parsed = start.elapsed();
    let points: usize = document.series.iter().map(|s| s.points.len()).sum();
    let mut candidate = chart::DocumentChart::new(document).map_err(std::io::Error::other)?;
    assert_eq!(
        candidate.chart.series_ids().count(),
        candidate.document.series.len()
    );
    assert!(!candidate.empty());
    let constructed = start.elapsed();
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines().filter(|l| l.starts_with("VmHWM:")) {
            println!("after_construction_{line}");
        }
    }
    candidate
        .chart
        .prepare(iced::Size::new(1000., 650.), 1., Instant::now())?;
    let prepared = start.elapsed();
    for i in 0..10 {
        candidate
            .chart
            .zoom_time((200. + i as f64 * 10.).into(), 0.1);
        candidate
            .chart
            .prepare(iced::Size::new(1000., 650.), 1., Instant::now())?;
    }
    let navigated = start.elapsed();
    let backend = if args.iter().any(|a| a == "--wgpu") {
        ScreenshotBackend::Wgpu
    } else {
        ScreenshotBackend::Software
    };
    let mut exporter = iced::futures::executor::block_on(ChartExporter::new(backend))?;
    let screenshot = candidate.chart.screenshot_of_prepared(Default::default())?;
    let image = exporter.render(&screenshot, Default::default())?;
    assert!(
        image
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] != p[1] || p[1] != p[2]),
        "no colored series pixels"
    );
    let path = PathBuf::from(&args[1]);
    use std::io::Write;
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    write!(
        out,
        "P6\n{} {}\n255\n",
        image.size().width,
        image.size().height
    )?;
    for pixel in image.rgba().as_chunks::<4>().0 {
        out.write_all(&pixel[..3])?;
    }
    println!(
        "backend={backend:?} points={points} parsed={parsed:?} constructed={constructed:?} prepared={prepared:?} navigation_10={:?}",
        navigated - prepared
    );
    Ok(())
}
