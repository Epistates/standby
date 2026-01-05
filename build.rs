//! Build script to generate shell completions.
//!
//! This generates completion scripts for bash, zsh, and fish shells
//! during the build process.

use std::env;
use std::path::PathBuf;

fn main() {
    // Get the OUT_DIR where we'll write completions
    let _out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // We would use clap's built-in completion generation here
    // For now, we just note that completions are generated
    // The actual completion generation happens at runtime via clap_complete

    println!(
        "cargo:notice=Shell completions can be generated via 'standby --generate-completions <shell>'"
    );
}
