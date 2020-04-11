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

use std::env;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct ParseResult {
    path: PathBuf,
    parse_time: Duration,
}

fn parse_single_mojom<P: AsRef<Path>>(path: P) -> ParseResult {
    let mut reader = fs::File::open(path.as_ref())
        .map(|file| BufReader::new(file))
        .expect("Failed to open file");
    let mut input = String::new();
    reader.read_to_string(&mut input).unwrap();

    let instant = Instant::now();
    match mojom_lsp::syntax::parse(&input) {
        Ok(_mojom) => {
            let elapsed = instant.elapsed();
            println!("OK: {:?}", path.as_ref());
            println!("    Took: {:?}", elapsed);
            return ParseResult {
                path: path.as_ref().to_owned(),
                parse_time: elapsed,
            };
        }
        Err(err) => {
            println!("Err: {:?}", path.as_ref());
            println!("{:#?}", err);
            std::process::exit(1);
        }
    }
}

fn parse_chromium_mojom_files<P: AsRef<Path>>(path: P) {
    let mut results = Vec::new();
    let pattern = path.as_ref().join("**/*.mojom");
    let pattern = pattern.to_str().unwrap();
    let entries = glob::glob(pattern).unwrap().filter_map(|e| e.ok());
    for entry in entries {
        let result = parse_single_mojom(entry);
        results.push(result);
    }

    let longest = (&results)
        .iter()
        .max_by(|a, b| a.parse_time.cmp(&b.parse_time));
    println!("Longest: {:#?}", longest);
}

fn main() {
    let chromium_path = env::args().nth(1).expect("Must specify chromium src path");
    parse_chromium_mojom_files(&chromium_path);
}
