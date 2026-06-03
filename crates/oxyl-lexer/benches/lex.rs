// lex benchmarks
//
// three diff workloads with diff levels of complexity

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use oxyl_lexer::Lexer;

fn prose_input() -> String {
    let para = "The quick brown fox jumps over the lazy dog.".repeat(100);
    (0..200).map(|_| para.as_str()).collect::<Vec<_>>().join("\n\n")
}

fn bench_lex(c: &mut Criterion) {
    for (name, input) in [
        ("prose", prose_input()),
    ] {
        let mut group = c.benchmark_group(format!("lex/{name}"));
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_function("tokenise", |b| {
            b.iter(|| {
                let r = Lexer::new(black_box(&input)).tokenise();
                black_box(r);
            });
        });
        group.finish();
    }
}

criterion_group!(benches, bench_lex);
criterion_main!(benches);
