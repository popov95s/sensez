use super::*;
use std::fs;

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// A small generated project: Python and TypeScript modules with calls, branches, comments and imports.
fn generate(root: &Path, rng: &mut Rng) -> Vec<PathBuf> {
    let words = [
        "alpha", "beta", "gamma", "delta", "order", "price", "user", "token",
    ];
    let mut files = Vec::new();
    for n in 0..2 + rng.below(6) {
        let name = words[rng.below(words.len())];
        let (path, body) = match rng.below(2) {
            0 => (
                root.join(format!("m{n}.py")),
                format!(
                    "import os\n\n# {name} helper\ndef {name}_{n}(a, b):\n    if a > {}:\n        return a + b\n    return os.getcwd()\n",
                    rng.below(50)
                ),
            ),
            _ => (
                root.join(format!("m{n}.ts")),
                format!(
                    "/** {name} */\nexport function {name}{n}(a: number, b: number[]): number {{\n  return b.filter((x) => x > {}).length + a;\n}}\n",
                    rng.below(50)
                ),
            ),
        };
        fs::write(&path, body).unwrap();
        files.push(path);
    }
    files.sort();
    files
}

fn project(files: &[PathBuf]) -> ProjectInputs {
    crate::spine::cache::load_project(files).unwrap()
}

/// The full report of a fresh scan, the reference every cached scan must match.
fn fresh_report(root: &Path) -> serde_json::Value {
    serde_json::to_value(crate::analyze_path(root, None).unwrap().0).unwrap()
}

/// The full report of a scan child, which reads and writes the parse cache.
fn cached_report(root: &Path) -> serde_json::Value {
    serde_json::to_value(
        crate::pipeline::analyze_path_for_child(root, None)
            .unwrap()
            .0,
    )
    .unwrap()
}

#[test]
fn warm_parses_restore_exactly_what_a_fresh_parse_produces() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..25 {
        let tmp = tempfile::tempdir().unwrap();
        let files = generate(tmp.path(), &mut rng);
        let fresh = fresh_report(tmp.path());

        let cold = cached_report(tmp.path());
        let warm = cached_report(tmp.path());

        assert_eq!(cold, fresh);
        assert_eq!(warm, fresh);
        assert_eq!(parse(tmp.path(), &project(&files), 7).1.reusable, 0);
        assert_eq!(
            parse(tmp.path(), &project(&files), 7).1.reusable,
            files.len()
        );
    }
}

#[test]
fn an_edited_file_misses_and_the_rest_hit() {
    let tmp = tempfile::tempdir().unwrap();
    let files = generate(tmp.path(), &mut Rng(11));
    cached_report(tmp.path());
    parse(tmp.path(), &project(&files), 7);
    fs::write(&files[0], "def edited():\n    return 1\n").unwrap();

    let (_, stats) = parse(tmp.path(), &project(&files), 7);

    assert_eq!(stats.reusable, files.len() - 1);
    assert_eq!(cached_report(tmp.path()), fresh_report(tmp.path()));
}

#[test]
fn a_config_or_build_change_misses_every_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let files = generate(tmp.path(), &mut Rng(12));
    parse(tmp.path(), &project(&files), 7);

    let (_, other_config) = parse(tmp.path(), &project(&files), 8);
    assert_eq!(other_config.reusable, 0);

    let sources = project(&files).sources;
    let current = DiskCache::new(tmp.path(), 7);
    let next_release = DiskCache {
        dir: current.dir.clone(),
        salt: current
            .salt
            .replacen(env!("CARGO_PKG_VERSION"), "999.0.0", 1),
    };
    assert!(current.read(&sources[0]).is_some());
    assert!(next_release.read(&sources[0]).is_none());
}

/// Truncate the first source's entry and flip a byte in the second's.
fn damage_two_entries(cache: &DiskCache, sources: &[SourceFile]) {
    let truncated = cache.entry_path(&sources[0].path, sources[0].content_hash);
    let bytes = fs::read(&truncated).unwrap();
    fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();
    let flipped = cache.entry_path(&sources[1].path, sources[1].content_hash);
    let mut bytes = fs::read(&flipped).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    fs::write(&flipped, &bytes).unwrap();
}

#[test]
fn a_corrupt_or_truncated_entry_falls_back_to_a_fresh_parse() {
    let tmp = tempfile::tempdir().unwrap();
    let files = generate_at_least(tmp.path(), 2);
    let signature = crate::config::model::Config::load_for_scan(tmp.path())
        .0
        .signature();
    let fresh = fresh_report(tmp.path());
    cached_report(tmp.path());
    let cache = DiskCache::new(tmp.path(), signature);
    let sources = project(&files).sources;

    damage_two_entries(&cache, &sources);
    assert_eq!(cached_report(tmp.path()), fresh);

    damage_two_entries(&cache, &sources);
    let (_, stats) = parse(tmp.path(), &project(&files), signature);
    assert_eq!(stats.reusable, files.len() - 2);
    assert_eq!(
        parse(tmp.path(), &project(&files), signature).1.reusable,
        files.len()
    );
}

#[test]
fn concurrent_writers_leave_complete_entries() {
    let tmp = tempfile::tempdir().unwrap();
    generate(tmp.path(), &mut Rng(14));
    let fresh = fresh_report(tmp.path());

    let results: Vec<serde_json::Value> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| cached_report(tmp.path())))
            .collect();
        workers.into_iter().map(|w| w.join().unwrap()).collect()
    });

    assert!(results.iter().all(|result| *result == fresh));
    assert_eq!(cached_report(tmp.path()), fresh);
    let leftovers = entry_files(&tmp.path().join(".sensez/parse-cache"))
        .into_iter()
        .filter(|(_, _, path)| path.extension().is_some_and(|ext| ext == "tmp"))
        .count();
    assert_eq!(leftovers, 0);
}

fn generate_at_least(root: &Path, count: usize) -> Vec<PathBuf> {
    let mut rng = Rng(13);
    loop {
        let files = generate(root, &mut rng);
        if files.len() >= count {
            return files;
        }
    }
}

#[test]
fn eviction_drops_stale_entries_and_keeps_the_current_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let files = generate(tmp.path(), &mut Rng(15));
    parse(tmp.path(), &project(&files), 7);
    for (n, file) in files.iter().enumerate() {
        fs::write(file, format!("def changed_{n}():\n    return {n}\n")).unwrap();
    }
    let sources = project(&files).sources;
    parse(tmp.path(), &project(&files), 7);
    let cache = DiskCache::new(tmp.path(), 7);
    assert_eq!(entry_files(&cache.dir).len(), 2 * files.len());

    cache.evict(&sources, 1);

    let kept: Vec<PathBuf> = entry_files(&cache.dir)
        .into_iter()
        .map(|(_, _, path)| path)
        .collect();
    assert_eq!(kept.len(), files.len());
    assert!(sources
        .iter()
        .all(|source| kept.contains(&cache.entry_path(&source.path, source.content_hash))));
}
