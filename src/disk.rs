use std::str;
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};

#[path ="./command.rs"]
mod command;

//use crate::command::MyNasCommand;
pub fn get_disk_info() -> Result<Value, Box<dyn std::error::Error>> {
    // 使用外部命令 `lsblk` 来获取硬盘信息
     let lsblk_output = command::MyNasCommand::run_command("lsblk", &["--json", "-O"]);
     let lsblk_output_str = String::from_utf8_lossy(&lsblk_output.stdout);
     let disk_info_json: Value = serde_json::from_str(&lsblk_output_str)?;
     Ok(disk_info_json)
}
