// src/config.rs

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use lazy_static::lazy_static;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub address: String,
    pub port: u16,
}

lazy_static! {
    // 使用 Mutex 包装全局的 CONFIG 变量
    pub static ref CONFIG: Mutex<Config> = Mutex::new(Config {
        address: String::new(),
        port: 0,
    });
}

impl Config {
    pub fn load(config_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 读取配置文件
        let mut config_file = File::open(config_file)?;
        let mut config_json = String::new();
        config_file.read_to_string(&mut config_json)?;

        let config: Self = serde_json::from_str(&config_json)?;
        // 使用 Mutex 的 lock 方法获取可变引用并更新 CONFIG 变量
        let mut config_lock = CONFIG.lock().unwrap();
        *config_lock = config.clone();
        Ok(config)
    }
}


