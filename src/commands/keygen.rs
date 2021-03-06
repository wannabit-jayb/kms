use abscissa::Callable;
use signatory::{Ed25519Seed, Encode};
use std::{env, process};

use ed25519::SECRET_KEY_ENCODING;

/// Options for the `keygen` command
#[derive(Debug, Default, Options)]
pub struct KeygenCommand {
    #[options(free)]
    output_paths: Vec<String>,
}

impl Callable for KeygenCommand {
    /// Generate an Ed25519 secret key for use with a software provider (i.e. ed25519-dalek)
    fn call(&self) {
        if self.output_paths.len() != 1 {
            eprintln!("Usage: {} keygen [PATH]", env::args().next().unwrap());
            process::exit(2);
        }

        let output_path = &self.output_paths[0];

        let seed = Ed25519Seed::generate();
        seed.encode_to_file(output_path, SECRET_KEY_ENCODING)
            .unwrap_or_else(|e| {
                status_err!("couldn't write to {}: {}", output_path, e);
                process::exit(1);
            });

        info!("Wrote random Ed25519 private key to {}", output_path);
    }
}
