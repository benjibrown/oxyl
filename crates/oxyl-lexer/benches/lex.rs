// lex benchmarks
//
// three diff workloads with diff levels of complexity
// prose - just like text heavy pgs 
// commands - control seq heavy stuff 
// mixed - table, math etc - more realistic

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use oxyl_lexer::Lexer;

fn prose_input() -> String {
    let para = "The quick brown fox jumps over the lazy dog.".repeat(100);
    (0..200).map(|_| para.as_str()).collect::<Vec<_>>().join("\n\n")
}

fn commands_input() -> String {
    let one = "\\section{intro} \\emph{hello} \\cite{abc} \\textbf{world} ";
    one.repeat(5000)
}

fn mixed_input() -> String {
    let mut s = String::with_capacity(200_000);
    for _ in 0..500 {
        s.push_str("\\begin{tabular}{cc}\n");
        for i in 0..20 {
            s.push_str(&format!("row {i} & col $x^{i}$ \\\\\n"));
        }
        s.push_str("\\end{tabular}\n\n");
        s.push_str("Plain prose paragraph mixed in between table blocs.\n\n");
    }
    s
}
fn bench_lex(c: &mut Criterion) {
    for (name, input) in [
        ("prose", prose_input()),
        ("commands", commands_input()),
        ("mixed", mixed_input()),
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
