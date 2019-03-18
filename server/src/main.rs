pub fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let exit_code = mojom_lsp_server::start(stdin, stdout).unwrap();
    std::process::exit(exit_code);
}
