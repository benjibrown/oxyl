// colourisation for --help output !!
//
// following same principles as other colourisation
// i could be using a crate for this but i like making my life hard 

use oxyl_diagnostics::Style;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const YELLOW: &str = "\x1b[1;33m";
const MAGENTA: &str = "\x1b[1;35m";
const CYAN: &str = "\x1b[36m";
const CYAN_B: &str = "\x1b[1;36m";

// description column at byte 20 of each line - 2 char indent and 
// and 18 char name col. shared by flags and environment blocks
// so they all line up visually and allat :)
const NAME_COL: usize = 18;

/// Wrap `s` in `code`...RESET when `style` is ansi - otherwise
/// just pass through. Same principle used for the `--dump-ast`
/// and `--dump-tokens` output.
fn paint(style: Style, code: &str, s: &str) -> String {
    match style {
        Style::Plain => s.to_owned(),
        Style::Ansi => format!("{code}{s}{RESET}"),
    }
}

/// Pad `text` to `width` visible columns, then paint the (unpadded)
/// text in `code`. The padding spaces stay outside the escape 
/// seq so column alignment only computed on visible characters.
/// Hopefully avoids any terrible formatting - if it looks trash, submit a PR lol 
fn pad_painted(style: Style, code: &str, text: &str, width: usize) -> String {
    let pad = " ".repeat(width.saturating_sub(text.chars().count()));
    format!("{}{pad}", paint(style, code, text))
}

/// Render the help text. `style` is resolved exactly like the dump styles
/// - tty check per stream, env var checks, flag priority and allat mumbo
/// jumbo.
pub fn print_help(style: Style) {
    let version = env!("CARGO_PKG_VERSION");
    let oxyl = paint(style, MAGENTA, "oxyl");
    println!("{oxyl} {version} - a LaTeX compiler (work in progress)");
    println!();

    println!("{}", paint(style, YELLOW, "USAGE:"));
    println!("  {oxyl} [FLAGS] {}", paint(style, CYAN, "<file.tex>"));
    println!();

    println!("{}", paint(style, YELLOW, "FLAGS:"));

    // single token flag - go thru the pad paint so 
    // theyre all nice and aligned
    println!("  {}Print every token with its byte span, then exit",
        pad_painted(style, MAGENTA, "--dump-tokens", NAME_COL));
    println!("  {}Print the parsed AST nodes, then exit",
        pad_painted(style, MAGENTA, "--dump-ast", NAME_COL));

    // --color is the only flag rn with distinct options / args
    // so pad the flag + metavar as one lil unit then 
    // chuck in the description
    {
        let flag = paint(style, MAGENTA, "--color");
        let arg = paint(style, CYAN, "<when>");
        let pad = " ".repeat(NAME_COL.saturating_sub(14));
        let dflt = paint(style, DIM, "(default)");
        println!("  {flag} {arg}{pad}auto {dflt}, always, or never");
    } 
    // --version and --help both have aliases (-h and -V)
    // so emit comma and short flag to make sure 
    // they all get that precious color fr
    {
        let long = paint(style, MAGENTA, "--version");
        let short = paint(style, MAGENTA, "-V");
        let combined = format!("{long}, {short}");
        // visible width is 13
        let pad = " ".repeat(NAME_COL.saturating_sub(13));
        println!("  {combined}{pad}Print the {oxyl} version and exit");
    }
    {
        let long = paint(style, MAGENTA, "--help");
        let short = paint(style, MAGENTA, "-h");
        let combined = format!("{long}, {short}");
        // visible width is 13
        let pad = " ".repeat(NAME_COL.saturating_sub(10));
        println!("  {combined}{pad}Print this help message");
    }
    println!();

    println!("{}", paint(style, YELLOW, "EXIT CODES:"));
    println!("  {}  success", paint(style, BOLD, "0"));
    println!("  {}  the file failed to lex/parse, or could not be read", paint(style, BOLD, "1"));
    println!("  {}  bad invocation (unknown flag, missing or extra arguments)", paint(style, BOLD, "2"));
}
