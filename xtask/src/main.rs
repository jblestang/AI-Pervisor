//! Workspace task runner entry point.

use std::process;

use xtask::parse_task_command;

fn main() {
    let command = match parse_task_command(std::env::args()) {
        Ok(command) => command,
        Err(err) => {
            let _ = err.print();
            process::exit(2);
        }
    };
    process::exit(xtask::dispatch_task(command));
}
