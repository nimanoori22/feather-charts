use feather_chart_file::ChartDocument;
use std::path::{Path, PathBuf};
pub fn read(path: &Path) -> Result<ChartDocument, String> {
    std::fs::File::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))
        .and_then(|file| {
            feather_chart_file::read_document(file).map_err(|e| format!("{}: {e}", path.display()))
        })
}
pub fn arguments(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Option<PathBuf>, String> {
    let mut path = None;
    for arg in args {
        if arg == "--help" || arg == "-h" {
            return Err("Usage: feather-charts [document.fchart]".into());
        }
        if path.is_some() || arg.to_string_lossy().starts_with('-') {
            return Err("Usage: feather-charts [document.fchart] (one local path)".into());
        }
        path = Some(PathBuf::from(arg));
    }
    Ok(path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_and_errors() {
        assert!(arguments([]).unwrap().is_none());
        assert_eq!(
            arguments(["a b.fchart".into()]).unwrap().unwrap(),
            PathBuf::from("a b.fchart")
        );
        assert!(arguments(["a".into(), "b".into()]).is_err());
        assert!(
            read(Path::new("/nonexistent/fchart"))
                .unwrap_err()
                .contains("/nonexistent/fchart")
        );
    }
}
