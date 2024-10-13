use std::collections::HashMap;

use pest::iterators::Pair;

use crate::{ParseError, Rule};

use super::*;

/// A complete unit of code, like a source file.
#[derive(Default, Debug)]
pub struct Document<'src> {
    pub imports: HashMap<&'src str, SpannedImport<'src>>,
    pub funcs: HashMap<&'src str, SpannedFuncDef<'src>>,
}

impl<'src> TryFrom<Pair<'src, Rule>> for Document<'src> {
    type Error = ParseError<'src>;

    fn try_from(value: Pair<'src, Rule>) -> ParseResult<Self> {
        let mut document = Document::default();

        // document  =  { SOI ~ statement* ~ EOI }
        for statement in value.into_inner() {
            match statement.as_rule() {
                Rule::func_def => {
                    let new = SpannedFuncDef::try_from(statement)?;

                    if let Some(old) = document.funcs.insert(new.name.text, new.clone()) {
                        return Err(ParseError::DuplicateFuncDef(old, new));
                    }
                }
                Rule::import => {
                    let new = SpannedImport::try_from(statement)?;

                    if let Some(old) = document.imports.insert(new.alias, new.clone()) {
                        return Err(ParseError::DumplicateImport(old, new));
                    }
                }
                Rule::EOI => {}
                _ => return Err(ParseError::UnexpectedFieldType),
            }
        }

        Ok(document)
    }
}
