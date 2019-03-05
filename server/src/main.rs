pub fn main() {
    let exit_code = mojom_lsp_server::start().unwrap();
    std::process::exit(exit_code);
}
