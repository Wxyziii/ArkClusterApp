//! Minimal Source RCON client for save_world before stop.
use anyhow::{bail, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_AUTH_RESPONSE: i32 = 2;

pub struct RconClient {
    stream: TcpStream,
}

impl RconClient {
    pub async fn connect(host: &str, port: u16, password: &str) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| anyhow::anyhow!("RCON connect timeout"))??;

        let mut client = Self { stream };
        client.send_packet(1, SERVERDATA_AUTH, password).await?;
        let resp = client.read_packet().await?;
        if resp.0 == -1 {
            bail!("RCON auth failed (wrong password?)");
        }
        Ok(client)
    }

    pub async fn exec(&mut self, command: &str) -> Result<String> {
        self.send_packet(2, SERVERDATA_EXECCOMMAND, command).await?;
        let (_, _, body) = self.read_packet().await?;
        Ok(body)
    }

    async fn send_packet(&mut self, id: i32, ptype: i32, body: &str) -> Result<()> {
        let body_bytes = body.as_bytes();
        let size = (4 + 4 + body_bytes.len() + 2) as i32;
        let mut buf = Vec::with_capacity(4 + size as usize);
        buf.extend_from_slice(&size.to_le_bytes());
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&ptype.to_le_bytes());
        buf.extend_from_slice(body_bytes);
        buf.push(0);
        buf.push(0);
        self.stream.write_all(&buf).await?;
        Ok(())
    }

    async fn read_packet(&mut self) -> Result<(i32, i32, String)> {
        let mut size_buf = [0u8; 4];
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.stream.read_exact(&mut size_buf),
        )
        .await
        .map_err(|_| anyhow::anyhow!("RCON read timeout"))??;
        let size = i32::from_le_bytes(size_buf) as usize;
        if size < 10 || size > 4096 {
            bail!("RCON packet size out of range: {}", size);
        }
        let mut data = vec![0u8; size];
        self.stream.read_exact(&mut data).await?;
        let id = i32::from_le_bytes(data[0..4].try_into()?);
        let ptype = i32::from_le_bytes(data[4..8].try_into()?);
        let body_end = data[8..].iter().position(|&b| b == 0).unwrap_or(data.len() - 8);
        let body = String::from_utf8_lossy(&data[8..8 + body_end]).to_string();
        Ok((id, ptype, body))
    }
}

pub async fn save_world(host: &str, port: u16, password: &str) -> Result<()> {
    let mut client = RconClient::connect(host, port, password).await?;
    let resp = client.exec("SaveWorld").await?;
    tracing::info!("SaveWorld response: {}", resp);
    Ok(())
}
