pub mod norminette_msg;
pub mod parser;

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::{
    Diagnostic, DiagnosticOptions, DiagnosticServerCapabilities, InitializeParams,
    LogMessageParams, MessageType, PublishDiagnosticsParams, ServerCapabilities,
    WorkDoneProgressOptions,
};
use parser::parse_norminette;
use std::error::Error;
use std::io;
use std::path::Path;

fn read_norminette(path: &Path) -> io::Result<Vec<Diagnostic>> {
    let output = std::process::Command::new("norminette")
        .arg(path)
        .output()?;
    let (_, diags) = parse_norminette(
        &String::from_utf8(output.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    )
    .map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("norminette parse error: {:?}", err),
        )
    })?;

    Ok(diags
        .into_iter()
        .map(|d| d.to_diagnostic())
        .filter(|o| o.is_some())
        .map(|o| o.unwrap())
        .collect())
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("starting norminette LSP server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: None,
            inter_file_dependencies: false,
            workspace_diagnostics: false,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        ..Default::default()
    })?;
    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    eprintln!("initialized connection!");
    main_loop(connection, initialization_params)?;
    io_threads.join()?;
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params)?;

    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                eprintln!("got request: {req:?}");
                match cast::<DocumentDiagnosticRequest>(req) {
                    Ok((id, params)) => {
                        eprintln!("got doc diagnostic request #{id} params: {params:?}");
                        match read_norminette(&Path::new(params.text_document.uri.path().as_str()))
                        {
                            Ok(diags) => {
                                connection.sender.send(Message::Response(Response {
                                    id,
                                    result: Some(serde_json::to_value(
                                        &PublishDiagnosticsParams {
                                            uri: params.text_document.uri,
                                            diagnostics: diags,
                                            version: None,
                                        },
                                    )?),
                                    error: None,
                                }))?;
                            }
                            Err(e) => {
                                connection.sender.send(Message::Response(Response {
                                    id,
                                    result: Some(serde_json::to_value(&LogMessageParams {
                                        typ: MessageType::ERROR,
                                        message: format!(
                                            "norminette read of {} failed: {}",
                                            params.text_document.uri.path(),
                                            e
                                        ),
                                    })?),
                                    error: None,
                                }))?;
                            }
                        }
                    }
                    Err(e) => return Err(Box::new(e)),
                };
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                eprintln!("got notification: {not:?}");
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
