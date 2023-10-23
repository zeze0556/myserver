use hyper::{Body, Request, Response, Server, Method, StatusCode};
use routerify::prelude::*;
use serde_json::{Value,json};
use tokio::net::TcpStream;
use std::fs;
use std::io;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE, UPGRADE, CONNECTION, HeaderValue};
use routerify::{Middleware, Router, RouterService, RequestInfo};
use hyper_staticfile::Static;
use hyper::service::Service;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use tungstenite::accept;
use tokio::sync::mpsc;
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::{
    tungstenite::{
        handshake::derive_accept_key,
        protocol::{Message, Role},
    },
    WebSocketStream,
};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt, SinkExt, TryFutureExt};
use clap::{Arg, App}; // 导入 clap
mod config;
mod disk;
mod sysstat;
use std::panic;
use std::process::{Command, Output};
#[path ="./command.rs"]
mod command;

mod remoteshell;

use crate::config::CONFIG;
// Define an app state to share it across the route handlers and middlewares.
struct State(u64);


async fn ws_handler(mut req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let ver = req.version();
    if hyper_tungstenite::is_upgrade_request(&req) {
        let Ok((response, websocket)) = hyper_tungstenite::upgrade(&mut req, None) else {todo!()};
        tokio::spawn(async move {
                        handle_ws_connection(websocket) .await;
        });
        return Ok(response)
    }
    // Return a 400 Bad Request response for non-WebSocket requests.
    let mut response = Response::new(Body::empty());
    *response.status_mut() = hyper::StatusCode::BAD_REQUEST;
    Ok(response)
}

async fn handle_ws_connection(websocket: HyperWebsocket)->Result<Response<Body>, Infallible>{
    let Ok(websocket) = websocket.await else { todo!() };
    //let websockt2 = websocket.close();
    let (mut tx, mut rx) = websocket.split();
    //let (mut tx2, mut rx2) = websocket2.split();
    // 创建一个通道，用于从 iostat_worker 发送消息到 handle_ws_connection
    let (iostat_sender, mut iostat_receiver) = mpsc::channel::<Message>(10);
    let sender = iostat_sender.clone();

    // 启动 iostat_worker 任务，并传递 iostat_sender 用于发送消息
    tokio::spawn(sysstat::MyNasStatus::iostat_worker(iostat_sender));

    // 创建一个任务，用于将来自 iostat_worker 的消息发送到 WebSocket 客户端
    tokio::spawn(async move {
        while let Some(message) = iostat_receiver.recv().await {
            match message {
                Message::Close(_) => {
                    println!("ok close");
                    break;
                }
                message => {
                    // 发送消息到 WebSocket 客户端
                    if let Err(e) = tx.send(message).await {
                        eprintln!("Failed to send message to WebSocket client: {}", e);
                        break;
                    }
                }
            }
        }
        println!("WebSocket connection closed.");
    });

    while let Some(message) = rx.next().await {
        match message {
            Ok(Message::Close(_)) => {
                println!("WebSocket connection closed.");
                break;
            }
            Ok(Message::Text(json_str)) => {
                // 使用 Serde 解析 JSON 数据
                if let Ok(parsed_json) = serde_json::from_str::<Value>(&json_str) {
                    // 检查 JSON 结构并执行相应的操作
                    let proc_type = parsed_json["type"].as_str().unwrap();//.get("type") else {todo!() };
                    println!("receive=={:?}, proc_type={:?}", json_str, proc_type);
                    match  Some(proc_type) {
                        Some("command")=> {
                            eprintln!("command to parse JSON data: {:?}", json_str);
                            // 执行命令操作，command.command 包含命令字符串
                            // 执行您的命令操作，并获取结果
                            let command = &parsed_json["command"];
                            let cmd = command["command"].as_str().unwrap();//.get("command");
                            // 提取请求中的 key 和 req 字段的值
                            let key_value = parsed_json["key"].as_str().unwrap_or("");
                            let Some(args) = command["args"].as_array() else { todo!() };//.unwrap();//.get("args");
                            let args_v: Vec<&str> =args.iter()
                                .filter_map(|v| v.as_str())
                                .collect();
                            let cmd = Command::new(cmd)
                                       .args(args_v)
                                       .output();
                            let response_json = match cmd {
                                Ok(result) => {
                                    // 设置 ret 字段的值，根据命令执行是否成功来决定
                                    let ret_value = result.status.code();//().unwrap_or(-1);
                                    if let Ok(result_string) = String::from_utf8(result.stdout) {
                                        // 创建包含 ret、data、key 和 req 字段的 JSON 对象
                                        json!({
                                            "ret": ret_value,
                                            "type": "command",
                                            "data": result_string,
                                            "key": key_value,
                                            "req": parsed_json,
                                        })
                                    } else {
                                        json!({
                                            "ret": ret_value,
                                            "type": "command",
                                            "data": "error string",
                                            "key": key_value,
                                            "req": parsed_json,
                                        })
                                    }
                                }
                                Err(err) => {
                                    println!("无法执行命令: {:?}", err);
                                    json!({
                                        "ret": -2,
                                        "type": "command",
                                        "error": format!("{:?}", err),
                                        "key": key_value,
                                        "req": parsed_json,
                                    })
                                }
                            };
                            // 使用 Serde 将 JSON 对象序列化为字符串
                            let response_str = serde_json::to_string(&response_json)
                                .unwrap_or_else(|e| {
                                    eprintln!("Failed to serialize JSON: {}", e);
                                    String::from("{\"ret\":-1,\"error\":\"Serialization error\"}")
                                });

                            let message = Message::Text(response_str);
                            if let Err(e) = sender.send(message).await {
                                eprintln!("Failed to send message to WebSocket client: {}", e);
                                break;
                            }

                        }
                        Some("ping") => {
                            eprintln!("Failed to parse JSON data: {}", json_str);
                        }
                        _ => {
                            eprintln!("Failed to parse JSON data: {}", json_str);
                        }
                    }
                } else {
                    eprintln!("Failed to parse JSON data: {}", json_str);
                }
            }
            _ => {
                // 处理其他类型的消息
                println!("m==={:?}", message);
            }
        }
    }

    println!("WebSocket connection closed.");
    let mut response = Response::new(Body::empty());
    *response.status_mut() = hyper::StatusCode::BAD_REQUEST;
    Ok(response)
}

