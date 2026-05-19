// oxyl-parser
//
// Builds a Document of Nodes from the lexers token stream
//
// AST types are in ast.rs btw - the parser proper 
// ( paser, parseresult, the recusrive descent over the token stream etc)
// all lives in parser.rs this file is just stitching all that together 
// because i hate massive, long files of code so much 


mod ast;
mod parser;

pub use ast::{Arg, Document, Node};
pub use parser::{ParseResult, Parser};
