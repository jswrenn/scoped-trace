use scoped_trace::Trace;

fn foo() {
    bar();
    baz();
}

#[inline(always)]
fn bar() {
    Trace::leaf();
}

#[inline(always)]
fn baz() {
    Trace::leaf();
}

#[allow(clippy::redundant_closure)]
#[test]
fn inlining() {
    let (_, trace) = Trace::root(|| foo());
    assert_eq!(
        format!("{trace}"),
        format!(
            "\
╼ inlining::inlining::{{{{closure}}}} at {file}:21:37
  ├╼ inlining::foo at {file}:4:5
  │  └╼ inlining::bar at {file}:10:5
  └╼ inlining::foo at {file}:5:5
     └╼ inlining::baz at {file}:15:5",
            file = format!(
                concat!(env!("CARGO_MANIFEST_DIR"), "{}", file!()),
                std::path::MAIN_SEPARATOR
            )
        )
    );
}
