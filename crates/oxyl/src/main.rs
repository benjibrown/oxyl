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

    let tokens = Lexer::new(&src).tokenise();
    println!("{} token(s) in {path}", tokens.len());
    for tok in &tokens {
        println!("  {tok}");
    }
}
