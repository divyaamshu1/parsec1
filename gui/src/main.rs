//! Parsec GUI main entry point - lightweight browser-hosted IDE server

#![allow(dead_code, unused_imports)]

use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use std::net::TcpListener as StdTcpListener;
use tungstenite::accept;
use tungstenite::protocol::Message as WsMessage;
use bytes::Bytes;

use anyhow::Result;
use tiny_http::{Server, Response, Request, Method, Header};
use tracing_subscriber::{fmt, EnvFilter};
use parsec_core::editor::Editor;
use tokio::runtime::Runtime;
use tokio::sync::Mutex as TokioMutex;
use std::io::Write;
use std::process::{Stdio, Child};
use std::fs::read_dir;
use serde_json::json;

fn main() -> Result<()> {
    // Initialize logging
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .init();

    tracing::info!("Starting Parsec lightweight GUI server");

    // locate dist directory
    let mut dist = PathBuf::from("gui");
    dist.push("dist");
    if !dist.exists() {
        // create a minimal frontend if missing
        fs::create_dir_all(&dist)?;
        let index = include_str!("../../gui_dist_index.html");
        fs::write(dist.join("index.html"), index)?;
        let mainjs = include_str!("../../gui_dist_main.js");
        fs::write(dist.join("main.js"), mainjs)?;
    }

    // find a free port
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let local_addr = listener.local_addr()?;
    // tiny_http's from_listener takes a listener and an optional SSL config
    // Map the tiny_http boxed error into anyhow::Error to satisfy `?` conversions
    let server = Server::from_listener(listener, None).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let server = Arc::new(server);
    let serve_dir = dist.clone();

    // Create a tokio runtime and shared Editor instance
    let rt = Arc::new(Runtime::new()?);
    let editor = Arc::new(TokioMutex::new(Editor::new()));

    // create terminal session store
    let sessions: Arc<StdMutex<HashMap<String, TermSession>>> = Arc::new(StdMutex::new(HashMap::new()));

    // start a websocket server for terminals on a dynamic port and publish its address
    let ws_listener = StdTcpListener::bind("127.0.0.1:0")?;
    let ws_addr = ws_listener.local_addr()?;
    let ws_url = format!("ws://{}:{}", ws_addr.ip(), ws_addr.port());
    let ws_url_arc = Arc::new(ws_url.clone());
    let ws_listener_thread = ws_listener.try_clone()?;
    let sessions_ws = sessions.clone();
    thread::spawn(move || {
        for stream_res in ws_listener_thread.incoming() {
            if let Ok(stream) = stream_res {
                // set non-blocking so websocket.read() will return Io(ErrorKind::WouldBlock) when no data
                let _ = stream.set_nonblocking(true);
                if let Ok(mut websocket) = accept(stream) {
                    // each ws connection handles a single terminal session control
                    let sessions_inner = sessions_ws.clone();
                    thread::spawn(move || {
                        use std::time::Duration;
                        use tungstenite::Error as WsErr;

                        let mut current_session: Option<String> = None;
                        loop {
                            // send any pending output for the subscribed session
                            if let Some(ref id) = current_session {
                                if let Some(sess) = sessions_inner.lock().unwrap().get(id) {
                                    let mut o = sess.output.lock().unwrap();
                                    if !o.is_empty() {
                                        let msg_text: String = o.clone();
                                        let _ = websocket.send(WsMessage::Text(msg_text.into()));
                                        o.clear();
                                    }
                                }
                            }

                            // try reading an incoming message (non-blocking)
                            match websocket.read() {
                                Ok(msg) => {
                                    if msg.is_text() {
                                        let txt = msg.into_text().unwrap_or_default();
                                        // simple protocol: "SESSION:<id>" to subscribe
                                        if txt.starts_with("SESSION:") {
                                            let id = txt[8..].to_string();
                                            current_session = Some(id);
                                        } else if txt.starts_with("IN:") {
                                            // input for session, format: IN:<id>:<payload>
                                            if let Some(p1) = txt.find(':') {
                                                if let Some(p2) = txt[p1+1..].find(':') {
                                                    let id = &txt[p1+1..p1+1+p2];
                                                    let payload = &txt[p1+1+p2+1..];
                                                    if let Some(sess) = sessions_inner.lock().unwrap().get(id) {
                                                        let mut stdin = sess.stdin.lock().unwrap();
                                                        let _ = stdin.write_all(payload.as_bytes());
                                                        let _ = stdin.flush();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    // If it's a WouldBlock (no data), continue the loop; otherwise break/close
                                    match e {
                                        WsErr::Io(ioe) if ioe.kind() == std::io::ErrorKind::WouldBlock => { /* nothing to read now */ }
                                        _ => { break; }
                                    }
                                }
                            }

                            std::thread::sleep(Duration::from_millis(80));
                        }
                    });
                }
            }
        }
    });

    // spawn server thread
    let srv = server.clone();
    let serve_dir_thread = serve_dir.clone();
    let editor_thread = editor.clone();
    let handle = rt.handle().clone();
    let sessions_thread = sessions.clone();
    let ws_for_thread = ws_url_arc.clone();
    thread::spawn(move || {
        for request in srv.incoming_requests() {
            handle_request(request, &serve_dir_thread, editor_thread.clone(), handle.clone(), sessions_thread.clone(), ws_for_thread.clone());
        }
    });

    let url = format!("http://{}:{}/", local_addr.ip(), local_addr.port());
    tracing::info!("Server running at {}", url);
    tracing::info!("WS terminal available at {}", ws_url);

    // open in default browser
    let _ = webbrowser::open(&url);

    // keep main thread alive
    loop { thread::park(); }
}

struct TermSession {
    stdin: Arc<StdMutex<std::process::ChildStdin>>,
    output: Arc<StdMutex<String>>,
    child: Arc<StdMutex<Child>>,
}

fn handle_request(mut request: Request, serve_dir: &PathBuf, editor: Arc<TokioMutex<Editor>>, handle: tokio::runtime::Handle, sessions: Arc<StdMutex<HashMap<String, TermSession>>>, ws_url: Arc<String>) {
    let method = request.method().clone();
    let url = request.url().to_string();

    // Editor endpoints
    if url.starts_with("/api/editor") {
        // GET /api/editor/content
        if url == "/api/editor/content" && method == Method::Get {
            let content = handle.block_on(async {
                let ed = editor.lock().await;
                ed.get_content()
            });
            let _ = request.respond(Response::from_string(content));
            return;
        }

        // POST /api/editor/set  body = full content
        if url == "/api/editor/set" && method == Method::Post {
            let mut body = String::new();
            if let Ok(_) = request.as_reader().read_to_string(&mut body) {
                let res = handle.block_on(async {
                    let mut ed = editor.lock().await;
                    // replace current buffer content
                    ed.create_new_buffer();
                    ed.insert(&body);
                    Ok::<(), anyhow::Error>(())
                });
                match res {
                    Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            } else {
                let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
            }
            return;
        }
        // GET /api/editor/open?path=...
        if url.starts_with("/api/editor/open") && method == Method::Get {
            if let Some(idx) = url.find("path=") {
                let raw = &url[idx + 5..];
                let path = percent_decode(raw);
                let res = handle.block_on(async {
                    let mut ed = editor.lock().await;
                    ed.open_file(path).await.map(|_| ())
                });
                match res {
                    Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            } else {
                let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
            }
            return;
        }

        // POST /api/editor/save_current
        if url == "/api/editor/save_current" && method == Method::Post {
            let res = handle.block_on(async {
                let ed = editor.lock().await;
                ed.save_current().await
            });
            match res {
                Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
            }
            return;
        }

        // POST /api/editor/undo and /api/editor/redo
        if url == "/api/editor/undo" && method == Method::Post {
            let _ = handle.block_on(async {
                let mut ed = editor.lock().await;
                ed.undo();
                Ok::<(), anyhow::Error>(())
            });
            let _ = request.respond(Response::from_string("OK"));
            return;
        }

        if url == "/api/editor/redo" && method == Method::Post {
            let _ = handle.block_on(async {
                let mut ed = editor.lock().await;
                ed.redo();
                Ok::<(), anyhow::Error>(())
            });
            let _ = request.respond(Response::from_string("OK"));
            return;
        }

        // GET /api/editor/list - return current editor statistics
        if url == "/api/editor/list" && method == Method::Get {
            let stats = handle.block_on(async {
                let ed = editor.lock().await;
                ed.statistics()
            });
            let txt = format!("lines: {}\nchars: {}\nwords: {}\nbytes: {}\ncursor: {}:{}\nmode: {:?}\nmodified: {}\npath: {:?}",
                stats.lines, stats.characters, stats.words, stats.bytes, stats.cursor_line, stats.cursor_column, stats.mode, stats.modified, stats.path);
            let _ = request.respond(Response::from_string(txt));
            return;
        }
    }

    if method == Method::Get {
        let path = if url == "/" { "index.html" } else { &url[1..] };
        let fs_path = serve_dir.join(path);
        if fs_path.exists() && fs_path.is_file() {
            if let Ok(mut f) = fs::File::open(&fs_path) {
                let mut buf = Vec::new();
                let _ = f.read_to_end(&mut buf);
                let ct = match fs_path.extension().and_then(|s| s.to_str()).unwrap_or("") {
                    "html" => "text/html",
                    "js" => "application/javascript",
                    "css" => "text/css",
                    "png" => "image/png",
                    "ico" => "image/x-icon",
                    _ => "application/octet-stream",
                };
                let response = Response::from_data(buf).with_header(Header::from_bytes(b"Content-Type", ct.as_bytes()).unwrap());
                let _ = request.respond(response);
                return;
            }
        }
        let _ = request.respond(Response::from_string("Not found").with_status_code(404));
        return;
    }

    // File read endpoint: GET /api/file?path=relative/path.txt
    if url.starts_with("/api/file") && method == Method::Get {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let fs_path = PathBuf::from(&path);
            if fs_path.exists() {
                match fs::read_to_string(&fs_path) {
                    Ok(s) => { let _ = request.respond(Response::from_string(s)); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            } else {
                let _ = request.respond(Response::from_string("File not found").with_status_code(404));
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // File write endpoint: POST /api/file?path=relative/path.txt  body = file contents
    if url.starts_with("/api/file") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let fs_path = PathBuf::from(&path);
            let mut body = String::new();
            if let Ok(_) = request.as_reader().read_to_string(&mut body) {
                if let Some(parent) = fs_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                match fs::File::create(&fs_path).and_then(|mut f| f.write_all(body.as_bytes())) {
                    Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            } else {
                let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Run python file: POST /api/python/run?path=relative/path.py
    if url.starts_with("/api/python/run") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let session_id = uuid::Uuid::new_v4().to_string();
            let python_cmd = if cfg!(windows) { "python" } else { "python3" };
            let mut cmd = Command::new(python_cmd);
            cmd.arg("-u").arg(&path);
            match cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                Ok(mut child) => {
                    let out_buf = Arc::new(StdMutex::new(String::new()));
                    let out_clone = out_buf.clone();
                    if let Some(mut s) = child.stdout.take() {
                        thread::spawn(move || {
                            let mut buf = [0u8; 1024];
                            loop {
                                match s.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let t = String::from_utf8_lossy(&buf[..n]).to_string();
                                        let mut o = out_clone.lock().unwrap();
                                        o.push_str(&t);
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                    if let Some(mut s2) = child.stderr.take() {
                        let out_clone2 = out_buf.clone();
                        thread::spawn(move || {
                            let mut buf = [0u8; 1024];
                            loop {
                                match s2.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let t = String::from_utf8_lossy(&buf[..n]).to_string();
                                        let mut o = out_clone2.lock().unwrap();
                                        o.push_str(&t);
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                    let stdin_handle = Arc::new(StdMutex::new(child.stdin.take().unwrap()));
                    let sess = TermSession { stdin: stdin_handle.clone(), output: out_buf.clone(), child: Arc::new(StdMutex::new(child)) };
                    sessions.lock().unwrap().insert(session_id.clone(), sess);
                    let _ = request.respond(Response::from_string(session_id));
                }
                Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Terminal endpoints (polling-based)
    // POST /api/terminal/start -> returns session id
    if url == "/api/terminal/start" && method == Method::Post {
        // create a new shell process
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.arg("/K");
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-i");
            c
        };
        match cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(mut child) => {
                let out_buf = Arc::new(StdMutex::new(String::new()));
                let out_clone = out_buf.clone();
                // read stdout
                if let Some(mut s) = child.stdout.take() {
                    thread::spawn(move || {
                        let mut buf = [0u8; 1024];
                        loop {
                            match s.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    let t = String::from_utf8_lossy(&buf[..n]).to_string();
                                    let mut o = out_clone.lock().unwrap();
                                    o.push_str(&t);
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                // read stderr
                if let Some(mut s2) = child.stderr.take() {
                    let out_clone2 = out_buf.clone();
                    thread::spawn(move || {
                        let mut buf = [0u8; 1024];
                        loop {
                            match s2.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    let t = String::from_utf8_lossy(&buf[..n]).to_string();
                                    let mut o = out_clone2.lock().unwrap();
                                    o.push_str(&t);
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }

                let stdin_handle = Arc::new(StdMutex::new(child.stdin.take().unwrap()));
                let sess = TermSession { stdin: stdin_handle.clone(), output: out_buf.clone(), child: Arc::new(StdMutex::new(child)) };
                sessions.lock().unwrap().insert(session_id.clone(), sess);
                let _ = request.respond(Response::from_string(session_id));
            }
            Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
        }
        return;
    }

    // GET /api/terminal/read?session=ID -> returns accumulated output and clears buffer
    if url.starts_with("/api/terminal/read") && method == Method::Get {
        if let Some(idx) = url.find("session=") {
            let id = &url[idx + 8..];
            let id = percent_decode(id);
            if let Some(sess) = sessions.lock().unwrap().get(&id) {
                let mut out = sess.output.lock().unwrap();
                let data = out.clone();
                out.clear();
                let _ = request.respond(Response::from_string(data));
            } else {
                let _ = request.respond(Response::from_string("Session not found").with_status_code(404));
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // POST /api/terminal/write?session=ID  body = input to write
    if url.starts_with("/api/terminal/write") && method == Method::Post {
        if let Some(idx) = url.find("session=") {
            let id = &url[idx + 8..];
            let id = percent_decode(id);
            let mut body = String::new();
            if let Ok(_) = request.as_reader().read_to_string(&mut body) {
                if let Some(sess) = sessions.lock().unwrap().get(&id) {
                    let mut stdin = sess.stdin.lock().unwrap();
                    let _ = stdin.write_all(body.as_bytes());
                    let _ = stdin.flush();
                    let _ = request.respond(Response::from_string("OK"));
                } else {
                    let _ = request.respond(Response::from_string("Session not found").with_status_code(404));
                }
            } else {
                let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // POST /api/terminal/stop?session=ID -> stops and removes session
    if url.starts_with("/api/terminal/stop") && method == Method::Post {
        if let Some(idx) = url.find("session=") {
            let id = &url[idx + 8..];
            let id = percent_decode(id);
            if let Some(sess) = sessions.lock().unwrap().remove(&id) {
                let mut child = sess.child.lock().unwrap();
                let _ = child.kill();
                let _ = request.respond(Response::from_string("OK"));
            } else {
                let _ = request.respond(Response::from_string("Session not found").with_status_code(404));
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // POST /api/python/run?path=...  or body = python code
    if url.starts_with("/api/python/run") && method == Method::Post {
        // determine path param
        let mut path_opt: Option<String> = None;
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            path_opt = Some(percent_decode(raw));
        }

        // read body (may be empty)
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);

        let session_id = uuid::Uuid::new_v4().to_string();
        let mut cmd = if let Some(ref p) = path_opt {
            let mut c = Command::new("python");
            c.arg(p);
            c
        } else if !body.is_empty() {
            let mut c = Command::new("python");
            c.arg("-c");
            c.arg(body.clone());
            c
        } else {
            let _ = request.respond(Response::from_string("Bad request: no path or code provided").with_status_code(400));
            return;
        };

        match cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(mut child) => {
                let out_buf = Arc::new(StdMutex::new(String::new()));
                let out_clone = out_buf.clone();
                if let Some(mut s) = child.stdout.take() {
                    thread::spawn(move || {
                        let mut buf = [0u8; 1024];
                        loop {
                            match s.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    let t = String::from_utf8_lossy(&buf[..n]).to_string();
                                    let mut o = out_clone.lock().unwrap();
                                    o.push_str(&t);
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                if let Some(mut s2) = child.stderr.take() {
                    let out_clone2 = out_buf.clone();
                    thread::spawn(move || {
                        let mut buf = [0u8; 1024];
                        loop {
                            match s2.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    let t = String::from_utf8_lossy(&buf[..n]).to_string();
                                    let mut o = out_clone2.lock().unwrap();
                                    o.push_str(&t);
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }

                let stdin_handle = Arc::new(StdMutex::new(child.stdin.take().unwrap()));
                let sess = TermSession { stdin: stdin_handle.clone(), output: out_buf.clone(), child: Arc::new(StdMutex::new(child)) };
                sessions.lock().unwrap().insert(session_id.clone(), sess);
                let _ = request.respond(Response::from_string(session_id));
            }
            Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
        }
        return;
    }

    // Git status: GET /api/git/status
    if url == "/api/git/status" && method == Method::Get {
        let output = Command::new("git").args(["status", "--porcelain"]).output();
        match output {
            Ok(o) => { let s = String::from_utf8_lossy(&o.stdout).to_string(); let _ = request.respond(Response::from_string(s)); }
            Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
        }
        return;
    }

    // Git add: POST /api/git/add?path=...
    if url.starts_with("/api/git/add") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let output = Command::new("git").args(["add", &path]).output();
            match output { Ok(o) => { let s = String::from_utf8_lossy(&o.stdout).to_string(); let _ = request.respond(Response::from_string(s)); } Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); } }
        } else { let _ = request.respond(Response::from_string("Bad request").with_status_code(400)); }
        return;
    }

    // Git commit: POST /api/git/commit  body = commit message
    if url == "/api/git/commit" && method == Method::Post {
        let mut msg = String::new();
        if let Ok(_) = request.as_reader().read_to_string(&mut msg) {
            let output = Command::new("git").args(["commit","-m", &msg]).output();
            match output { Ok(o) => { let s = String::from_utf8_lossy(&o.stdout).to_string(); let _ = request.respond(Response::from_string(s)); } Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); } }
        } else { let _ = request.respond(Response::from_string("Bad request").with_status_code(400)); }
        return;
    }

    // Git diff: GET /api/git/diff?path=...
    if url.starts_with("/api/git/diff") && method == Method::Get {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let output = Command::new("git").args(["diff","--", &path]).output();
            match output { Ok(o) => { let s = String::from_utf8_lossy(&o.stdout).to_string(); let _ = request.respond(Response::from_string(s)); } Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); } }
        } else {
            let output = Command::new("git").args(["diff"]).output();
            match output { Ok(o) => { let s = String::from_utf8_lossy(&o.stdout).to_string(); let _ = request.respond(Response::from_string(s)); } Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); } }
        }
        return;
    }

    // WS address endpoint for frontend to connect xterm
    if url == "/api/terminal/ws_addr" && method == Method::Get {
        let s = ws_url.as_ref().clone();
        let _ = request.respond(Response::from_string(s));
        return;
    }

    // AI chat stub: POST /api/ai/chat  body = prompt
    if url == "/api/ai/chat" && method == Method::Post {
        let mut body = String::new();
        if let Ok(_) = request.as_reader().read_to_string(&mut body) {
            // TODO: forward to real AI providers in parsec-ai crate
            let reply = format!("AI stub received: {}\n\n(Configure providers to get real responses)", body);
            let _ = request.respond(Response::from_string(reply));
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Directory listing: GET /api/tree?path=...
    if url.starts_with("/api/tree") && method == Method::Get {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let target = PathBuf::from(&path);
            if target.exists() {
                let mut entries = Vec::new();
                if let Ok(rd) = read_dir(&target) {
                    for e in rd.flatten() {
                        if let Ok(mt) = e.metadata() {
                            entries.push(json!({
                                "name": e.file_name().to_string_lossy(),
                                "is_dir": mt.is_dir(),
                                "path": e.path().to_string_lossy(),
                            }));
                        }
                    }
                }
                let s = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
                let _ = request.respond(Response::from_string(s).with_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap()));
            } else {
                let _ = request.respond(Response::from_string("Path not found").with_status_code(404));
            }
        } else {
            // default to workspace root
            let mut entries = Vec::new();
            if let Ok(rd) = read_dir(".") {
                for e in rd.flatten() {
                    if let Ok(mt) = e.metadata() {
                        entries.push(json!({
                            "name": e.file_name().to_string_lossy(),
                            "is_dir": mt.is_dir(),
                            "path": e.path().to_string_lossy(),
                        }));
                    }
                }
            }
            let s = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
            let _ = request.respond(Response::from_string(s).with_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap()));
        }
        return;
    }

    // Create file/folder: POST /api/tree/create?path=...&folder=1
    if url.starts_with("/api/tree/create") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let fs_path = PathBuf::from(&path);
            if url.contains("folder=1") {
                match fs::create_dir_all(&fs_path) {
                    Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            } else {
                if let Some(parent) = fs_path.parent() { let _ = fs::create_dir_all(parent); }
                match fs::File::create(&fs_path) {
                    Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Delete file/folder: POST /api/tree/delete?path=...
    if url.starts_with("/api/tree/delete") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let fs_path = PathBuf::from(&path);
            let res = if fs_path.is_dir() { fs::remove_dir_all(&fs_path) } else { fs::remove_file(&fs_path) };
            match res {
                Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Rename: POST /api/tree/rename?path=...&to=...
    if url.starts_with("/api/tree/rename") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            if let Some(idx2) = url.find("&to=") {
                let raw = &url[idx + 5..idx2];
                let to_raw = &url[idx2 + 4..];
                let path = percent_decode(raw);
                let to = percent_decode(to_raw);
                match fs::rename(PathBuf::from(&path), PathBuf::from(&to)) {
                    Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                    Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
                }
            } else {
                let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Create file or folder: POST /api/tree/create?path=...&folder=1
    if url.starts_with("/api/tree/create") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let fs_path = PathBuf::from(&path);
            let is_folder = url.contains("folder=1");
            let res = if is_folder { fs::create_dir_all(&fs_path).map_err(|e| e.to_string()) } else { if let Some(parent) = fs_path.parent() { let _ = fs::create_dir_all(parent); } fs::File::create(&fs_path).map(|_| ()).map_err(|e| e.to_string()) };
            match res {
                Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Delete file or folder: POST /api/tree/delete?path=...
    if url.starts_with("/api/tree/delete") && method == Method::Post {
        if let Some(idx) = url.find("path=") {
            let raw = &url[idx + 5..];
            let path = percent_decode(raw);
            let fs_path = PathBuf::from(&path);
            let res = if fs_path.is_dir() { fs::remove_dir_all(&fs_path).map_err(|e| e.to_string()) } else { fs::remove_file(&fs_path).map_err(|e| e.to_string()) };
            match res {
                Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }

    // Rename: POST /api/tree/rename?path=...&to=...
    if url.starts_with("/api/tree/rename") && method == Method::Post {
        if let (Some(a), Some(b)) = (url.find("path="), url.find("&to=")) {
            let raw = &url[a + 5..b];
            let to_raw = &url[b + 4..];
            let from = percent_decode(raw);
            let to = percent_decode(to_raw);
            match fs::rename(&from, &to) {
                Ok(_) => { let _ = request.respond(Response::from_string("OK")); }
                Err(e) => { let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500)); }
            }
        } else {
            let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        }
        return;
    }


    if method == Method::Post {
        // simple command endpoint: POST /api/exec with body = command string
        if url.starts_with("/api/exec") {
            let mut body = String::new();
            if let Ok(_) = request.as_reader().read_to_string(&mut body) {
                // run command
                let output = if cfg!(windows) {
                    Command::new("cmd").args(["/C", body.trim()]).output()
                } else {
                    Command::new("sh").args(["-c", body.trim()]).output()
                };

                match output {
                    Ok(o) => {
                        let s = String::from_utf8_lossy(&o.stdout).to_string();
                        let _ = request.respond(Response::from_string(s));
                    }
                    Err(e) => {
                        let _ = request.respond(Response::from_string(format!("Error: {}", e)).with_status_code(500));
                    }
                }
                return;
            }
        }
        let _ = request.respond(Response::from_string("Bad request").with_status_code(400));
        return;
    }

    let _ = request.respond(Response::from_string("Method not allowed").with_status_code(405));
}

fn percent_decode(s: &str) -> String {
    // minimal percent-decoding for simple paths
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next().unwrap_or('0');
            let lo = chars.next().unwrap_or('0');
            if let (Some(h), Some(l)) = (hi.to_digit(16), lo.to_digit(16)) {
                let byte = (h * 16 + l) as u8;
                out.push(byte as char);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}
