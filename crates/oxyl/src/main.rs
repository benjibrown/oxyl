use oxyl_diagnostics::{Diagnostic, DiagSpan};
use oxyl_lexer::Lexer;
use oxyl_parser::Parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse flags and positional argument.
    let mut dump_tokens = false;
    let mut dump_ast = false; 
    let mut file: Option<String> = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--dump-tokens" => dump_tokens = true, 
            "--dump-ast" => dump_ast = true,
            "--help" | "-h" => {
                print_help();
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("oxyl: unknown flag '{other}'. Try --help.");
                std::process::exit(1);
            }
            other => {
                if file.is_some() {
                    eprintln!("oxyl: too many positional arguments. Try --help");
                    std::process::exit(1);
                }
                file = Some(other.to_owned());
            }
        }
    }

    let path = match file {
        Some(p) => p,
        None => {
            print_help();
            std::process::exit(1);
        }
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("oxyl: could not read {path}: {e}");
            std::process::exit(1);
        }
    };

    // --- Lex. --- 
    let lex_result = Lexer::new(&src).tokenise();
    let mut had_error = false;

    for e in &lex_result.errors {
        let d: Diagnostic = e.clone().into();
        print_diagnostic(&d, &src);
        had_error = true;
    }

    if dump_tokens {
        println!("=== tokens ({}) ===", lex_result.tokens.len());
        for tok in &lex_result.tokens {
            println!("  {:>6}..{:<6}  {}", tok.span.start, tok.span.end, tok.kind);
        }
        if had_error {
            std::process::exit(1);
        }
        return;
    }

    // Parse.
    let parse_result = Parser::new(lex_result.tokens).parse();

    for d in &parse_result.errors {
        print_diagnostic(d, &src);
        had_error = true;
    }

    if dump_ast {
        println!("=== AST ({} top-lexel AST node(s)) ===", parse_result.document.body.len());
        for node in &parse_result.document.body {
            println!("  {node:?}");
        }
        if had_error {
            std::process::exit(1);
        }
        return;
    }
    
    if had_error {
        std::process::exit(1);
    }

    // Success: print a brief summary.
    let node_count = parse_result.document.body.len();
    println!("ok: parsed {node_count} top-level node(s) from {path}");
}

fn print_help() {
    println!("oxyl - a LaTeX compiler (work in progress)");
    println!();
    println!("USAGE:");
    println!(" oxyl [FLAGS] <file.tex>");
    println!();
    println!("FLAGS:");
    println!("  --dump-tokens   Print every token with its byte span, then exit");
    println!("  --dump-ast      Print the parsed AST nodes, then exit");
    println!("  --help, -h      Print this help message");
}

/// Print a diagnostic with an inline source extract if possible.
fn print_diagnostic(d: &Diagnostic, src: &str) {
    let mut enriched = d.clone();

    // If the diagnostic has a span but no source hint yet, extract the 
    // relevant line from the source and attach it.
    if let (Some(span), None) = (&d.span, &d.source_hint) {
        let hint = extract_line(src, span.start);
        enriched = enriched.with_source_hint(hint);
    }

    // Also add a span if the diagnostic was created without one but we can 
    // reocver a position from the message (best effort for parser errors 
    // that have the location in the message text).
    if enriched.span.is_none() {
        if let Some(span) = parse_span_from_message(&d.message) {
            let hint = extract_line(src, span.start);
            enriched = enriched.with_span(span).with_source_hint(hint);
        }
    }

    eprintln!("{enriched}");
}

/// Extract the source line containing `byte_pos`.
fn extract_line(src: &str, byte_pos: usize) -> String {
    let safe_pos = byte_pos.min(src.len().saturating_sub(1));
    let line_start = src[..safe_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = src[safe_pos..]
        .find('\n')
        .map(|i| safe_pos + i)
        .unwrap_or(src.len());
    src[line_start..line_end].to_owned()
}

/// Best-effort: parse a `N..M` span out of a message like "unclosed '{' at 3..4"
fn parse_span_from_message(msg: &str) -> Option<DiagSpan> {
    let at = msg.rfind("at ")?;
    let rest = &msg[at + 3..];
    let dot2 = rest.find("..")?;
    let start: usize = rest[..dot2].trim().parse().ok()?;
    let end: usize = rest[dot2 + 2..].trim().parse().ok()?;
    Some(DiagSpan::new(start, end))
}
