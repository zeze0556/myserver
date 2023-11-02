use nix::pty::{posix_openpt, grantpt, unlockpt, ptsname};
use nix::unistd::{fork, ForkResult, setsid, close, dup2};
use std::ffi::CStr;
use std::io::Write;
use std::io::Read;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::io::AsRawFd;
use nix::fcntl::{OFlag};
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::Error;
use hyper_tungstenite::{HyperWebsocket};
use hyper::{Body, Request, Response};
use std::convert::Infallible;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use std::collections::HashMap;
use serde::Deserialize;
use crate::api_error::ApiError; // 使用相对路径

#[derive(Deserialize)]
#[derive(Debug)]
struct CommandArgs {
    command: String,
    args: Option<Vec<String>>,
}

pub async fn ws_handler(mut req: Request<Body>) -> Result<Response<Body>, ApiError> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        let Ok((response, websocket)) = hyper_tungstenite::upgrade(&mut req, None) else {todo!()};
        tokio::spawn(async move {
             let _ = handle_xterm_session(req,websocket) .await;
        });
        return Ok(response)
    }
    // Return a 400 Bad Request response for non-WebSocket requests.
    let mut response = Response::new(Body::empty());
    *response.status_mut() = hyper::StatusCode::BAD_REQUEST;
    Ok(response)
}

async fn handle_xterm_session(mut req:Request<Body>,websocket: HyperWebsocket) -> Result<(), Error> {
    // 获取查询参数 "args" 的值
    let query_params = req.uri().query().unwrap_or("");
    //let args_param = query_params.get("args").unwrap_or("");
    // 将 URL 编码的查询参数解码为 JSON 字符串
    //let decoded_args = urlencoding::decode(args_param).unwrap_or("".to_string());
    // 解析 JSON 数据
    //let command_args: CommandArgs = serde_json::from_str(&decoded_args).unwrap();
    let params: HashMap<String, String> = req
        .uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new);
    let default = String::from("{\"command\":\"/bin/sh\"}");
    let args = params.get("args").unwrap_or(&default);
    let command_args: CommandArgs = serde_json::from_str(args).unwrap();
    let Ok(websocket) = websocket.await else { todo!() };
    let mut master_fd = posix_openpt(nix::fcntl::OFlag::O_RDWR).expect("error posix_openpt");

    // Grant access to the slave PTY
    grantpt(&master_fd).expect("error grantpt");
    unlockpt(&master_fd).expect("error unlockpt");

    // Get the name of the slave PTY
    let slave_name = unsafe {
        let cstr = ptsname(&master_fd).expect("error ptsname");
        CStr::from_ptr(cstr.as_ptr() as *const i8).to_str().expect("error cstr::from_ptr")
    };
    // Get the name of the slave PTY
    // Fork a child process
     match unsafe{fork()} {
         Ok(ForkResult::Parent { child, .. }) => {
             // This is the parent process
             // Perform any parent-specific tasks here
             //println!("Parent process ID: {}", nix::unistd::getpid());
             //println!("Child process ID: {}", child);
             // Close the master PTY file descriptor in the parent process
             let (mut ws_writer, mut ws_reader) = websocket.split();
             let mut input = unsafe { File::from_raw_fd(master_fd.as_raw_fd()) };
             tokio::spawn(async move {
                 let mut buf = vec![0; 1024];
                 loop {
                     match input.read(&mut buf) {
                         Err(err) => {
                             println!("line: {} Error reading from stream: {}, read={:?}", line!(), err,input);
                             if let Err(_e) = ws_writer.send(Message::Close(None)).await {
                             }
                             break;
                         }
                         Ok(0) => {
                             println!("eof===");
                             break; // EOF from PTY
                         }
                         Ok(n) => {
                             if let Err(e) = ws_writer.send(Message::Text(String::from_utf8_lossy(&buf[..n]).to_string())).await {
                                 println!("send remote shell connection closed. error:{:?}", e);
                                 break;
                             }
                         }
                     }
                 }
                 println!("end pty to wsocket");
             });
             //let mut output = unsafe { File::from_raw_fd(master_fd.as_raw_fd()) };
             while let Some(message) = ws_reader.next().await {
                 match message {
                     Ok(Message::Close(_)) => {
                         println!("WebSocket connection closed.");
                         break;
                     }
                     Ok(Message::Text(message)) => {
                         let _ = master_fd.write_all(message.as_bytes());
                     }
                     Err(e) => {
                         println!("shell write error:{:?}", e);
                         break;
                     }
                     _ => todo!()
                 }
             }
             // 关闭WebSocket后发送SIGTERM信号给子进程
             if let Err(e) = kill(child, Signal::SIGTERM) {
                 eprintln!("Error sending SIGTERM to child process: {}", e);
             }

             // 等待子进程退出
             match waitpid(child, None).expect("waitpid failed") {
                 WaitStatus::Exited(_, _) => {
                     println!("Child process has exited");
                 }
                 _ => {
                     println!("Child process did not exit as expected");
                 }
             }
             println!("end websocket");
             close(master_fd.as_raw_fd()).expect("close master_fd error");
         }
        Ok(ForkResult::Child) => {
            // This is the child process
            // Perform any child-specific tasks here
            println!("Child process ID: {}", nix::unistd::getpid());

            // Create a new session and make the child process the session leader
            setsid().expect("setsid failed");

            // Open the slave PTY and associate it with standard input, output, and error
            let slave_fd = nix::fcntl::open(slave_name, OFlag::O_RDWR, nix::sys::stat::Mode::empty())
                .expect("open slave PTY failed");
            // Duplicate the slave PTY file descriptor to standard input, output, and error
            dup2(slave_fd, 0).expect("dup2 for stdin failed");
            dup2(slave_fd, 1).expect("dup2 for stdout failed");
            dup2(slave_fd, 2).expect("dup2 for stderr failed");

            // Close the original slave PTY file descriptor
            close(slave_fd).expect("close slave_fd in child process failed");

            // Now the child process is set up with the PTY
            // You can execute shell or other programs here
            //let shell_cmd = std::process::Command::new("/bin/sh").spawn();
            //let shell_cmd = std::process::Command::new(cmd).spawn();
            println!("run command=={:?}", command_args);
            //let mut shell_cmd = std::process::Command::new(command_args.command);
            let shell_cmd = match command_args.args {
                Some(ref cmd_args) => {
                    std::process::Command::new(command_args.command)
                        .args(cmd_args)
                        .spawn()
                }
                None => {
                    std::process::Command::new(command_args.command)
                        .spawn()
                }
            };
            shell_cmd?.wait().expect("error wait");
            // Exit the child process
            println!("exit");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Fork failed: {}", e);
        }
    }

    Ok(())
}
