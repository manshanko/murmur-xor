use std::collections::HashSet;
use std::process::ExitCode;

mod filter;
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
    println!("  --output <FILE>    Output file for found keys. Defaults to stdout.");
    println!("  --print-filtered   Print filtered keys to stdout.");
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let _bin = args.next();

    let mut parsing_opts = true;
    let mut num_args = 0;
    let mut help = false;
    let mut key_files = Vec::new();
    let mut hash_files = Vec::new();
    let mut output = None;
    let mut print_filtered = false;
    let mut debug = false;
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

            "--output"
            | "-o" => {
                let Some(out) = args.next() else {
                    eprintln!("ERROR: missing parameter to {opt}");
                    return ExitCode::FAILURE;
                };

                output = Some(out);
            }

            "--print-filtered" => print_filtered = true,

            "--debug" => debug = true,

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

    let start = std::time::Instant::now();

    let mut keys = Vec::new();
    for file in key_files {
        let text = std::fs::read_to_string(file).unwrap().leak();
        for line in text.lines() {
            keys.push(line);
        }
    }
    keys.sort();
    keys.dedup();

    let mut hashes = HashSet::new();
    for file in hash_files {
        let file = std::path::Path::new(&file);
        if file.extension() == Some(std::ffi::OsStr::new("bin")) {
            let data = std::fs::read(file).unwrap();
            for chunk in data.chunks_exact(16) {
                let (_ext, name) = chunk.split_at(8);
                let hash = u64::from_le_bytes(<[u8; 8]>::try_from(name).unwrap());
                hashes.insert(hash);
            }
        } else {
            let text = std::fs::read_to_string(file).unwrap();
            for line in text.lines() {
                let Ok(hash) = u64::from_str_radix(line, 16) else {
                    continue;
                };
                hashes.insert(hash);
            }
        }
    }

    let mut used = 0;
    for key in &keys {
        let hash = hash::mmh64a(key.as_bytes());
        if hashes.remove(&hash) {
            used += 1;
        }
    }

    let hashes = hashes.into_iter().collect::<Vec<_>>();

    if keys.is_empty() {
        eprintln!("ERROR: no keys found");
        return ExitCode::FAILURE;
    } else if hashes.is_empty() {
        eprintln!("ERROR: no hashes found");
        return ExitCode::FAILURE;
    }

    let mut filter = filter::FilterTrie::new();
    filter.add_keys(&keys);

    let load_time = start.elapsed().as_millis();

    let num_input = keys.len();
    let mut lookup = lookup::KeyLookup::new(&hashes);
    let mut total = 0;
    let mut filtered = HashSet::new();
    let mut total_found = HashSet::new();
    let mut target = keys;
    let mut buffer = String::new();
    loop {
        let mut found = HashSet::new();
        for key in &target {
            let prefix = &key[..key.len() - key.len() % 8];
            let prefix_end = if !prefix.is_empty() {
                [prefix.as_bytes()[prefix.len() - 1], prefix.as_bytes()[prefix.len() - 2]]
            } else {
                *b"na"
            };

            buffer.clear();
            buffer.push_str(prefix);
            let end = buffer.len();
            for (_hash, tail) in lookup.find_neighbors(prefix.as_bytes()) {
                total += 1;
                if filter.check_trie(prefix_end, tail.as_bytes()) {
                    let tail = tail.as_str();

                    buffer.truncate(end);
                    buffer.push_str(tail);
                    if !found.contains(buffer.as_str()) {
                        found.insert(&*buffer.to_string().leak());
                    }
                } else if filter::is_valid(tail.as_bytes()) {
                    let tail = tail.as_str();

                    buffer.truncate(end);
                    buffer.push_str(tail);
                    if !filtered.contains(&buffer) {
                        filtered.insert(buffer.to_string());
                    }
                }
            }
        }

        if found.is_empty() {
            break;
        }

        target = found.into_iter().collect::<Vec<_>>();
        target.sort();

        filter.add_keys(&target);

        for key in &target {
            lookup.remove(hash::mmh64a(key.as_bytes()));
            total_found.insert(*key);
        }
    }

    for key in &total_found {
        filtered.remove(*key);
    }

    if print_filtered {
        let mut filtered = filtered.into_iter().collect::<Vec<_>>();
        filtered.sort();
        for key in &filtered {
            println!("{key}");
        }
    }

    let mut found = total_found.into_iter().collect::<Vec<_>>();
    found.sort();
    let output = if let Some(output) = output {
        let found = found.join("\n");
        std::fs::write(&output, found).unwrap();
        std::path::Path::new(&output).display().to_string()
    } else {
        for key in &found {
            println!("{key}");
        }
        "@stdout".to_string()
    };

    if debug {
        eprintln!("DEBUG");
        eprintln!("  load_time: {}ms", load_time);
        eprintln!("  total_time: {}ms", start.elapsed().as_millis());
        eprintln!("  total_matches: {}", total);
        eprintln!("  keys_with_hash: {} (of {})", used, num_input);
        eprintln!();
    }
    eprintln!("INFO");
    eprintln!("  input_keys: {}", num_input);
    eprintln!("  found_keys: {}", found.len());
    eprintln!("  output: {}", output);

    ExitCode::SUCCESS
}
