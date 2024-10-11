#![feature(assert_matches)]

pub mod ast;
use ast::*;

mod error;
pub use error::{EvalError, ParseError};
use error::{EvalResult, ParseResult};

mod eval;
use eval::EvalCache;
pub use eval::Value;

use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
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

/// A collection of documents by path.
type DocSet<'src> = HashMap<FQPath, Document<'src>>;

/// Parse `main` and any imports recursively.
///
/// `get_source` should return a reader to a source file given an FQPath.
pub fn parse_all<'src, R, F>(
    source_arena: &'src Arena<u8>,
    main: &FQPath,
    get_source: F,
) -> ParseResult<'src, DocSet<'src>>
where
    R: Read,
    F: Fn(&FQPath) -> ParseResult<'src, R>,
{
    let mut to_parse = BTreeSet::new();
    let mut parsed = HashMap::new();

    to_parse.insert(main.clone());

    while let Some(current) = to_parse.pop_first() {
        if !parsed.contains_key(&current) {
            let src = alloc_src(source_arena, get_source(&current)?)?;

            let doc = parse_document(src)?;

            for import in doc.imports.values() {
                to_parse.insert(current.import_path(&import)?);
            }

            parsed.insert(current, doc);
        }
    }

    Ok(parsed)
}

/// Read and parse the file `main` and any imports recursively.
pub fn parse_all_files<'src>(
    source_arena: &'src Arena<u8>,
    main: &Path,
) -> ParseResult<'src, DocSet<'src>> {
    let (Some(path), Some(main_name)) = (main.parent(), main.file_stem()) else {
        return Err(ParseError::InvalidMain);
    };

    parse_all(
        source_arena,
        &FQPath(vec![main_name.to_string_lossy().into()]),
        |source_path| {
            let src = File::open(source_path.file_path(path))?;
            Ok(src)
        },
    )
}

/// Evaluate a single function in `doc_path` by name.
pub fn eval_function<'src>(
    docs: &DocSet<'src>,
    doc_path: &FQPath,
    func_name: &str,
) -> EvalResult<'src, Value> {
    let mut cache = EvalCache::new(docs);
    cache.eval_func_by_name(doc_path, func_name)
}

/// A "fully qualified" path to a document or function.
///
/// An FQPath is not interchangable with a [`Path`] and is only fully qualified
/// in the sense that it is relative to the entry point (the directory of main)
/// and not an individual document.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct FQPath(pub Vec<String>);

impl Display for FQPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}

impl FQPath {
    /// Returns the `FQPath` of an import in a doc with path this path.
    pub(crate) fn import_path<'src>(
        &self,
        import: &Import<'src>,
    ) -> Result<FQPath, ParseError<'src>> {
        let mut new_parts = self.0.clone();
        new_parts.pop();

        for part in import.file.split("/") {
            match part {
                ".." => match new_parts.pop() {
                    None => return Err(ParseError::ImportNotInDir(*import)),
                    _ => {}
                },
                ident => {
                    new_parts.push(ident.to_string());
                }
            }
        }

        Ok(Self(new_parts))
    }

    /// Returns a Pathbuf pointing to a .fc file with this path.
    pub(crate) fn file_path(&self, base: &Path) -> PathBuf {
        return base.join(format!("{}.fc", self.0.join("/")));
    }
}

fn alloc_src<'src, R: Read>(
    source_arena: &'src Arena<u8>,
    mut reader: R,
) -> Result<&'src str, ParseError<'src>> {
    let mut src_string = String::new();
    reader.read_to_string(&mut src_string)?;
    Ok(source_arena.alloc_str(&src_string))
}
