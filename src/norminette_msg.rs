use std::u32;

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Debug)]
pub enum NorminetteMsg {
    NoLocation {
        message: String,
    },
    Line {
        line: i32,
        message: String,
    },
    LineColumn {
        error_type: String,
        line: i32,
        column: i32,
        message: String,
    },
}

impl NorminetteMsg {
    pub fn find_range(&self) -> Range {
        match self {
            NorminetteMsg::NoLocation { message: _ } => Range {
                start: Position::new(0, 0),
                end: Position::new(1, u32::MAX),
            },
            NorminetteMsg::Line { line, message: _ } => Range {
                start: Position::new((line - 1) as u32, 0),
                end: Position::new(*line as u32, u32::MAX),
            },
            NorminetteMsg::LineColumn {
                error_type: _,
                line,
                column,
                message: _,
            } => Range {
                start: Position::new((line - 1) as u32, *column as u32),
                end: Position::new(*line as u32, u32::MAX),
            },
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            range: self.find_range(),
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("norminette".to_string()),
            message: self.message().to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn message(&self) -> &str {
        match self {
            NorminetteMsg::NoLocation { message } => message,
            NorminetteMsg::Line { line: _, message } => message,
            NorminetteMsg::LineColumn {
                error_type: _,
                line: _,
                column: _,
                message,
            } => message,
        }
    }
}
