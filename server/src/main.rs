use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Parser)]
#[command(about = "IME switcher for remote Neovim over SSH")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args, Clone)]
struct ConnectArgs {
    #[arg(long)]
    port: u16,
    /// PIN for authentication. Can also be set via IM_SELECT_PIN env var.
    #[arg(long, env = "IM_SELECT_PIN")]
    pin: String,
}

#[derive(Subcommand, Clone)]
#[command(rename_all = "snake_case")]
enum Command {
    SaveAndSwitch(ConnectArgs),
    Restore(ConnectArgs),
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    command: String,
    pin: String,
}

#[derive(Debug, Deserialize)]
struct Response {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<(), String> {
    let len = u32::try_from(payload.len())
        .map_err(|_| format!("payload too large: {} bytes", payload.len()))?;
    stream
        .write_all(&len.to_be_bytes())
        .map_err(|e| format!("write length: {e}"))?;
    stream
        .write_all(payload)
        .map_err(|e| format!("write payload: {e}"))?;
    Ok(())
}

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| format!("read response length: {e}"))?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;

    if resp_len > MAX_RESPONSE_BYTES {
        return Err(format!("response too large: {resp_len} bytes"));
    }

    let mut buf = vec![0u8; resp_len];
    stream
        .read_exact(&mut buf)
        .map_err(|e| format!("read response: {e}"))?;
    Ok(buf)
}

fn send_request(port: u16, request: &Request) -> Result<Response, String> {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let mut stream =
        TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|e| format!("connect: {e}"))?;

    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|e| format!("set read timeout: {e}"))?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|e| format!("set write timeout: {e}"))?;

    let payload = rmp_serde::to_vec_named(request).map_err(|e| format!("serialize: {e}"))?;
    write_frame(&mut stream, &payload)?;

    let resp_buf = read_frame(&mut stream)?;
    rmp_serde::from_slice(&resp_buf).map_err(|e| format!("deserialize response: {e}"))
}

