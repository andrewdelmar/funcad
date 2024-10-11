#![feature(assert_matches)]
use std::assert_matches::assert_matches;

use funcad::*;
use typed_arena::Arena;

mod util;
use util::FileSet;

/// Literally 1+1.
#[test]
fn eval_call_simple_ok() {
    let mut set = FileSet::default();
    set.insert("main", "a = 1 + 1");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(eval_result, Ok(Value::Number(2.)))
}

/// Calling a function with zero args.
#[test]
fn eval_call_no_args_ok() {
    let mut set = FileSet::default();
    set.insert("main", "a = b\nb = 1 + 1");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(eval_result, Ok(Value::Number(2.)))
}

/// Calling a function with args, shadowing a function name.
#[test]
fn eval_call_args_shadow_ok() {
    let mut set = FileSet::default();
    set.insert("main", "a = b(1)\nb(a) = a + 1");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(eval_result, Ok(Value::Number(2.)))
}

/// Calling a function with defaults and only supplying some named args.
#[test]
fn eval_call_args_shadow_and_default_ok() {
    let mut set = FileSet::default();
    set.insert("main", "a = b(a=1)\nb(a=1, b=1) = a + b");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(eval_result, Ok(Value::Number(2.)))
}

/// Calling a function in an import.
#[test]
fn eval_call_in_import_ok() {
    let mut set = FileSet::default();
    set.insert("main", "import b\na = b.c + 1");
    set.insert("b", "c = 1");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(eval_result, Ok(Value::Number(2.)))
}
