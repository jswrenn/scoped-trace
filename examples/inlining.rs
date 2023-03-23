use scoped_trace::Trace;

#[allow(clippy::redundant_closure)]
fn main() {
    let (_, trace) = Trace::root(|| foo());
    println!("{trace}");
}

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
