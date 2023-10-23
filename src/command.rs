// src/command.rs
use std::process::{Command, Output};
use std::str;

pub struct MyNasCommand {
}

impl MyNasCommand {
    pub fn run_command(command: &str, args: &[&str]) -> Output{
    Command::new(command)
        .args(args)
        .output()
        .expect("Failed to execute command")
}
}
