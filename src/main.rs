mod hash;
mod lookup;

fn main() {
    println!("{:016x}", hash::mmh64a("Hello, world!".as_bytes()));
}
