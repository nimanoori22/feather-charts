//! Linux thread diagnostics for the fixed-price public-update acceptance fixture.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/update_counters.rs"]
mod update_counters;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {

    use super::update_counters::*;
    use feather_charts::{
        api::Chart,
        model::{
            data_consumer::{
                BuiltInSeriesDataItem, CandlestickData, CandlestickDataItem, HistogramData,
                HistogramDataItem, LineData, LineDataItem, TimedData,
            },
            horz_scale_behavior_time::types::{Time, UtcTimestamp},
            pane::PriceScalePosition,
            series_options::SeriesOptionsMap,
            time_data::LogicalRange,
        },
        ui::{multi_pane_frame::IcedMultiPaneChartFrame, price_axis::AxisFontResolver},
    };
    use std::{ffi::c_long, hint::black_box, time::Instant};

    #[derive(Clone, Copy, Debug)]
    struct Sample {
        index: usize,
        wall_ns: u64,
        cpu_ns: u64,
        minor: c_long,
        major: c_long,
        voluntary: c_long,
        involuntary: c_long,
        hardware: [Option<HardwareCount>; 3],
    }
    fn row(
        candle: bool,
        histogram: bool,
        index: usize,
        revision: usize,
        fixed: bool,
    ) -> BuiltInSeriesDataItem<Time> {
        let time = UtcTimestamp::new(1_700_000_000. + index as f64 * 60.).into();
        let value = if fixed {
            100.
        } else {
            100. + (index as f64 * 0.03).sin() * 10.
        } + revision as f64 * 0.0001;
        if candle {
            BuiltInSeriesDataItem::Candlestick(CandlestickDataItem::Data(CandlestickData {
                time,
                open: value - 1.,
                high: value + 2.,
                low: value - 2.,
                close: value,
                color: None,
                border_color: None,
                wick_color: None,
                custom_values: None,
            }))
        } else if histogram {
            BuiltInSeriesDataItem::Histogram(HistogramDataItem::Data(HistogramData {
                time,
                value,
                color: None,
                custom_values: None,
            }))
        } else {
            BuiltInSeriesDataItem::Line(LineDataItem::Data(LineData {
                time,
                value,
                color: None,
                custom_values: None,
            }))
        }
    }
    fn percentile(values: impl Iterator<Item = u64>, percentile: usize) -> u64 {
        let mut values: Vec<_> = values.collect();
        values.sort_unstable();
        values[(values.len() * percentile / 100).min(values.len() - 1)]
    }

