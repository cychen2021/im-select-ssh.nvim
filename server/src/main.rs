use std::io::{Read, Write};
use std::net::TcpStream;
use std::process;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

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
    #[arg(long)]
    pin: String,
}

#[derive(Subcommand, Clone)]
#[command(rename_all = "snake_case")]
enum Command {
    SaveAndSwitch(ConnectArgs),
    Restore(ConnectArgs),
}

#[derive(Debug, Serialize)]
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

fn send_request(port: u16, request: &Request) -> Result<Response, String> {
    let mut stream =
        TcpStream::connect(format!("127.0.0.1:{port}")).map_err(|e| format!("connect: {e}"))?;

    let payload = rmp_serde::to_vec_named(request).map_err(|e| format!("serialize: {e}"))?;
    let len = (payload.len() as u32).to_be_bytes();
    stream
        .write_all(&len)
        .map_err(|e| format!("write length: {e}"))?;
    stream
        .write_all(&payload)
        .map_err(|e| format!("write payload: {e}"))?;

    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| format!("read response length: {e}"))?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;

    if resp_len > 64 * 1024 {
        return Err(format!("response too large: {resp_len} bytes"));
    }

    let mut resp_buf = vec![0u8; resp_len];
    stream
        .read_exact(&mut resp_buf)
        .map_err(|e| format!("read response: {e}"))?;

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
}
