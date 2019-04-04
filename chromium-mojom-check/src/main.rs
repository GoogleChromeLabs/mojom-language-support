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
    match mojom_syntax::parse(&input) {
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
