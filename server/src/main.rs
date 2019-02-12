extern crate mojo_lsp_server;

pub fn main() {
    let exit_code = mojo_lsp_server::start().unwrap();
    std::process::exit(exit_code);
}
