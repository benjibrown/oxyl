use oxyl_diagnostics::{Diagnostic, Source};
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
                    eprintln!("oxyl: too many positional arguments. Try --help.");
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

    // Build the source view once so every diagnostic shares its line index
    // and so renderings include the filename for awesome diagnostics.
    let source = Source::with_name(&src, &path);

    // --- Lex. --- 
    let lex_result = Lexer::new(&src).tokenise();
    let mut had_error = false;

    for e in &lex_result.errors {
        let d: Diagnostic = e.clone().into();
        eprintln!("{}", d.render(&source));
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
        eprintln!("{}", d.render(&source));
        had_error = true;
    }

    if dump_ast {
        println!("=== AST ({} top-level node(s)) ===", parse_result.document.body.len());
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
    println!("  oxyl [FLAGS] <file.tex>");
    println!();
    println!("FLAGS:");
    println!("  --dump-tokens   Print every token with its byte span, then exit");
    println!("  --dump-ast      Print the parsed AST nodes, then exit");
    println!("  --help, -h      Print this help message");
}

