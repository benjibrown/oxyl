use std::io::IsTerminal;

use oxyl_diagnostics::{Diagnostic, Source, Style};
use oxyl_lexer::Lexer;
use oxyl_parser::Parser;

mod dump;
mod help;

// unix convetion (sysexists.h) - 0 for success, 1 for the operation 
// itself failing, 2 for the user invoking anything incorrectly. 
// Split out so shell scripts can tell if you had a syntax err in ur file 
// without having to parse stderr.
const EXIT_OK: i32 = 0;
const EXIT_COMPILE: i32 = 1;
const EXIT_USAGE: i32 = 2;

/// users choice of when to use ANSI colour. `Auto` resolves to `Ansi` if 
/// stderr is a terminal and the `NO_COLOR` env var is unset, otherwise to 
/// `Plain`. Matches convention used by other stuff like rustc and 
/// cargo etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // parse flags and positional arg
    let mut dump_tokens = false;
    let mut dump_ast = false;
    let mut show_help = false;
    let mut show_version = false;
    let mut color = ColorChoice::Auto;
    let mut file: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--dump-tokens" => dump_tokens = true, 
            "--dump-ast" => dump_ast = true,
            "--help" | "-h" => show_help = true,
            "--version" | "-V" => show_version = true,
            // support --color= and --color (without eqls)
            s if s.starts_with("--color=") => {
                let value = &s["--color=".len()..];
                color = match parse_color(value) {
                    Some(c) => c,
                    None => {
                        eprintln!("oxyl: --color expects auto, always, or never (got '{value}').");
                        std::process::exit(EXIT_USAGE);
                    }
                };
            }
            "--color" => {
                i += 1;
                let value = match args.get(i) {
                    Some(v) => v.as_str(),
                    None => {
                        eprintln!("oxyl: --color needs a value (auto, always, never).");
                        std::process::exit(EXIT_USAGE);
                    }
                };
                color = match parse_color(value) {
                    Some(c) => c,
                    None => {
                        eprintln!("oxyl: --color expects auto, always, or never (got '{value}').");
                        std::process::exit(EXIT_USAGE);
                    }
                };
            }
            // convenience alias for --color=never like most 
            // modern cli tools have - cos we like to optimise frfr
            "--no-color" => color = ColorChoice::Never,
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
        i += 1;
    }

    // two streams so two diff tty checks. diagnostics write to stderr, dump
    // output writes to stdout - so check both individually to ensure
    // all cases are actually covered so stderr can have no color whilst
    // stdout does have color
    let err_style = resolve_style(color, std::io::stderr().is_terminal());
    let out_style = resolve_style(color, std::io::stdout().is_terminal());

    if show_help {
        help::print_help(out_style);
        return;
    }
    if show_version {
        println!("oxyl {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let path = match file {
        Some(p) => p,
        None => {
            help::print_help(out_style);
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

    // build the source view once so every diagnostic shares its line index
    // and so renderings include the filename to get --> file:line:col
    let source = Source::with_name(&src, &path);

    // lexer !
    let lex_result = Lexer::new(&src).tokenise();
    let mut had_error = false;

    for e in &lex_result.errors {
        let d: Diagnostic = e.clone().into();
        eprintln!("{}", d.render_styled(&source, err_style));
        had_error = true;
    }

    if dump_tokens {
        dump::dump_tokens(&lex_result.tokens, out_style);
        std::process::exit(if had_error { EXIT_COMPILE } else { EXIT_OK });
    }

    // parse stuff
    let parse_result = Parser::new(lex_result.tokens, &src).parse();

    for d in &parse_result.errors {
        eprintln!("{}", d.render_styled(&source, err_style));
        had_error = true;
    }

    if dump_ast {
        dump::dump_ast(&parse_result.document.body, out_style);
        std::process::exit(if had_error { EXIT_COMPILE } else { EXIT_OK });
    }
    
    if had_error {
        std::process::exit(EXIT_COMPILE);
    }

    // rint a summary.
    let node_count = parse_result.document.body.len();
    println!("ok: parsed {node_count} top-level node(s) from {path}");
}

fn parse_color(s: &str) -> Option<ColorChoice> {
    match s {
        "auto" => Some(ColorChoice::Auto),
        "always" => Some(ColorChoice::Always),
        "never" => Some(ColorChoice::Never),
        _ => None,
    }
}

/// Decide the actual `Style` to use, given the user's preference.
///
/// In `Auto`, two different conventions are usedL
/// 
/// - `CLICOLOR_FORCE` set to a non-empty value forces colour even 
/// when stderr is a pipe. 
/// - `NO_COLOR` - any non-empty value disables colour, regardless 
/// of TTY and allat.
///
/// When neither variable applies, only emit ANSI if stderr is a terminal 
/// since piping into something like tee or less would otherwise produce
/// a whole load of rubbish (ansi escape sequences)
///
/// `Always`/`Never` will skip the env-var checks so if an explicit flag
/// is give, that takes priority. :)
fn resolve_style(choice: ColorChoice, is_tty: bool) -> Style {
    match choice {
        ColorChoice::Always => Style::Ansi,
        ColorChoice::Never => Style::Plain,
        ColorChoice::Auto => {
            let force = std::env::var_os("CLICOLOR_FORCE")
                .map_or(false, |v| !v.is_empty());
            if force {
                return Style::Ansi;
            }
            let no_color = std::env::var_os("NO_COLOR")
                .map_or(false, |v| !v.is_empty());
            if no_color {
                Style::Plain
            } else if is_tty {
                Style::Ansi
            } else {
                Style::Plain
            }
        }
    }
}

