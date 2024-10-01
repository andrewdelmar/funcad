#![feature(assert_matches)]
use std::{
    assert_matches::assert_matches,
    collections::BTreeMap,
    io::{self, Cursor},
};

use funcad::{parse_all, ParseError};
use typed_arena::Arena;

struct FileSet(BTreeMap<String, String>);

impl FileSet {
    fn get_source(&self, name: &str) -> Result<Cursor<&[u8]>, ParseError> {
        self.0
            .get(name)
            .map(|src| Cursor::new(src.as_bytes()))
            .ok_or(ParseError::IO(io::Error::new(
                io::ErrorKind::NotFound,
                name,
            )))
    }
}

/// Importing past the path of main should result in an error.
#[test]
fn import_past_entry_errors() {
    let mut map = BTreeMap::<String, String>::new();
    map.insert("main.fc".into(), "import ../a".into());
    map.insert("a.fc".into(), "".into());
    let set = FileSet(map);

    let arena = Arena::new();

    let result = parse_all(&arena, "main.fc".into(), |s| set.get_source(s));

    assert_matches!(result, Err(ParseError::ImportNotInDir(_)));
}

/// `..` should import from parent directory.
#[test]
fn import_parent_dir_ok() {
    let mut map = BTreeMap::<String, String>::new();
    map.insert("main.fc".into(), "import a/a".into());
    map.insert("a/a.fc".into(), "import ../b".into());
    map.insert("b.fc".into(), "".into());
    let set = FileSet(map);

    let arena = Arena::new();

    let result = parse_all(&arena, "main.fc".into(), |s| set.get_source(s));

    assert_matches!(result, Ok(_));
}

/// Importing the same file multiple times should parse to only one document.
#[test]
fn import_same_file_doesnt_duplicate() {
    let mut map = BTreeMap::<String, String>::new();
    map.insert("main.fc".into(), "import a\nimport b".into());
    map.insert("a.fc".into(), "import b".into());
    map.insert("b.fc".into(), "".into());
    let set = FileSet(map);

    let arena = Arena::new();

    let result = parse_all(&arena, "main.fc".into(), |s| set.get_source(s));

    assert_matches!(result, Ok(_));
    let doc = result.unwrap();
    assert!(doc.len() == 3);
}

/// Importing a missing file should result in a ParseError.
#[test]
fn import_missing_errors() {
    let mut map = BTreeMap::<String, String>::new();
    map.insert("main.fc".into(), "import a".into());
    let set = FileSet(map);

    let arena = Arena::new();

    let result = parse_all(&arena, "main.fc".into(), |s| set.get_source(s));

    assert_matches!(result, Err(ParseError::IO(_)));
}
