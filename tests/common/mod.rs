//! A one-shot HTTP server on the loopback interface: it answers the first
//! request it receives with a canned JSON body and hands that request back,
//! so the tests run without a network and can look at what was sent.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

/// What the server received.
#[derive(Debug)]
pub struct Recorded {
    /// The request line, e.g. `GET /reverse?format=json HTTP/1.1`.
    pub request_line: String,
    /// The header lines, names in lower case.
    pub headers: Vec<(String, String)>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Start the server; returns its base url (with a trailing `/`) and the
/// handle that resolves to the recorded request.
pub async fn serve_once(status: u16, body: &'static str) -> (String, JoinHandle<Recorded>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/", listener.local_addr().unwrap());

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();

        let mut received = Vec::new();
        let mut buffer = [0u8; 4096];
        while !received.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = socket.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            received.extend_from_slice(&buffer[..n]);
        }

        let response = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();

        let text = String::from_utf8(received).unwrap();
        let mut lines = text.split("\r\n");
        let request_line = lines.next().unwrap().to_string();
        let headers = lines
            .take_while(|line| !line.is_empty())
            .filter_map(|line| line.split_once(": "))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.to_string()))
            .collect();

        Recorded {
            request_line,
            headers,
        }
    });

    (base_url, handle)
}
