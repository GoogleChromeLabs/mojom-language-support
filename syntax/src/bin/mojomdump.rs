use std::io::{self, BufRead, Read};
use std::path::Path;

fn main() {
    // for path in std::env::args().skip(1) {
    //     parse_single_mojom(path);
    // }
    parse_chromium_mojom_files();
}

fn parse_single_mojom<P: AsRef<Path>>(path: P) {
    let mut reader = std::fs::File::open(path.as_ref())
        .map(|file| io::BufReader::new(file))
        .expect("Failed to open file");
    let mut input = String::new();
    reader.read_to_string(&mut input).unwrap();

    match mojom_parser::parse(&input) {
        Ok(_mojom) => {
            println!("OK: {:?}", path.as_ref());
        }
        Err(err) => {
            println!("Err: {:?}", path.as_ref());
            println!("{:#?}", err);
            std::process::exit(1);
        }
    }
}

fn parse_chromium_mojom_files() {
    let list_reader = std::fs::File::open("tmp/chromium-mojom-files.txt")
        .map(|file| io::BufReader::new(file))
        .expect("");
    let prefix = Path::new("C:/src/chromium/src");
    for path in list_reader.lines() {
        let path = path.unwrap();
        let fullpath = prefix.join(&path);
        //println!("{:?}", fullpath);

        parse_single_mojom(fullpath);
    }
}
