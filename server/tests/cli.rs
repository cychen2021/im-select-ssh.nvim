use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Request {
    command: String,
    pin: String,
}

#[derive(Serialize)]
struct Response {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_im-select-server")
}

fn read_request(conn: &mut std::net::TcpStream) -> Request {
    let mut len_buf = [0u8; 4];
    conn.read_exact(&mut len_buf).unwrap();
    let req_len = u32::from_be_bytes(len_buf) as usize;
    let mut req_buf = vec![0u8; req_len];
    conn.read_exact(&mut req_buf).unwrap();
    rmp_serde::from_slice(&req_buf).unwrap()
}

fn send_response(conn: &mut std::net::TcpStream, resp: &Response) {
    let bytes = rmp_serde::to_vec_named(resp).unwrap();
    conn.write_all(&(bytes.len() as u32).to_be_bytes()).unwrap();
    conn.write_all(&bytes).unwrap();
}

#[test]
fn help_flag_prints_description() {
    let out = Command::new(bin()).arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("IME switcher"));
}

#[test]
fn no_args_exits_with_error() {
    let out = Command::new(bin()).output().unwrap();
    assert!(!out.status.success());
}

#[test]
fn save_and_switch_success() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let srv = std::thread::spawn(move || {
        let (mut c, _) = listener.accept().unwrap();
        let req = read_request(&mut c);
        assert_eq!(req.command, "save_and_switch");
        assert_eq!(req.pin, "mypin");
        send_response(&mut c, &Response { success: true, error: None });
    });

    let out = Command::new(bin())
        .args(["save_and_switch", "--port", &port.to_string(), "--pin", "mypin"])
        .output()
        .unwrap();
    srv.join().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn restore_success() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let srv = std::thread::spawn(move || {
        let (mut c, _) = listener.accept().unwrap();
        let req = read_request(&mut c);
        assert_eq!(req.command, "restore");
        assert_eq!(req.pin, "pin2");
        send_response(&mut c, &Response { success: true, error: None });
    });

    let out = Command::new(bin())
        .args(["restore", "--port", &port.to_string(), "--pin", "pin2"])
        .output()
        .unwrap();
    srv.join().unwrap();
    assert!(out.status.success());
}

#[test]
fn server_error_causes_nonzero_exit() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let srv = std::thread::spawn(move || {
        let (mut c, _) = listener.accept().unwrap();
        let _ = read_request(&mut c);
        send_response(
            &mut c,
            &Response {
                success: false,
                error: Some("bad pin".into()),
            },
        );
    });

    let out = Command::new(bin())
        .args(["save_and_switch", "--port", &port.to_string(), "--pin", "wrong"])
        .output()
        .unwrap();
    srv.join().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("bad pin"));
}

#[test]
fn connection_refused_exits_nonzero() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let out = Command::new(bin())
        .args(["save_and_switch", "--port", &port.to_string(), "--pin", "test"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn pin_via_env_var() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let srv = std::thread::spawn(move || {
        let (mut c, _) = listener.accept().unwrap();
        let req = read_request(&mut c);
        assert_eq!(req.pin, "env_pin_123");
        send_response(&mut c, &Response { success: true, error: None });
    });

    let out = Command::new(bin())
        .args(["save_and_switch", "--port", &port.to_string()])
        .env("IM_SELECT_PIN", "env_pin_123")
        .output()
        .unwrap();
    srv.join().unwrap();
    assert!(out.status.success());
}
