use crate::spine::parser::{parse_file, ParsedFile};
use std::fs;

pub(super) fn write_files(dir: &std::path::Path, files: &[(&str, &str)]) -> Vec<ParsedFile> {
    for (name, body) in files {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    files
        .iter()
        .enumerate()
        .map(|(i, (name, _))| parse_file(&dir.join(name), i as u32).unwrap())
        .collect()
}
