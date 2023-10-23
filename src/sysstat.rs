use std::str;
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;
use std::process::{Command, Output, Stdio};
use std::io::{BufRead, BufReader};

//#[path ="./command.rs"]
//mod command;


pub struct MyNasStatus {
}

impl MyNasStatus {
    pub async fn iostat_worker(sender: mpsc::Sender<Message>) {
        let command = Command::new("iostat")
            //.args(&["-o", "JSON", "1"])
            .args(&["-k", "1"])
            .stdout(Stdio::piped()) // 捕获标准输出
            .spawn();
            //.output()
            //.expect("Failed to execute command")
        // 检查命令是否成功启动
        let mut child = match command {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to start the command: {}", e);
                return;
            }
        };
        // 从子进程中获取标准输出的句柄
        let stdout = child.stdout.take().unwrap();

        // 创建一个缓冲读取器
        let mut reader = BufReader::new(stdout);
        loop {
            let mut buf = vec![];
            let num_bytes = reader.read_until(b'-', &mut buf)
                .expect("reading from cursor won't fail");
            if let Ok(iostat_json) = String::from_utf8(buf) {
                //println!("send={}",iostat_json);
                let message = Message::Text(iostat_json);
                if let Err(e) = sender.send(message).await {
                    eprintln!("Failed to send message to WebSocket client: {}", e);
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                // 逐行读取标准输出
                /*
                for line in reader.lines() {
                match line {
                Ok(text) => {
                println!("{}", text);
                let message = Message::Text(text);
                if let Err(e) = sender.send(message).await {
                eprintln!("Failed to send message to WebSocket client: {}", e);
                break;
            }
            }
                Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
            }
            }*/
            } else {
                break;
            }
        }
        /*
        loop {
            // 运行 iostat 命令并获取输出
            let iostat_output = command::MyNasCommand::run_command("iostat", &["-o", "JSON"]);
            //.arg("-c") // 连续输出模式
            //.arg("1")  // 每秒更新一次
                //.arg("-o") // 以 JSON 格式输出
                //.arg("JSON")
                //.output()
                //.expect("Failed to run iostat");
            // 将 iostat 输出的 JSON 字符串转换为 Message::Text 并发送到 WebSocket 客户端
            if let Ok(iostat_json) = String::from_utf8(iostat_output.stdout) {
                let message = Message::Text(iostat_json);
                if let Err(e) = sender.send(message).await {
                    eprintln!("Failed to send message to WebSocket client: {}", e);
                    break;
                }
            }
            // 每秒更新一次 iostat
        }
        */
        println!("!end====");
        let result = child.kill();
        if let Err(err) = result {
            eprintln!("Failed to kill child process: {}", err);
        }
        // 等待子进程结束
        let status = child.wait().expect("Failed to wait for the child process");
        if status.success() {
            println!("Command executed successfully");
        } else {
            eprintln!("Command failed with exit code: {:?}", status.code());
        }
    }
}