async fn handle_disk_info(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        match disk::get_disk_info() {
            Ok(disk_info) => {
                let response_json = json!({
                    "ret": 0,
                    "disk": disk_info,
                });

                let response_body = serde_json::to_string(&response_json).unwrap();

                let response = Response::builder()
                    .header("Content-Type", "application/json")
                    .body(Body::from(response_body))
                    .unwrap();
                Ok(response)
            }
            Err(error) => {
                let response_json = json!({
                    "ret": -2,
                    "error": format!("{}", error),
                });

                let response_body = serde_json::to_string(&response_json)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to serialize JSON: {}", e);
                        String::from("{\"ret\":-2,\"error\":\"Serialization error\"}")
                    });

                let response = Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(Body::from(response_body))
                    .unwrap();
                Ok(response)
            }
        }
}

async fn handle_command_run(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // 从请求的 Body 中获取 JSON 数据
    let body_bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // 将 JSON 数据反序列化为 CommandRequest 结构
    match  serde_json::from_str::<Value>(&body_str) {
        Ok(parsed_json) => {
            let cmd = parsed_json["command"].as_str().unwrap();//.get("command");
            // 提取请求中的 key 和 req 字段的值
            let key_value = parsed_json["key"].as_str().unwrap_or("");
            let Some(args) = parsed_json["args"].as_array() else { todo!() };//.unwrap();//.get("args");
            let args_v: Vec<&str> =args.iter()
                .filter_map(|v| v.as_str())
                .collect();
            let cmd = Command::new(cmd)
                .args(args_v)
                .output();
            let response_json = match cmd {
                Ok(result) => {
                    //println!("exec result={:?}", result);
                    // 设置 ret 字段的值，根据命令执行是否成功来决定
                    let ret_value = result.status.code();//().unwrap_or(-1);
                    let std_out = String::from_utf8(result.stdout).unwrap();
                    let std_err = String::from_utf8(result.stderr).unwrap();//.unwrap_or("");
                        // 创建包含 ret、data、key 和 req 字段的 JSON 对象
                    json!({
                        "ret": ret_value,
                        "type": "command",
                        "data": {
                            "stdout": std_out,
                            "stderr": std_err
                        },
                        "key": key_value,
                        "req": parsed_json,
                    })
                }
                Err(err) => {
                    println!("无法执行命令: {:?}", err);
                    json!({
                        "ret": -2,
                        "type": "command",
                        "error": format!("{:?}", err),
                        "key": key_value,
                        "req": parsed_json,
                    })
                }
            };
            // 使用 Serde 将 JSON 对象序列化为字符串
            let response_body = serde_json::to_string(&response_json)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to serialize JSON: {}", e);
                    String::from("{\"ret\":-1,\"error\":\"Serialization error\"}")
                });
            let response = Response::builder()
                .header("Content-Type", "application/json")
                .body(Body::from(response_body))
                .unwrap();
            Ok(response)
        }
        Err(error)=> {
            let response_json = json!({
                "ret": -2,
                "error": format!("{}", error),
            });

            let response_body = serde_json::to_string(&response_json)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to serialize JSON: {}", e);
                    String::from("{\"ret\":-2,\"error\":\"Serialization error\"}")
                });

            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(response_body))
                .unwrap();
            Ok(response)
        }
    }
}

