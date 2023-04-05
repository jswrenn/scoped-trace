use std::pin::pin;

use scoped_trace::{Trace};

#[allow(clippy::redundant_closure)]
fn main() {
    let (_, trace) = Trace::capture(|| foo());
    println!("{trace}");
}

#[inline(never)]
fn foo() {
    bar();
}

#[inline(never)]
fn bar() {
    baz();
}

#[inline(never)]
fn baz() {
    Trace::leaf();
    println!("HI");
}
