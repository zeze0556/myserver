use lazy_static::lazy_static;
use std::convert::Infallible;
use routerify::ext::RequestExt;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use serde_json::{Value,json};
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE, UPGRADE, CONNECTION, HeaderValue};
use hyper::header;
use routerify::{Middleware, Router, RouterService, RequestInfo};
use std::collections::HashMap;
use passwd_rs::{Group, User, Shadow, AccountStatus};
use pwhash::unix;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Future;
use cookie::{Cookie, CookieJar};
use rand::Rng;
use core::fmt;
use std::sync::Mutex;
use crate::api_error::ApiError;
// User data structure to store API Tokens
#[derive(Clone)]
#[derive(Debug)]
pub struct HttpUser {
    api_token: String,
    username: String,
}

impl HttpUser {
    pub fn new(api_token: &str, username:&str) -> Self {
        HttpUser {
            api_token: api_token.to_string(),
            username: username.to_string()
        }
    }
}

impl fmt::Display for HttpUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "api_token={} username={}", self.api_token, self.username)
    }
}

lazy_static! {
    static ref white_list: HashMap<String, bool> = {
        let mut map = HashMap::new();
        map.insert("/api/login".to_string(), true);
        map
    };
    static ref global_user_store: Mutex<HashMap<String, HttpUser>> = {
        let mut map = HashMap::new();
        Mutex::new(map)
    };
}

pub async fn auth_check(req: Request<Body>) -> Result<Request<Body>, ApiError> {
    match white_list.get(req.uri().path()) {
        Some(true) => {
            Ok(req)
        }
        _ => {
            if let Some(header) = req.headers().get("Cookie") {
                if let Ok(header_value) = header.to_str() {
                    let cookies: Vec<Cookie<'_>> = header_value
                        .split("; ")
                        .filter_map(|s| Cookie::parse(s).ok())
                        .collect();
                    for cookie in cookies {
                        if cookie.name() == "api_token" {
                            // Found the "api_token" cookie
                            let api_token = cookie.value();
                            let mut user_store = global_user_store.lock().unwrap();
                            if user_store.contains_key(api_token) {
                                return Ok(req); // API Token is valid, allow the request to proceed
                            } else {
                                return Err(ApiError::Unauthorized);
                            }
                        }
                    }
                }
                Err(ApiError::Unauthorized)
            } else {
                // Handle the case where the "Cookie" header is not present in the request
                Err(ApiError::Unauthorized)
            }
        }
    }
}

pub async fn handle_login(mut req: Request<Body>) -> Result<Response<Body>, ApiError> {
    //let (parts, body) = req.into_parts();
    let body = req.body_mut();
    let body_bytes = hyper::body::to_bytes(body)
        .await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    match  serde_json::from_str::<Value>(&body_str) {
        Ok(parsed_json) => {
            let username = parsed_json["username"].as_str().unwrap();
            let input_password = parsed_json["password"].as_str().unwrap();
            match User::new_from_name(username) {
                Ok(user) => {
                    let password;
                    if user.passwd.as_ref().unwrap().eq("x") {
                        // WARN! This works only if program is executed as root
                        let shadow = match Shadow::new_from_username(&user.name.clone()) {
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::PermissionDenied {
                                    println!("Must be run as root to access shadow passwords");
                                    let response_json = json!({
                                        "ret": -2,
                                        "errror": "permission error"
                                    });
                                    let response = Response::builder()
                                        .status(StatusCode::FORBIDDEN)
                                        .header("Content-Type", "application/json")
                                        .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                                        .unwrap();
                                    return Ok(response)
                                    //Err(ApiError::Unauthorized)
                                }
                                else {
                                    let response_json = json!({
                                        "ret": -2,
                                        "errror": format!("{:?}", e)
                                    });
                                    let response = Response::builder()
                                        .status(StatusCode::FORBIDDEN)
                                        .header("Content-Type", "application/json")
                                        .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                                        .unwrap();
                                    return Ok(response)
                                    //Err(ApiError::Unauthorized)
                                };
                            },
                            Ok(o) => o,
                        };
                        if let AccountStatus::Active(passwd) = shadow.passwd {
                            password = passwd;
                        } else {
                            password = shadow.passwd.to_string();
                        }
                    } else {
		                    password = user.passwd.unwrap()
	                  }
                    match unix::verify(input_password, &password) {
                        true => {
                            let response_json = json!({
                                "ret": 0,
                            });
                            let length = 20; // 生成的随机字符串长度
                            let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                            let charset_len = charset.len();

                            let mut rng = rand::thread_rng();
                            let api_token: String = (0..length)
                                .map(|_| {
                                    let idx = rng.gen_range(0..charset_len);
                                    charset.chars().nth(idx).unwrap()
                                })
                                .collect();
                            let mut cookie_jar = CookieJar::new();
                            cookie_jar.add_original(Cookie::build(("api_token", api_token.clone())).path("/"));
                            let mut user_store = global_user_store.lock().unwrap();
                            user_store.insert(api_token.to_string(), HttpUser::new(&api_token,username));
                            println!("user_store=={:?}", user_store);
                            // Set API Token in a Cookie
                            let mut response = Response::builder()
                                .status(StatusCode::OK)
                                .header("Content-Type", "application/json")

                                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                            .unwrap();
                            for cookie in cookie_jar.iter() {
                                let cookie_header = cookie.to_string();
                                response.headers_mut().insert("Set-Cookie", cookie_header.parse().unwrap());
                            }
                            Ok(response)
                        }
                        false => {
                            let response_json = json!({
                                "ret": -2,
                                //"errror": "password error"
                            });
                            let response = Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .header("Content-Type", "application/json")
                                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                                .unwrap();
                            Err(ApiError::Unauthorized)
                        }
                    }
                }
                Err(_) => {
                    let response_json = json!({
                        "ret": -2,
                        //"error": "no user"
                    });
                    let response = Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .header("Content-Type", "application/json")
                        .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                        .unwrap();
                    Err(ApiError::Unauthorized)
                }
            }
        }
        Err(err)=> {
            println!("handle_login error: {:?}", err);
            let response_json = json!({
                "ret": -2,
                "error": format!("{:?}", err),
                "req": body_str,
            });
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_userinfo(mut req: Request<Body>) -> Result<Response<Body>, ApiError> {
    if let Some(header) = req.headers().get("Cookie") {
        if let Ok(header_value) = header.to_str() {
            let cookies: Vec<Cookie<'_>> = header_value
                .split("; ")
                .filter_map(|s| Cookie::parse(s).ok())
                .collect();
            for cookie in cookies {
                if cookie.name() == "api_token" {
                    // Found the "api_token" cookie
                    let api_token = cookie.value();
                    let mut user_store = global_user_store.lock().unwrap();
                    match user_store.get(api_token) {
                        None => {
                            return Err(ApiError::Unauthorized);
                        }
                        Some(user) => {
                            let response_json = json!({
                                "ret": 0,
                                "data": {
                                    "username": user.username
                                }
                            });
                            let response = Response::builder()
                                .status(StatusCode::OK)
                                .header("Content-Type", "application/json")
                                .body(Body::from(serde_json::to_string(&response_json).unwrap()))
                                .unwrap();
                            return Ok(response)
                        }
                    }
                }
            }
        }
    }
    return Err(ApiError::Unauthorized);
}
