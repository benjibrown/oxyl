use oxyl_diagnostics::{Diagnostic, DiagSpan};
use oxyl_lexer::Lexer;
use oxyl_parser::Parser;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: oxyl <file.tex>");
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

    // Lex.
    let lex_result = Lexer::new(&src).tokenise();
    let mut had_error = false;

    for e in &lex_result.errors {
        let d: Diagnostic = e.clone().into();
        print_diagnostic(&d, &src);
        had_error = true;
    }

    // Parse.
    let parse_result = Parser::new(lex_result.tokens).parse();

    for d in &parse_result.errors {
        print_diagnostic(d, &src);
        had_error = true;
    }

    if had_error {
        std::process::exit(1);
    }

    // Success: print a brief summary.
    let node_count = parse_result.document.body.len();
    println!("ok: parsed {node_count} top-level node(s) from {path}");
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

    if enriched.span.is_none() {
        if let Some(span) = parse_span_from_message(&d.message) {
            let hint = extract_line(src, span.start);
            enriched = enriched.with_span(span).with_source_hint(hint);
        }
    }

    eprintln!("{enriched}");
}

/// Extract the source line containing `byte_pos
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
