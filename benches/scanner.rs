#[macro_use]
extern crate criterion;

use criterion::Criterion;
use lang::scanner::Scanner;

static SRC: &str = r#"(def long 5.0) ; this will be a very long source
        (def (f x) (* x x))
        (def (laugh) (print "hahaha"))
        (print #:out file 5.4)
        ; just another comment about the code"#;

fn scan_src(src: &str) {
    let mut scanner = Scanner::new(SRC);

    let lexemes = scanner.collect::<Vec<_>>();
}

fn scanner_bench(c: &mut Criterion)
{
    c.bench_function("scan", |b| b.iter(|| scan_src(SRC)));
}

criterion::criterion_group!(benches, scanner_bench);
criterion::criterion_main!(benches);