# mojo-lsp

A language server for Chromium mojo modules.

## Supported messages

Notifications:

- `exit`
- `initialized`

Requests:

- `initialize`
- `shutdown`

    // let parsed = mojom_parser::parse(input);
    // eprintln!("{:?}", parsed);
    // match parsed {
    //     Ok(_) => {
    //         if ctx.has_error {
    //             ctx.has_error = false;
    //             let params = lsp_types::PublishDiagnosticsParams {
    //                 uri: uri,
    //                 diagnostics: vec![],
    //             };
    //             publish_diagnotics(writer, params)?;
    //         }
    //         Ok(())
    //     }
    //     Err(mojom_parser::Error::SyntaxError(span)) => {
    //         ctx.has_error = true;
    //         let start = lsp_types::Position {
    //             line: span.line as u64 - 1,
    //             character: span.get_column() as u64 - 1,
    //         };
    //         let end = lsp_types::Position {
    //             line: span.line as u64 - 1,
    //             character: (span.get_column() + span.fragment.len()) as u64 - 1,
    //         };
    //         let range = lsp_types::Range {
    //             start: start,
    //             end: end,
    //         };
    //         let diagostic = lsp_types::Diagnostic {
    //             range: range,
    //             severity: None,
    //             code: None,
    //             source: None,
    //             message: "Syntax error".to_owned(),
    //             related_information: None,
    //         };
    //         let params = lsp_types::PublishDiagnosticsParams {
    //             uri: uri,
    //             diagnostics: vec![diagostic],
    //         };
    //         publish_diagnotics(writer, params)
    //     }
    // }
