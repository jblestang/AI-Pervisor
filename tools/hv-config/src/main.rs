//! Configuration compiler CLI entry point.

use std::process;

use hv_config::parse_config_command;

fn main() {
    let command = match parse_config_command(std::env::args()) {
        Ok(command) => command,
        Err(err) => {
            let _ = err.print();
            process::exit(2);
        }
    };
    process::exit(hv_config::dispatch_config(command));
}
