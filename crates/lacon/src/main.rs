use std::process;

mod pipeline;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("run") => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("usage: lacon run <file.lc>");
                process::exit(1);
            });
            pipeline::run_file(path);
        }
        _ => {
            eprintln!("lacon v0.1.0");
            eprintln!("usage: lacon run <file.lc>");
            process::exit(1);
        }
    }
}
