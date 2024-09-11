use thumbscache::open_thumbscache;
use std::io;
use std::io::*;
use std::path::Path;

fn main() {
    print!("Thumbscache path : ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    input = input.trim().to_string();
    let mut thumbscache_file = open_thumbscache(input);
    println!("{:?}",thumbscache_file.read());
    println!("{:?}",thumbscache_file);
}