fn read_json_file(file_name: &str) -> Result<String, io::Error> {
    // 构建文件路径
    let file_path = format!("{}", file_name);
    
    // 读取文件内容
    fs::read_to_string(file_path)
}

fn write_json_file(file_name: &str, content: &str) -> Result<(), io::Error> {
    // 构建文件路径
    let file_path = format!("{}", file_name);
    
    // 将内容写入文件
    fs::write(file_path, content)
}

async fn handle_file_get(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (parts, body) = req.into_parts();
    // 使用 .map 处理请求体内容
    let body_bytes = hyper::body::to_bytes(body)
        .await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    match  serde_json::from_str::<Value>(&body_str) {
        Ok(parsed_json) => {
            let file_name = parsed_json["filename"].as_str().unwrap();
            let response_json = match read_json_file(&file_name) {
                Ok(file_content) => {
                    json!({
                        "ret": 0,
                        "data": file_content,
                        "req": body_str,
                    })
                }
                Err(err)=> {
                    println!("read_file error: {:?}", err);
                    json!({
                        "ret": -2,
                        "error": format!("{:?}", err),
                        "req": body_str,
                    })
                }
            };
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();
            Ok(response)
        }
        Err(err)=> {
            println!("read_file error: {:?}", err);
            let response_json = json!({
                "ret": -2,
                "error": format!("{:?}", err),
                "req": body_str,
            });
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();
            Ok(response)
        }
    }
}

async fn handle_file_put(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (parts, body) = req.into_parts();
    // 使用 .map 处理请求体内容
    let body_bytes = hyper::body::to_bytes(body)
        .await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    match  serde_json::from_str::<Value>(&body_str) {
        Ok(parsed_json) => {
            let file_name = parsed_json["filename"].as_str().unwrap();
            let file_content = parsed_json["data"].as_str().unwrap();
            let response_json = match write_json_file(&file_name, &file_content) {
                Ok(result) => {
                    json!({
                        "ret": 0,
                        "req": body_str,
                    })
                }
                Err(err)=> {
                    println!("write_file error: {:?}", err);
                    json!({
                        "ret": -2,
                        "error": format!("{:?}", err),
                        "req": body_str,
                    })
                }
            };
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();
            Ok(response)
        }
        Err(err)=> {
            println!("write_file error: {:?}", err);
            let response_json = json!({
                "ret": -2,
                "error": format!("{:?}", err),
                "req": body_str,
            });
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();
            Ok(response)
        }
    }
}

