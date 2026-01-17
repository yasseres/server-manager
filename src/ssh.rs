// =============================================================================
// SSH CONNECTION
// =============================================================================
// Handles SSH connections and command execution using the russh crate.
// =============================================================================

use russh::*;
use russh_keys::*;
use std::sync::Arc;

/// Connect to a server and execute a command with streaming output.
/// The callback is called for each line of output as it arrives.
pub fn connect_and_execute_with_callback<F>(
    ip: &str,
    username: &str,
    password: &str,
    command: &str,
    mut callback: F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: FnMut(&str),
{
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        // Add default port if not specified
        let address = if ip.contains(':') {
            ip.to_string()
        } else {
            format!("{}:22", ip)
        };

        let config = Arc::new(client::Config::default());
        let mut session = client::connect(config, &address, Client {}).await?;

        // Authenticate
        let auth_result = session.authenticate_password(username, password).await?;
        if !auth_result {
            return Err("Authentication failed".into());
        }

        // Execute command
        let mut channel = session.channel_open_session().await?;
        channel.exec(true, command).await?;

        // Read output with streaming
        let mut output = String::new();
        let mut code = None;
        let mut line_buffer = String::new();

        loop {
            let msg = channel.wait().await;
            match msg {
                Some(ChannelMsg::Data { ref data }) => {
                    let chunk = String::from_utf8_lossy(data);
                    output.push_str(&chunk);
                    line_buffer.push_str(&chunk);

                    while let Some(pos) = line_buffer.find('\n') {
                        let line = line_buffer[..pos].to_string();
                        line_buffer = line_buffer[pos + 1..].to_string();
                        callback(&line);
                    }
                }
                Some(ChannelMsg::ExtendedData { ref data, ext }) => {
                    let chunk = String::from_utf8_lossy(data);
                    output.push_str(&chunk);
                    line_buffer.push_str(&chunk);

                    while let Some(pos) = line_buffer.find('\n') {
                        let line = line_buffer[..pos].to_string();
                        line_buffer = line_buffer[pos + 1..].to_string();
                        if ext == 1 {
                            callback(&format!("[stderr] {}", line));
                        } else {
                            callback(&line);
                        }
                    }
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    code = Some(exit_status);
                }
                Some(ChannelMsg::Eof) => {
                    if !line_buffer.is_empty() {
                        callback(&line_buffer);
                    }
                    break;
                }
                None => break,
                _ => {}
            }
        }

        if let Some(exit_status) = code {
            if exit_status != 0 {
                return Err(format!(
                    "Command failed with exit code {}: {}",
                    exit_status,
                    output.trim()
                ).into());
            }
        }

        Ok(output)
    })
}

/// SSH client handler
struct Client {}

#[async_trait::async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys (for simplicity)
        // In production, you should verify the server's key
        Ok(true)
    }
}
