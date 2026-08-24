use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

const MAX_FILE_RESULTS: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub parent: String,
}

/// Searches the macOS Spotlight metadata index by file name.
pub fn search(query: &str) -> std::io::Result<Vec<FileEntry>> {
    let output = Command::new("/usr/bin/mdfind")
        .args(["-0", "-name", query])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "mdfind exited with {}",
            output.status
        )));
    }

    Ok(parse_paths(&output.stdout, MAX_FILE_RESULTS))
}

fn parse_paths(output: &[u8], limit: usize) -> Vec<FileEntry> {
    let mut seen = HashSet::new();
    output
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .filter_map(|path| {
            let path = String::from_utf8_lossy(path).into_owned();
            let path_ref = Path::new(&path);
            if path_ref
                .ancestors()
                .any(|ancestor| ancestor.extension().is_some_and(|ext| ext == "app"))
                || !seen.insert(path.clone())
            {
                return None;
            }
            Some(FileEntry {
                name: path_ref.file_name()?.to_string_lossy().into_owned(),
                parent: path_ref
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path,
            })
        })
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_null_delimited_paths_and_skips_app_contents() {
        let output = b"/Users/me/report.pdf\0/Applications/Notes.app\0/Applications/Notes.app/Contents/Info.plist\0/Users/me/report.pdf\0";
        assert_eq!(
            parse_paths(output, 10),
            vec![FileEntry {
                name: "report.pdf".into(),
                path: "/Users/me/report.pdf".into(),
                parent: "/Users/me".into(),
            }]
        );
    }

    #[test]
    fn respects_result_limit() {
        let output = b"/tmp/one\0/tmp/two\0/tmp/three\0";
        assert_eq!(parse_paths(output, 2).len(), 2);
    }
}
