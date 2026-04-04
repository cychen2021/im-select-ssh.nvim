use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command")]
enum Request {
    #[serde(rename = "save_and_switch")]
    SaveAndSwitch,
    #[serde(rename = "restore")]
    Restore,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn main() {
    println!("im-select-server: not yet implemented");
    // TODO: Accept a socket path/port as CLI arg
    // TODO: Connect to the SSH-tunneled TCP port
    // TODO: Send MsgPack-encoded Request, read Response
}
