use std::{
    collections::HashMap,
    io::{self, Cursor},
};

use funcad::*;

#[derive(Default)]
pub(crate) struct FileSet(HashMap<FQPath, String>);

/// A convenience struct for testing multiple files.
impl FileSet {
    pub(crate) fn get_source(&self, name: &FQPath) -> Result<Cursor<&[u8]>, ParseError> {
        self.0
            .get(name)
            .map(|src| Cursor::new(src.as_bytes()))
            .ok_or(ParseError::IO(io::Error::new(io::ErrorKind::NotFound, "")))
    }

    pub(crate) fn insert(&mut self, name: &str, src: &str) {
        self.0.insert(
            FQPath(name.split("/").map(str::to_string).collect()),
            src.to_string(),
        );
    }
}
