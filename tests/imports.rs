#![feature(assert_matches)]
use std::assert_matches::assert_matches;

use funcad::{parse_all, FQPath, ParseError};
use typed_arena::Arena;

mod util;
use util::FileSet;

/// Importing past the path of main should result in an error.
#[test]
fn import_past_entry_errors() {
    let mut set = FileSet::default();
    set.insert("main", "import ../a");
    set.insert("a", "");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let result = parse_all(&arena, &entry, |s| set.get_source(s));

    assert_matches!(result, Err(ParseError::ImportNotInDir(_)));
}

/// `..` should import from parent directory.
#[test]
fn import_parent_dir_ok() {
    let mut set = FileSet::default();
    set.insert("main", "import a/a");
    set.insert("a/a", "import ../b");
    set.insert("b", "");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let result = parse_all(&arena, &entry, |s| set.get_source(s));

    assert_matches!(result, Ok(_));
}

/// Importing the same file multiple times should parse to only one document.
#[test]
fn import_same_file_doesnt_duplicate() {
    let mut set = FileSet::default();
    set.insert("main", "import a\nimport b");
    set.insert("a", "import b");
    set.insert("b", "");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let result = parse_all(&arena, &entry, |s| set.get_source(s));

    assert_matches!(result, Ok(_));
    let doc = result.unwrap();
    assert!(doc.len() == 3);
}

/// Importing a missing file should result in a ParseError.
#[test]
fn import_missing_errors() {
    let mut set = FileSet::default();
    set.insert("main".into(), "import a".into());

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let result = parse_all(&arena, &entry, |s| set.get_source(s));

    assert_matches!(result, Err(ParseError::IO(_)));
}
