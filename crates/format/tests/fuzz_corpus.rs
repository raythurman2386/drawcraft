//! Stable replay of `fuzz/corpus/from_json/*` without nightly/libFuzzer.
//!
//! `from_json` must return Ok or Err for every corpus file — never panic.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

#[test]
fn fuzz_corpus_does_not_panic() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fuzz")
        .join("corpus")
        .join("from_json");
    assert!(
        root.is_dir(),
        "missing fuzz corpus directory at {}",
        root.display()
    );
    let mut count = 0;
    for entry in fs::read_dir(&root).expect("read corpus dir") {
        let entry = entry.expect("corpus entry");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        count += 1;
        let bytes = fs::read(&path).expect("read corpus file");
        match std::str::from_utf8(&bytes) {
            Ok(text) => {
                let _ = drawcraft_format::from_json(text);
            }
            Err(_) => {
                // Non-UTF-8 inputs are rejected by the fuzz target before from_json.
            }
        }
    }
    assert!(count > 0, "corpus was empty");
}
