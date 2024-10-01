#![feature(assert_matches)]

pub mod ast;
use ast::*;

mod error;
pub use error::ParseError;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
};

use pest::Parser;

use typed_arena::Arena;

mod lang {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "funcad.pest"]
    pub(crate) struct FCParser;
}
pub(crate) use lang::{FCParser, Rule};

/// Parses a single document.
///
/// `src` should contain the source to parse.
///
/// Imported files won't be parsed and only parsing validation is performed so
/// the code cannot be guaranteed to be correct.
pub fn parse_document<'src>(src: &'src str) -> Result<Document, ParseError> {
    let pair = FCParser::parse(Rule::document, src)?.try_next()?;

    Document::try_from(pair)
}

/// Parse `main` and any imports recursively.
///
/// `main` is the name of the first file to fetch with the get_source function.
/// It should be in format of funcad imports (identifiers separated by "/").
///
/// `get_source` should return a reader to a source file relative to the path
/// containing main.
pub fn parse_all<'src, R: Read, F: Fn(&str) -> Result<R, ParseError<'src>>>(
    source_arena: &'src Arena<u8>,
    main: String,
    get_source: F,
) -> Result<BTreeMap<String, Document<'src>>, ParseError<'src>> {
    let mut to_parse = BTreeSet::new();
    let mut parsed = BTreeMap::new();

    to_parse.insert(main.to_string());

    while let Some(current) = to_parse.pop_first() {
        if !parsed.contains_key(&current) {
            let src = alloc_src(source_arena, get_source(&current.clone())?)?;

            let doc = parse_document(src)?;

            let current_dir = file_dir(&current);
            for import in doc.imports.values() {
                to_parse.insert(import.file_path(&current_dir)?);
            }

            parsed.insert(current, doc);
        }
    }

    Ok(parsed)
}

/// Read and parse `main` and any imports recursively.
pub fn parse_all_files<'src>(
    source_arena: &'src Arena<u8>,
    main: &Path,
) -> Result<BTreeMap<String, Document<'src>>, ParseError<'src>> {
    let (Some(path), Some(main_name)) = (main.parent(), main.file_name()) else {
        return Err(ParseError::MainRoot);
    };

    parse_all(source_arena, main_name.to_string_lossy().into(), |file| {
        let src = File::open(path.join(file))?;
        Ok(src)
    })
}

fn file_dir(file: &str) -> String {
    // Since files in import directives are only separated by forward slashes,
    // we can extract the dir with just a split.
    let mut parts: Vec<_> = file.split("/").collect();
    parts.pop();
    parts.join("/")
}

fn alloc_src<'src, R: Read>(
    source_arena: &'src Arena<u8>,
    mut reader: R,
) -> Result<&'src str, ParseError<'src>> {
    let mut src_string = String::new();
    reader.read_to_string(&mut src_string)?;
    Ok(source_arena.alloc_str(&src_string))
}
