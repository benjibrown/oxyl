// small funcs used by the parser. put these all here 
// so that mod.rs can stay focused on the recursive descent
// bits and bobs. legit just a file full of functions

use oxyl_diagnostics::DiagSpan;
use oxyl_lexer::{Span, TokenKind};

use crate::ast::{Arg, Node};

pub(super) fn diag_span(s: Span) -> DiagSpan {
    DiagSpan::new(s.start, s.end)
}

/// Stop predicate for `parse_nodes` when scanning the body of `\[ ... \]`.
pub(super) fn is_display_math_close(k: &TokenKind) -> bool {
    matches!(k, TokenKind::ControlSeq(s) if s == "]")
}

/// Stop predicate for `parse_nodes` when scanning the body of an environment.
pub(super) fn is_end_control_seq(k: &TokenKind) -> bool {
    matches!(k, TokenKind::ControlSeq(s) if s == "end")
}

/// Find the first `Arg::Mandatory` whose children are all `Node::Text`,
/// concatenate that text and return its index along with the trimmed name.
/// This is how the environment name is recovered (either from the begin 
/// statement or the end one).
pub(super) fn find_env_name(args: &[Arg]) -> Option<(usize, String)> {
    for (i, arg) in args.iter().enumerate() {
        if let Arg::Mandatory(children) = arg {
            let mut name = String::new();
            for child in children {
                if let Node::Text(t, _) = child {
                    name.push_str(t);
                } else {
                    return None;
                }
            }
            let trimmed = name.trim().to_owned();
            if !trimmed.is_empty() {
                return Some((i, trimmed));
            }
        }
    }
    None 
}
