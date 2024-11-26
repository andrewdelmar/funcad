#![feature(assert_matches)]
use std::assert_matches::assert_matches;

use funcad::*;
use typed_arena::Arena;

mod util;
use util::FileSet;

/// Make sure trig functions output something sensible.
#[test]
fn eval_trig() {
    let mut set = FileSet::default();
    set.insert("main", "a = Tan(theta)-Sin(theta)/Cos(theta)\ntheta = 27");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(
        eval_result,
        Ok(Value::Number(num)) if (num).abs() < 0.0001
    );
}

/// Tan is undefined in some places.
#[test]
fn tan_not_finite() {
    let mut set = FileSet::default();
    set.insert("main", "a = Tan(90)");

    let arena = Arena::new();
    let entry = FQPath(vec!["main".into()]);

    let parse_result = parse_all(&arena, &entry, |s| set.get_source(s));
    assert_matches!(parse_result, Ok(_));
    let doc_set = parse_result.unwrap();

    let eval_result = eval_function(&doc_set, &entry, "a");
    assert_matches!(
        eval_result,
        Err(EvalError {
            error_type: EvalErrorType::NumExprNotFinite,
            ..
        })
    );
}
