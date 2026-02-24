pub mod detect;

pub use detect::{
    build_fallback_command, build_split_command, detect_terminal, SplitCommand, TerminalKind,
};
