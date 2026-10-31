//! Synthetic producer, or validate any supplied document with --check PATH.
use feather_chart_file::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--check" {
        let doc = read_document(std::fs::File::open(&args[1])?)?;
        println!(
            "{}: {} series, {} points",
            doc.title,
            doc.series.len(),
            doc.series.iter().map(|s| s.points.len()).sum::<usize>()
        );
        return Ok(());
    }
    if args.len() != 1 {
        return Err("usage: write_benchmark OUTPUT.fchart | --check INPUT.fchart".into());
    }
    let mut doc = read_document(include_bytes!("../fixtures/minimal.fchart").as_slice())?;
    doc.title = "Synthetic demonstration benchmark".into();
    doc.metadata.insert(
        "data".into(),
        "synthetic demonstration; not measured".into(),
    );
    let path = std::path::Path::new(&args[0]);
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    if let Err(e) = write_document(&mut output, &doc)
        .and_then(|_| std::io::Write::flush(&mut output).map_err(|e| DocumentError(e.to_string())))
    {
        drop(output);
        std::fs::remove_file(path)?;
        return Err(e.into());
    }
    Ok(())
}
