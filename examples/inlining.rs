use scoped_tree_trace::Trace;

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
