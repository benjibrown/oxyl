use oxyl_diagnostics::{Diagnostic, Source};
use oxyl_lexer::Lexer;
use oxyl_parser::Parser;

// unix convetion (sysexists.h) - 0 for success, 1 for the operation 
// itself failing, 2 for the user invoking anything incorrectly. 
// Split out so shell scripts can tell if you had a syntax err in ur file 
// without having to parse stderr.
const EXIT_OK: i32 = 0;
const EXIT_COMPILE: i32 = 1;
const EXIT_USAGE: i32 = 2;

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
            "--version" | "-V" => {
                println!("oxyl {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            other if other.starts_with('-') => {
                eprintln!("oxyl: unknown flag '{other}'. Try --help.");
                std::process::exit(EXIT_USAGE);
            }
            other => {
                if file.is_some() {
                    eprintln!("oxyl: too many positional arguments. Try --help.");
                    std::process::exit(EXIT_USAGE);
                }
                file = Some(other.to_owned());
            }
        }
    }


    let path = match file {
        Some(p) => p,
        None => {
            print_help();
            std::process::exit(EXIT_USAGE);
        }
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("oxyl: could not read {path}: {e}");
            std::process::exit(EXIT_COMPILE);
        }
    };

    // Build the source view once so every diagnostic shares its line index
    // and so renderings include the filename to get --> file:line:col
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
        std::process::exit(if had_error { EXIT_COMPILE } else { EXIT_OK });
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
        std::process::exit(if had_error { EXIT_COMPILE} else { EXIT_OK });
    }
    
    if had_error {
        std::process::exit(EXIT_COMPILE);
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
    println!("  --version, -V   Print the oxyl version and exit");
    println!("  --help, -h      Print this help message");
}

