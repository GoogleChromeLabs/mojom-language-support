pub fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let exit_code = mojom_lsp_server::start(stdin, stdout)?;
    std::process::exit(exit_code);
}
