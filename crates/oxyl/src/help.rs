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

fn paint(style: Style, code: &str, s: &str) -> String {
    match style {
        Style::Plain => s.to_owned(),
        Style::Ansi => format!("{code}{s}{RESET}"),
    }
}

fn pad_painted(style: Style, code: &str, text: &str, width: usize) -> String {
    let pad = " ".repeat(width.saturating_sub(text.chars().count()));
    format!("{}{pad}", paint(style, code, text))
}

pub fn print_help(style: Style) {
    let version = env!("CARGO_PKG_VERSION");
    let oxyl = paint(style, MAGENTA, "oxyl");
    println!("{oxyl} {version} - a LaTeX compiler (work in progress)");
    println!();

    println!("{}", paint(style, YELLOW, "USAGE:"));
    println!("  {oxyl} [FLAGS] {}", paint(style, CYAN, "<file.tex>"));
    println!();

    println!("{}", paint(style, YELLOW, "FLAGS:"));
}
