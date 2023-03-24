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
        "\
╼ inlining::inlining::{{closure}} at /home/ubuntu/projects/scoped-trace/tests/inlining.rs:21:37
  ├╼ inlining::foo at /home/ubuntu/projects/scoped-trace/tests/inlining.rs:4:5
  │  └╼ inlining::bar at /home/ubuntu/projects/scoped-trace/tests/inlining.rs:10:5
  └╼ inlining::foo at /home/ubuntu/projects/scoped-trace/tests/inlining.rs:5:5
     └╼ inlining::baz at /home/ubuntu/projects/scoped-trace/tests/inlining.rs:15:5"
    );
}
