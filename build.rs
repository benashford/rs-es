use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("Generating Query DSL to: {}", out_dir);

    // Use Ruby to generate src/query.rs - should probably be written in Rust
    // itself to avoid an unnecessary dependency
    match Command::new("tools/generate_query_dsl.rb").arg(out_dir).status() {
        Ok(exit_status) => {
            println!("Finished with status: {}", exit_status);
            match exit_status.code() {
                Some(code) if code == 0 => (),
                Some(code)              => { panic!("Status code: {}", code); },
                None                    => { panic!("No status code"); }
            };
        },
        Err(err)        => { panic!("Error: {}", err); }
    }
}
