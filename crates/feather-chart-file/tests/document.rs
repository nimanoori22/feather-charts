use feather_chart_file::*;
fn fixture() -> ChartDocument {
    read_document(include_bytes!("../fixtures/minimal.fchart").as_slice()).unwrap()
}
#[test]
fn fixtures_roundtrip() {
    for bytes in [
        include_bytes!("../fixtures/minimal.fchart").as_slice(),
        include_bytes!("../fixtures/benchmark.fchart").as_slice(),
    ] {
        let doc = read_document(bytes).unwrap();
        let mut out = Vec::new();
        write_document(&mut out, &doc).unwrap();
        assert_eq!(read_document(out.as_slice()).unwrap(), doc);
    }
}
#[test]
fn rejects_invalid_semantics_before_writing() {
    let original = fixture();
    for mutate in [
        |d: &mut ChartDocument| d.version = 2,
        |d: &mut ChartDocument| d.series.clear(),
        |d: &mut ChartDocument| d.series[0].points[1][0] = 1.,
        |d: &mut ChartDocument| d.series[0].points[0][1] = f64::NAN,
        |d: &mut ChartDocument| d.series[0].bar_width = Some(1.),
        |d: &mut ChartDocument| d.x_axis.precision = 13,
        |d: &mut ChartDocument| d.series[0].color = Some("red".into()),
        |d: &mut ChartDocument| d.title = "a".repeat(1025),
        |d: &mut ChartDocument| d.series[0].points = vec![[1., 0.]; 250001],
    ] {
        let mut d = original.clone();
        mutate(&mut d);
        let mut out = Vec::new();
        assert!(write_document(&mut out, &d).is_err());
        assert!(out.is_empty());
    }
    let mut d = original;
    d.series[0].points.clear();
    assert!(d.validate().is_ok());
}
#[test]
fn strict_json_and_bounded_read() {
    let s = String::from_utf8(include_bytes!("../fixtures/minimal.fchart").to_vec()).unwrap();
    for invalid in [
        s.replace("\"version\": 1", "\"version\": 2"),
        s.replace("\"version\": 1", "\"unknown\": 1, \"version\": 1"),
        s.replace("0.421", "null"),
        s.replace("0.421", "1e999"),
        s.replace("\"line\"", "\"unknown\""),
        s.replace("0.421", "0.421, 3"),
    ] {
        assert!(read_document(invalid.as_bytes()).is_err(), "{invalid}");
    }
    assert!(read_document(std::io::repeat(b' ').take((MAX_BYTES + 1) as u64)).is_err());
}
use std::io::Read;
#[test]
fn io_failures_are_reported() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken"))
        }
    }
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(
        read_document(Broken)
            .unwrap_err()
            .to_string()
            .contains("broken")
    );
    assert!(write_document(Broken, &fixture()).is_err());
}

#[test]
fn budgets_histogram_edges_and_extents() {
    let base = fixture();
    let mut d = base.clone();
    d.series = vec![base.series[0].clone(); 33];
    assert!(d.validate().is_err());
    let mut d = base.clone();
    for i in 0..65 {
        d.metadata.insert(i.to_string(), String::new());
    }
    assert!(d.validate().is_err());
    let mut d = base.clone();
    d.metadata.insert("key".into(), "v".repeat(4097));
    assert!(d.validate().is_err());
    let mut d = base.clone();
    d.series[0].kind = SeriesKind::Histogram;
    assert!(d.validate().is_err());
    for w in [0., -1., f64::INFINITY, f64::from_bits(1)] {
        d.series[0].bar_width = Some(w);
        assert!(d.validate().is_err());
    }
    d.series[0].bar_width = Some(0.5);
    assert!(d.validate().is_ok());
    d.series[0].points = vec![[1e300, 1.]];
    assert!(d.validate().is_err());
    let mut d = base;
    d.series[0].points = vec![[-f64::MAX, 0.], [f64::MAX, 1.]];
    assert!(d.validate().is_err());
}

#[test]
fn single_point_fit_must_have_representable_padding() {
    let mut d = fixture();
    d.series[0].points = vec![[f64::MAX, 1.]];
    assert!(d.validate().is_err());
    d.series[0].points = vec![[1e100, 1.]];
    assert!(d.validate().is_ok());
}

#[test]
fn line_rejects_even_null_bar_width() {
    let original =
        String::from_utf8(include_bytes!("../fixtures/minimal.fchart").to_vec()).unwrap();
    let invalid = original.replace(
        "\"kind\": \"line\"",
        "\"kind\": \"line\", \"bar_width\": null",
    );
    assert!(read_document(invalid.as_bytes()).is_err());
}