fn main() {
    let cli = Cli::parse();

    let (command_str, args) = match &cli.command {
        Command::SaveAndSwitch(a) => ("save_and_switch", a),
        Command::Restore(a) => ("restore", a),
    };

    let request = Request {
        command: command_str.to_string(),
        pin: args.pin.clone(),
    };

    match send_request(args.port, &request) {
        Ok(resp) if resp.success => {}
        Ok(resp) => {
            let msg = resp.error.unwrap_or_else(|| "unknown error".into());
            eprintln!("im-select-server: {msg}");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("im-select-server: {e}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn request_serialization_roundtrip() {
        let req = Request {
            command: "save_and_switch".into(),
            pin: "123456".into(),
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let decoded: std::collections::HashMap<String, String> =
            rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded["command"], "save_and_switch");
        assert_eq!(decoded["pin"], "123456");
    }

    #[test]
    fn response_deserialization_success() {
        #[derive(Serialize)]
        struct TestResp {
            success: bool,
        }
        let bytes = rmp_serde::to_vec_named(&TestResp { success: true }).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        assert!(decoded.success);
        assert!(decoded.error.is_none());
    }

    #[test]
    fn response_with_error() {
        #[derive(Serialize)]
        struct TestResp {
            success: bool,
            error: Option<String>,
        }
        let bytes = rmp_serde::to_vec_named(&TestResp {
            success: false,
            error: Some("bad pin".into()),
        })
        .unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        assert!(!decoded.success);
        assert_eq!(decoded.error.as_deref(), Some("bad pin"));
    }

    #[test]
    fn framing_roundtrip_over_tcp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();

            // Read the request frame
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).unwrap();
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut req_buf = vec![0u8; req_len];
            conn.read_exact(&mut req_buf).unwrap();

            let req: Request = rmp_serde::from_slice(&req_buf).unwrap();
            assert_eq!(req.command, "save_and_switch");
            assert_eq!(req.pin, "999999");

            // Write a success response frame
            #[derive(Serialize)]
            struct Resp {
                success: bool,
            }
            let resp_payload = rmp_serde::to_vec_named(&Resp { success: true }).unwrap();
            let len = (resp_payload.len() as u32).to_be_bytes();
            conn.write_all(&len).unwrap();
            conn.write_all(&resp_payload).unwrap();
        });

        let request = Request {
            command: "save_and_switch".into(),
            pin: "999999".into(),
        };
        let resp = send_request(port, &request).unwrap();
        assert!(resp.success);
        assert!(resp.error.is_none());

        server.join().unwrap();
    }

    #[test]
    fn framing_rejects_oversize_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();

            // Read and discard the request frame
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).unwrap();
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut req_buf = vec![0u8; req_len];
            conn.read_exact(&mut req_buf).unwrap();

            // Send a response claiming to be larger than MAX_RESPONSE_BYTES
            let fake_len = (MAX_RESPONSE_BYTES as u32 + 1).to_be_bytes();
            conn.write_all(&fake_len).unwrap();
        });

        let request = Request {
            command: "restore".into(),
            pin: "000000".into(),
        };
        let err = send_request(port, &request).unwrap_err();
        assert!(err.contains("response too large"), "got: {err}");

        server.join().unwrap();
    }

    #[test]
    fn request_with_empty_fields() {
        let req = Request {
            command: "".into(),
            pin: "".into(),
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let decoded: Request = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.command, "");
        assert_eq!(decoded.pin, "");
    }

    #[test]
    fn request_restore_roundtrip() {
        let req = Request {
            command: "restore".into(),
            pin: "short".into(),
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let decoded: Request = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.command, "restore");
        assert_eq!(decoded.pin, "short");
    }

    #[test]
    fn response_success_without_error_field() {
        let mut map = std::collections::HashMap::new();
        map.insert("success", true);
        let bytes = rmp_serde::to_vec_named(&map).unwrap();
        let decoded: Response = rmp_serde::from_slice(&bytes).unwrap();
        assert!(decoded.success);
        assert!(decoded.error.is_none());
    }

    #[test]
    fn send_request_connection_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let req = Request {
            command: "save_and_switch".into(),
            pin: "123456".into(),
        };
        let err = send_request(port, &req).unwrap_err();
        assert!(
            err.starts_with("connect:"),
            "expected connect error, got: {err}"
        );
    }

    #[test]
    fn framing_restore_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).unwrap();
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut req_buf = vec![0u8; req_len];
            conn.read_exact(&mut req_buf).unwrap();

            let req: Request = rmp_serde::from_slice(&req_buf).unwrap();
            assert_eq!(req.command, "restore");

            #[derive(Serialize)]
            struct Resp {
                success: bool,
            }
            let resp_payload = rmp_serde::to_vec_named(&Resp { success: true }).unwrap();
            let len = (resp_payload.len() as u32).to_be_bytes();
            conn.write_all(&len).unwrap();
            conn.write_all(&resp_payload).unwrap();
        });

        let request = Request {
            command: "restore".into(),
            pin: "mypin".into(),
        };
        let resp = send_request(port, &request).unwrap();
        assert!(resp.success);
        server.join().unwrap();
    }

    #[test]
    fn server_returns_error_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).unwrap();
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut req_buf = vec![0u8; req_len];
            conn.read_exact(&mut req_buf).unwrap();

            #[derive(Serialize)]
            struct Resp {
                success: bool,
                error: Option<String>,
            }
            let resp_payload = rmp_serde::to_vec_named(&Resp {
                success: false,
                error: Some("invalid pin".into()),
            })
            .unwrap();
            let len = (resp_payload.len() as u32).to_be_bytes();
            conn.write_all(&len).unwrap();
            conn.write_all(&resp_payload).unwrap();
        });

        let request = Request {
            command: "save_and_switch".into(),
            pin: "wrong".into(),
        };
        let resp = send_request(port, &request).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.error.as_deref(), Some("invalid pin"));
        server.join().unwrap();
    }

    #[test]
    fn server_drops_connection_before_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).unwrap();
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut req_buf = vec![0u8; req_len];
            conn.read_exact(&mut req_buf).unwrap();
            drop(conn);
        });

        let request = Request {
            command: "save_and_switch".into(),
            pin: "123456".into(),
        };
        let err = send_request(port, &request).unwrap_err();
        assert!(
            err.contains("read response length"),
            "expected read error, got: {err}"
        );
        server.join().unwrap();
    }

    #[test]
    fn framing_exact_max_size_accepted() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).unwrap();
            let req_len = u32::from_be_bytes(len_buf) as usize;
            let mut req_buf = vec![0u8; req_len];
            conn.read_exact(&mut req_buf).unwrap();

            let big = vec![0xc0u8; MAX_RESPONSE_BYTES];
            let len = (big.len() as u32).to_be_bytes();
            conn.write_all(&len).unwrap();
            conn.write_all(&big).unwrap();
        });

        let request = Request {
            command: "restore".into(),
            pin: "000000".into(),
        };
        let err = send_request(port, &request).unwrap_err();
        assert!(
            err.contains("deserialize response"),
            "frame should be accepted but deserialization should fail, got: {err}"
        );
        server.join().unwrap();
    }

    #[test]
    fn multiple_sequential_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut conn, _) = listener.accept().unwrap();
                let mut len_buf = [0u8; 4];
                conn.read_exact(&mut len_buf).unwrap();
                let req_len = u32::from_be_bytes(len_buf) as usize;
                let mut req_buf = vec![0u8; req_len];
                conn.read_exact(&mut req_buf).unwrap();

                #[derive(Serialize)]
                struct Resp {
                    success: bool,
                }
                let resp_payload =
                    rmp_serde::to_vec_named(&Resp { success: true }).unwrap();
                let len = (resp_payload.len() as u32).to_be_bytes();
                conn.write_all(&len).unwrap();
                conn.write_all(&resp_payload).unwrap();
            }
        });

        for cmd in &["save_and_switch", "restore", "save_and_switch"] {
            let request = Request {
                command: cmd.to_string(),
                pin: "123".into(),
            };
            let resp = send_request(port, &request).unwrap();
            assert!(resp.success, "request '{cmd}' should succeed");
        }

        server.join().unwrap();
    }
}
