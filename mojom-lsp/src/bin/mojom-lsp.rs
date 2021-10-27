// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {}

pub fn main() -> anyhow::Result<()> {
    // For help/version information. There is no option now.
    let _ = Opt::from_args();

    env_logger::init();

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let exit_code = mojom_lsp::server::start(stdin, stdout)?;
    std::process::exit(exit_code);
}
