use bytes::Bytes;

use crossterm::terminal::WindowSize;
use inquire::{ui::RenderConfig, Confirm};

use crossterm::event::{NoTtyEvent, SenderWriter};
use rand::rng;
use russh::keys::ssh_key::PublicKey;
use std::sync::Arc;

use russh::keys::Algorithm;
use russh::server::*;
use russh::{Channel, ChannelId, Pty};
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone)]
struct AppServer {
    id: usize,
}

impl AppServer {
    pub fn new() -> Self {
        AppServer { id: 0 }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![
                russh::keys::PrivateKey::random(&mut rng(), Algorithm::Ed25519)
                    .map_err(russh::Error::from)?,
            ],

            nodelay: true,
            ..Default::default()
        };

        self.run_on_address(Arc::new(config), ("127.0.0.1", 2224))
            .await?;
        Ok(())
    }
}
impl Server for AppServer {
    type Handler = SshHandler;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> SshHandler {
        self.id += 1;
        let (app_send, term_recv) = channel::<Bytes>(64);
        let (psudo_tty, app_recv) = NoTtyEvent::new(term_recv);
        SshHandler::new(psudo_tty, app_send, Some(app_recv))
    }
}

struct SshHandler {
    pub pty: NoTtyEvent,
    pub send: Sender<Bytes>,
    pub recv: Option<Receiver<Bytes>>,
}
impl SshHandler {
    const fn new(pty: NoTtyEvent, send: Sender<Bytes>, recv: Option<Receiver<Bytes>>) -> Self {
        Self { pty, send, recv }
    }
}
impl Handler for SshHandler {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let _ = self.send.send(Bytes::copy_from_slice(data)).await;

        Ok(())
    }

    /// The client's window size has changed.
    async fn window_change_request(
        &mut self,
        _channel: ChannelId,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        *self.pty.window_size.lock() = WindowSize {
            rows: row_height as u16,
            columns: col_width as u16,
            width: pix_width as u16,
            height: pix_height as u16,
        };

        let mut win_raw = Vec::from(b"\x1B[W");
        let col = (col_width as u16).to_string();
        let row = (row_height as u16).to_string();
        win_raw.extend_from_slice(col.as_bytes());
        win_raw.push(b';');
        win_raw.extend_from_slice(row.as_bytes());
        win_raw.push(b'R');
        let _ = self.send.send(win_raw.into()).await;

        Ok(())
    }

    /// The client requests a pseudo-terminal with the given
    /// specifications.
    ///
    /// NOTE: Success or failure should be communicated to the client by calling
    /// `session.channel_success(channel)` or `session.channel_failure(channel)` respectively.
    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        *self.pty.window_size.lock() = WindowSize {
            rows: row_height as u16,
            columns: col_width as u16,
            width: pix_width as u16,
            height: pix_height as u16,
        };

        session.channel_success(channel)?;

        Ok(())
    }
    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let pty = self.pty.clone();
        let handle = session.handle();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Bytes>(5);
        let (tx_status, mut rx_status) = tokio::sync::mpsc::channel::<u8>(1);
        let app_recv_for_forward = self.recv.take().unwrap();
        let writer = SenderWriter::new(tx.clone());
        tokio::spawn(async move {
            let _ = run_shell(pty, tx_status, writer).await;
        });
        let mut r = app_recv_for_forward;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data= r.recv() =>{
                        if let Some(d)=data {
                            let _ = tx.send(d).await;
                        }
                    }
                    _ = rx_status.recv() => {
                        break
                    }
                }
            }
        });
        tokio::spawn(async move {
            loop {
                if let Some(data) = rx.recv().await {
                    let _ = handle.data(channel, data).await;
                } else {
                    let _ = handle.close(channel).await;
                    break;
                }
            }
        });
        session.channel_success(channel)?;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut server = AppServer::new();
    server.run().await.expect("Failed running server");
}

async fn run_shell(
    event: NoTtyEvent,
    tx_status: Sender<u8>,
    writer: SenderWriter,
) -> Result<(), anyhow::Error> {
    let ans = Confirm::new("Do you live in Brazil?")
        .with_default(false)
        .with_help_message("This data is stored for good reasons")
        .prompt(event.clone(), writer.clone())
        .await
        .unwrap();

    writer
        .write_all(&format!("Your answer: {ans}\r\n").into_bytes())
        .await?;

    let ans = Confirm::new("Do you want to move to another country?")
        .prompt(event.clone(), writer.clone())
        .await
        .unwrap();

    writer
        .write_all(&format!("Your answer: {ans}\r\n").into_bytes())
        .await?;

    let ans = Confirm {
        message: "Are you happy?",
        starting_input: None,
        default: Some(false),
        placeholder: Some("sim|não"),
        help_message: Some("It's alright if you're not"),
        formatter: &|ans| match ans {
            true => "sim".to_owned(),
            false => "não".to_owned(),
        },
        parser: &|ans| match ans {
            "sim" => Ok(true),
            "não" => Ok(false),
            _ => Err(()),
        },
        error_message: "Reply with 'sim' or 'não'".into(),
        default_value_formatter: &|def| match def {
            true => String::from("sim"),
            false => String::from("não"),
        },
        render_config: RenderConfig::default(),
    }
    .prompt(event.clone(), writer.clone())
    .await
    .unwrap();

    writer
        .write_all(&format!("Your answer: {ans}\r\n").into_bytes())
        .await?;
    tx_status.send(1).await?;
    Ok(())
}
