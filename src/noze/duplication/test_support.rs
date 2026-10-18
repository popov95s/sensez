use crate::report::CloneClass;
use crate::spine::parser::{parse_file, ParsedFile};
use std::fs;

pub(crate) fn write_files(dir: &std::path::Path, files: &[(&str, &str)]) -> Vec<ParsedFile> {
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

/// Whether some clone class has occurrences in both `left` and `right`.
pub(crate) fn pairs(clones: &[CloneClass], left: &str, right: &str) -> bool {
    clones.iter().any(|class| {
        let has = |name: &str| class.occurrences.iter().any(|o| o.file.ends_with(name));
        has(left) && has(right)
    })
}
