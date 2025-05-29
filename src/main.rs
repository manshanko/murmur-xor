use std::process::ExitCode;

mod hash;
mod lookup;

fn print_help() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("{}", env!("CARGO_PKG_AUTHORS"));
    println!();
    println!("Finds neighbors of MurmurHash64A keys.");
    println!("Only works with keys containing [a-z0-9_/].");
    println!();
    println!("Project home: {}", env!("CARGO_PKG_REPOSITORY"));
    println!();
    println!("USAGE:");
    println!("  murmur-xor [OPTIONS] <KEYS ...>");
    println!();
    println!("ARGS:");
    println!("  <KEYS>...   Text file with a key on each line.");
    println!();
    println!("OPTIONS:");
    println!("  --hashes <FILE>    Text file with a hex encoded hash on each line.");
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let _bin = args.next();

    let mut parsing_opts = true;
    let mut num_args = 0;
    let mut help = false;
    let mut key_files = Vec::new();
    let mut hash_files = Vec::new();
    while let Some(arg) = args.next() {
        num_args += 1;

        if !parsing_opts || arg.as_encoded_bytes()[0] != b'-' {
            key_files.push(arg);
            continue;
        }

        let Some(opt) = arg.to_str() else {
            eprintln!("ERROR: invalid UTF-8 in switch {arg:?}");
            return ExitCode::FAILURE;
        };

        match opt {
            "--" => parsing_opts = false,

            "--help"
            | "-h" => help = true,

            "--hashes" => {
                let Some(path) = args.next() else {
                    eprintln!("ERROR: missing parameter to {opt}");
                    return ExitCode::FAILURE;
                };

                hash_files.push(path);
            }

            _ => {
                if (opt.len() == 2 && opt.starts_with("-"))
                    || opt.starts_with("--")
                {
                    eprintln!("WARN: unknown switch {opt:?}");
                } else {
                    key_files.push(arg);
                }
            }
        }
    }

    if help || num_args == 0 {
        print_help();
        return ExitCode::SUCCESS;
    }

    ExitCode::SUCCESS
}