    pub fn run() {
        assert_eq!(std::mem::size_of::<Rusage>(), 144);
        assert_eq!(std::mem::size_of::<Timespec>(), 16);
        let args: Vec<_> = std::env::args().collect();
        let argument = |name: &str| {
            args.iter()
                .position(|arg| arg == name)
                .map(|at| args.get(at + 1).expect("missing argument value").as_str())
        };
        let number = |name, fallback| {
            argument(name).map_or(fallback, |value| {
                value.parse::<usize>().expect("expected integer")
            })
        };
        let case = argument("--case").unwrap_or("append-line");
        assert!(
            [
                "append-line",
                "append-histogram",
                "latest-candle",
                "middle-line"
            ]
            .contains(&case)
        );
        let rows = number("--rows", 100_000);
        let sources = number("--sources", 16);
        let samples = number("--samples", 1000);
        let hardware = args.iter().any(|arg| arg == "--instructions");
        let mut counters: [Option<HardwareCounter>; 3] = std::array::from_fn(|index| {
            if !hardware {
                return None;
            }
            let (name, event) = EVENTS[index];
            match HardwareCounter::open(event) {
                Ok(counter) => Some(counter),
                Err(error) => {
                    eprintln!(
                        "--instructions event={name} unavailable for this user thread: {error}; kernel settings unchanged"
                    );
                    None
                }
            }
        });
        if hardware && counters.iter().all(Option::is_none) {
            return;
        }
        assert!(rows > 0 && sources > 0 && samples > 0);
        assert!(case != "middle-line" || (sources >= 2 && rows > samples + 40));
        let candle = case == "latest-candle";
        let histogram = case == "append-histogram";
        let mut chart = Chart::default();
        let mut ids = Vec::new();
        for source in 0..sources {
            let options = if candle {
                SeriesOptionsMap::Candlestick(Default::default())
            } else if histogram {
                SeriesOptionsMap::Histogram(Default::default())
            } else {
                SeriesOptionsMap::Line(Default::default())
            };
            let id = chart
                .add_series(options, 0, PriceScalePosition::Right)
                .unwrap();
            let count = if case == "middle-line" && source == 0 {
                0
            } else {
                rows
            };
            chart
                .set_data(
                    id,
                    (0..count)
                        .map(|index| row(candle, histogram, index, 0, index + 1 == rows))
                        .collect(),
                )
                .unwrap();
            ids.push(id);
        }
        chart.set_logical_range(LogicalRange {
            from: 20.0.into(),
            to: 100.0.into(),
        });
        let size = iced::Size::new(917., 870.80005);
        chart.prepare(size, 1.25, Instant::now()).unwrap();
        println!(
            "fixture case={case} rows={rows} sources={sources} samples={samples} warmup=20 size=917x870.80005 ratio=1.25 logical=20..100 fixed_last_price=true plot={:?} price_lines={} right_labels={}",
            chart.frame().layout.plot,
            chart.frame().plot.price_lines.len(),
            chart
                .frame()
                .plot
                .axes
                .as_ref()
                .map_or(0, |axes| axes.right.labels.len())
        );
        if hardware {
            println!(
                "hardware=user-only instructions/cycles/cache-misses; read intervals enclose CPU-clock reads plus wall interval and other hardware reads; enabled/running deltas expose multiplexing; counts are unscaled"
            );
        }
        println!(
            "measurement wall=only-public-update cpu=thread-clock-bracketing-wall-including-Instant-reads rusage=thread-before-and-after-clock-reads; no UI/GPU; raw tuple=(sample,wall_ns,cpu_ns,minor,major,voluntary,involuntary)"
        );
        let fonts = AxisFontResolver::default();
        let mut results = Vec::with_capacity(samples);
        for sample in 0..samples + 20 {
            let index = match case {
                "latest-candle" => rows - 1,
                "middle-line" => 20 + sample,
                _ => rows + sample,
            };
            let mut item = row(candle, histogram, index, sample, true);
            if case == "middle-line" {
                *item.time_mut() =
                    UtcTimestamp::new(1_700_000_000. + index as f64 * 60. + 30.).into();
            }
            let before = usage();
            let hardware_before: [Option<HardwareCount>; 3] =
                std::array::from_fn(|index| counters[index].as_mut().map(HardwareCounter::read));
            let cpu_before = thread_cpu_ns();
            let started = Instant::now();
            let updated = chart.update_data(ids[0], item, false);
            let finished = Instant::now();
            let cpu_after = thread_cpu_ns();
            let hardware_after: [Option<HardwareCount>; 3] =
                std::array::from_fn(|index| counters[index].as_mut().map(HardwareCounter::read));
            let after = usage();
            updated.unwrap();
            let measurement = Sample {
                index: sample,
                wall_ns: (finished - started).as_nanos().try_into().unwrap(),
                cpu_ns: cpu_after.checked_sub(cpu_before).unwrap(),
                minor: after.minor_faults - before.minor_faults,
                major: after.major_faults - before.major_faults,
                voluntary: after.voluntary_switches - before.voluntary_switches,
                involuntary: after.involuntary_switches - before.involuntary_switches,
                hardware: std::array::from_fn(|index| {
                    hardware_after[index]
                        .zip(hardware_before[index])
                        .map(|(after, before)| after.since(before))
                }),
            };
            let snapshot = chart.prepare_snapshot(size, 1.25, finished).unwrap();
            black_box(IcedMultiPaneChartFrame::from_snapshot(snapshot, &fonts, "#eee").unwrap());
            if sample >= 20 {
                results.push(measurement);
            }
        }
        let expected_rows = match case {
            "latest-candle" => rows,
            "middle-line" => samples + 20,
            _ => rows + samples + 20,
        };
        assert_eq!(
            chart.model().series(ids[0]).unwrap().bars().rows().len(),
            expected_rows
        );
        for sample in &results {
            println!(
                "raw ({},{},{},{},{},{},{})",
                sample.index,
                sample.wall_ns,
                sample.cpu_ns,
                sample.minor,
                sample.major,
                sample.voluntary,
                sample.involuntary
            );
            for (index, counts) in sample
                .hardware
                .iter()
                .enumerate()
                .filter_map(|(index, counts)| counts.map(|counts| (index, counts)))
            {
                println!(
                    "hardware event={} sample={} count={} enabled_ns={} running_ns={}",
                    EVENTS[index].0,
                    sample.index,
                    counts.count,
                    counts.enabled_ns,
                    counts.running_ns
                );
            }
        }
        for (index, _) in counters
            .iter()
            .enumerate()
            .filter(|(_, counter)| counter.is_some())
        {
            let counts: Vec<_> = results.iter().map(|s| s.hardware[index].unwrap()).collect();
            println!(
                "hardware_summary event={} median={} p95={} max={} enabled_ns={} running_ns={} multiplexed_samples={}",
                EVENTS[index].0,
                percentile(counts.iter().map(|s| s.count), 50),
                percentile(counts.iter().map(|s| s.count), 95),
                percentile(counts.iter().map(|s| s.count), 100),
                counts.iter().map(|s| s.enabled_ns).sum::<u64>(),
                counts.iter().map(|s| s.running_ns).sum::<u64>(),
                counts
                    .iter()
                    .filter(|s| s.running_ns < s.enabled_ns)
                    .count()
            );
        }
        println!(
            "summary case={case} rows={rows} sources={sources} wall_median_ns={} wall_p95_ns={} wall_max_ns={} cpu_median_ns={} cpu_p95_ns={} cpu_max_ns={} minor_faults={} major_faults={} voluntary_switches={} involuntary_switches={}",
            percentile(results.iter().map(|s| s.wall_ns), 50),
            percentile(results.iter().map(|s| s.wall_ns), 95),
            percentile(results.iter().map(|s| s.wall_ns), 100),
            percentile(results.iter().map(|s| s.cpu_ns), 50),
            percentile(results.iter().map(|s| s.cpu_ns), 95),
            percentile(results.iter().map(|s| s.cpu_ns), 100),
            results.iter().map(|s| s.minor).sum::<c_long>(),
            results.iter().map(|s| s.major).sum::<c_long>(),
            results.iter().map(|s| s.voluntary).sum::<c_long>(),
            results.iter().map(|s| s.involuntary).sum::<c_long>()
        );
        for faults in [false, true] {
            for switched in [false, true] {
                let group: Vec<_> = results
                    .iter()
                    .filter(|s| {
                        (s.minor + s.major > 0) == faults
                            && (s.voluntary + s.involuntary > 0) == switched
                    })
                    .collect();
                if !group.is_empty() {
                    println!(
                        "group faults={faults} switched={switched} samples={} wall_median_ns={} wall_p95_ns={} cpu_median_ns={} cpu_p95_ns={}",
                        group.len(),
                        percentile(group.iter().map(|s| s.wall_ns), 50),
                        percentile(group.iter().map(|s| s.wall_ns), 95),
                        percentile(group.iter().map(|s| s.cpu_ns), 50),
                        percentile(group.iter().map(|s| s.cpu_ns), 95)
                    );
                }
            }
        }
        results.sort_unstable_by_key(|s| std::cmp::Reverse(s.wall_ns));
        for sample in results.iter().take(20) {
            println!(
                "outlier {sample:?} wall_minus_cpu_ns={}",
                sample.wall_ns as i128 - sample.cpu_ns as i128
            );
        }
    }
}

fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    linux::run();
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    eprintln!("update_latency_probe thread counters currently support Linux x86_64 only");
}
