use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    println!("Generating Query DSL to: {}", out_dir);

    // Use Ruby to generate src/query.rs - should probably be written in Rust
    // itself to avoid an unnecessary dependency
    Command::new("tools/generate_query_dsl.rb").arg(out_dir).status().unwrap();
}
