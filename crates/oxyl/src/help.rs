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
