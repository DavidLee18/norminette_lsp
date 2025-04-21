use std::u32;

use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

#[derive(Debug)]
pub enum NorminetteMsg {
    Error {
        error_type: String,
        line: i32,
        column: i32,
        message: String,
    },
    Notice {
        error_type: String,
        line: i32,
        column: i32,
        message: String,
    },
    Ok,
}

impl NorminetteMsg {
    pub fn find_range(&self) -> Option<Range> {
        match self {
            NorminetteMsg::Error { line, column, .. } => Some(Range {
                start: Position::new((line - 1) as u32, (column - 1) as u32),
                end: Position::new((line - 1) as u32, (column + 2) as u32),
            }),
            NorminetteMsg::Notice { line, column, .. } => Some(Range {
                start: Position::new((line - 1) as u32, (column - 1) as u32),
                end: Position::new((line - 1) as u32, (column + 2) as u32),
            }),
            NorminetteMsg::Ok => None,
        }
    }

    pub fn to_diagnostic(self) -> Option<Diagnostic> {
        let range = self.find_range()?;
        match self {
            NorminetteMsg::Error {
                error_type,
                message,
                ..
            } => Some(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String(error_type)),
                code_description: None,
                source: Some("norminette".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            }),
            NorminetteMsg::Notice {
                error_type,
                message,
                ..
            } => Some(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::INFORMATION),
                code: Some(NumberOrString::String(error_type)),
                code_description: None,
                source: Some("norminette".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            }),
            NorminetteMsg::Ok => None,
        }
    }
}
