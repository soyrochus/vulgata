use crate::types::{ActionType, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardRuntimeAction {
    ConsolePrint,
    ConsolePrintln,
    ConsoleEprint,
    ConsoleEprintln,
    ConsoleReadLine,
    FileReadText,
    FileWriteText,
    FileAppendText,
    FileExists,
}

impl StandardRuntimeAction {
    pub fn qualified_name(self) -> &'static str {
        match self {
            Self::ConsolePrint => "console.print",
            Self::ConsolePrintln => "console.println",
            Self::ConsoleEprint => "console.eprint",
            Self::ConsoleEprintln => "console.eprintln",
            Self::ConsoleReadLine => "console.read_line",
            Self::FileReadText => "file.read_text",
            Self::FileWriteText => "file.write_text",
            Self::FileAppendText => "file.append_text",
            Self::FileExists => "file.exists",
        }
    }

    pub fn signature(self) -> ActionType {
        match self {
            Self::ConsolePrint
            | Self::ConsolePrintln
            | Self::ConsoleEprint
            | Self::ConsoleEprintln => ActionType {
                params: vec![Type::Text],
                result: Box::new(Type::Result(Box::new(Type::None), Box::new(Type::Text))),
            },
            Self::ConsoleReadLine => ActionType {
                params: Vec::new(),
                result: Box::new(Type::Result(Box::new(Type::Text), Box::new(Type::Text))),
            },
            Self::FileReadText => ActionType {
                params: vec![Type::Text],
                result: Box::new(Type::Result(Box::new(Type::Text), Box::new(Type::Text))),
            },
            Self::FileWriteText | Self::FileAppendText => ActionType {
                params: vec![Type::Text, Type::Text],
                result: Box::new(Type::Result(Box::new(Type::None), Box::new(Type::Text))),
            },
            Self::FileExists => ActionType {
                params: vec![Type::Text],
                result: Box::new(Type::Bool),
            },
        }
    }
}

pub fn is_standard_runtime_module(name: &str) -> bool {
    matches!(name, "console" | "file")
}

pub fn lookup_standard_runtime_member(
    module_name: &str,
    member_name: &str,
) -> Option<StandardRuntimeAction> {
    match (module_name, member_name) {
        ("console", "print") => Some(StandardRuntimeAction::ConsolePrint),
        ("console", "println") => Some(StandardRuntimeAction::ConsolePrintln),
        ("console", "eprint") => Some(StandardRuntimeAction::ConsoleEprint),
        ("console", "eprintln") => Some(StandardRuntimeAction::ConsoleEprintln),
        ("console", "read_line") => Some(StandardRuntimeAction::ConsoleReadLine),
        ("file", "read_text") => Some(StandardRuntimeAction::FileReadText),
        ("file", "write_text") => Some(StandardRuntimeAction::FileWriteText),
        ("file", "append_text") => Some(StandardRuntimeAction::FileAppendText),
        ("file", "exists") => Some(StandardRuntimeAction::FileExists),
        _ => None,
    }
}

pub fn lookup_standard_runtime_name(name: &str) -> Option<StandardRuntimeAction> {
    match name {
        "console.print" => Some(StandardRuntimeAction::ConsolePrint),
        "console.println" => Some(StandardRuntimeAction::ConsolePrintln),
        "console.eprint" => Some(StandardRuntimeAction::ConsoleEprint),
        "console.eprintln" => Some(StandardRuntimeAction::ConsoleEprintln),
        "console.read_line" => Some(StandardRuntimeAction::ConsoleReadLine),
        "file.read_text" => Some(StandardRuntimeAction::FileReadText),
        "file.write_text" => Some(StandardRuntimeAction::FileWriteText),
        "file.append_text" => Some(StandardRuntimeAction::FileAppendText),
        "file.exists" => Some(StandardRuntimeAction::FileExists),
        _ => None,
    }
}
