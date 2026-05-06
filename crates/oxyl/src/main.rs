use oxyl_diagnostics::Diagnostic;
use oxyl_lexer::Lexer;

fn main() {
    let path = std::env::args().nth(1);

    let Some(path) = path else {
        eprintln!("usage: oxyl <file.tex>");
        std::process::exit(1);
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("oxyl: could not read {path}: {e}");
            std::process::exit(1);
        }
    };

    let result = Lexer::new(&src).tokenise();

    // Print any lex errors before showing tokens. 
    for e in &result.errors {
        let d: Diagnostic = e.clone().into();
        eprintln!("{d}");
    }

    println!("{} token(s) in {path}", result.tokens.len());
    for tok in &result.tokens {
        println!("  {tok}");
    }

    if result.has_errors() {
        std::process::exit(1);
    }
}
