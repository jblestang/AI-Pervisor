//! Workspace task runner entry point.

use std::process;

use hv_config::constants::CLI_EXIT_USAGE;
use xtask::parse_task_command;

fn main() {
    let command = match parse_task_command(std::env::args()) {
        Ok(command) => command,
        Err(err) => {
            let _ = err.print();
            process::exit(CLI_EXIT_USAGE);
        }
    };
    process::exit(xtask::dispatch_task(command));
}