async fn handle_config_file(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // 获取请求路径中的文件名（*部分）
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let method = parts.method;
    let file_name = path.rsplit('/').next().unwrap_or_default();  // 获取最后一个斜杠后的部分作为文件名

    // 使用 .map 处理请求体内容
    let body_bytes = hyper::body::to_bytes(body)
        .await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // 根据请求方法执行相应的操作
    match method {
        Method::POST => {
            // 处理 POST 请求，返回文件内容
            // 请根据您的需求，将文件内容读取并构建 JSON 响应
            // 示例：读取文件内容
            let response_json = match read_json_file(&file_name) {
                Ok(file_content) => {
                    json!({
                        "ret": 0,
                        "data": file_content,
                        "req": body_str,
                    })
                }
                Err(err)=> {
                    println!("read_json_file error: {:?}", err);
                    json!({
                        "ret": -2,
                        "error": format!("{:?}", err),
                        "req": body_str,
                    })
                }
            };
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();
            Ok(response)
        }
        Method::PUT => {
            // 处理 PUT 请求，将请求体的内容写入文件
            // 将请求体内容写入文件
            let response_json = match write_json_file(&file_name, &body_str) {
                Err(err) => {
                    println!("write error {:?}", err);
                     json!({
                        "ret": -2,
                        "req": body_str,
                        "error": format!("{:?}", err),
                    })
                }
                Ok(result)=> {
                    json!({
                        "ret": 0,
                        "req": body_str,
                    })
                }
            };

            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();

            Ok(response)
        }
        _ => {
            // 返回不支持的请求方法的错误
            let response_json = json!({
                "ret": -1,
                "error": "Unsupported request method",
            });

            let response = Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();

            Ok(response)
        }
    }
}


// A middleware which logs an http request.
async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    println!("{} {} {}", req.remote_addr(), req.method(), req.uri().path());
    Ok(req)
}

// Define an error handler function which will accept the `routerify::Error`
// and the request information and generates an appropriate response.
async fn error_handler(err: routerify::RouteError, _: RequestInfo) -> Response<Body> {
    eprintln!("{}", err);
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(format!("Something went wrong: {}", err)))
        .unwrap()
}

async fn handle_static_file(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    //let resp = static_file.clone().serve(req)?;
    let static_file = Static::new("dist");
    let resp = match(static_file.serve(req).await) {
        Ok(resp)=> {
            resp
        }
        Err(err)=> {
            println!("handle_static_file error:{:?}", err);
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(format!("not found")))
                .unwrap()
        }
    };
    Ok(resp)
}

fn router() -> Router<Body, Infallible> {
    let ws_route = Router::builder()
        //.data(sender.clone())
        .any_method("/sys_stat", ws_handler)
        .any_method("/shell", remoteshell::ws_handler)
        .build()
        .unwrap();

    Router::builder()
    // Specify the state data which will be available to every route handlers,
    // error handler and middlewares.
        //.data(sender.clone())
        .data(State(100))
        .middleware(Middleware::pre(logger))
        .post("/api/disk/info",handle_disk_info)
        .post("/api/command/run",handle_command_run)
        .post("/api/file/get", handle_file_get)
        .put("/api/file/put", handle_file_put)
        .scope("/api/ws", ws_route)
        .get("*", handle_static_file)
        //.add(Route::from(Method::PATCH, "/asd").using(request_handler))
        //.build();
        .err_handler_with_info(error_handler)
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用 clap 解析命令行参数
    let matches = App::new("My NAS Server")
        .arg(Arg::with_name("config")
             .short("c")
             .long("config")
             .value_name("FILE")
             .help("Sets a custom config file")
             .takes_value(true))
        .get_matches();

    // 获取配置文件名参数，如果没有传递则使用默认值 "config.json"
    let config_file = matches.value_of("config").unwrap_or("config.json");
    // 加载配置文件
     config::Config::load(config_file)?;

    let config = CONFIG.lock().unwrap();

    // 使用 format! 宏构建 SocketAddr
    let addr = format!("{}:{}", config.address, config.port).parse()?;
    let router = router();
    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();
    let server = Server::bind(&addr)
        .serve(service);
    println!("HTTP server is running on http://{}:{}", config.address, config.port);
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}

