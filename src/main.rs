// =============================================================================
// Server Manager - GUI Application
// =============================================================================
// A graphical tool for managing multiple servers via SSH.
//
// MODULES:
// - config.rs: Server configuration loading from servers.toml
// - ssh.rs: SSH connection and command execution
// - commands.rs: Command scripts (test, info, update)
// =============================================================================

mod config;
mod ssh;
mod commands;

use config::{OsType, Server};
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Server Manager",
        options,
        Box::new(|cc| {
            // Softer dark theme - easier on the eyes
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(42, 42, 46);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 55, 60);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(65, 65, 72);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(75, 75, 85);
            visuals.selection.bg_fill = egui::Color32::from_rgb(70, 90, 120);
            visuals.extreme_bg_color = egui::Color32::from_rgb(32, 32, 36);
            visuals.faint_bg_color = egui::Color32::from_rgb(48, 48, 52);
            visuals.window_fill = egui::Color32::from_rgb(38, 38, 42);
            visuals.panel_fill = egui::Color32::from_rgb(38, 38, 42);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(ServerManagerApp::new()))
        }),
    )
}

// =============================================================================
// CLIXML PARSER - Convert PowerShell CLIXML to readable text
// =============================================================================
fn parse_clixml(input: &str) -> String {
    if !input.contains("<Obj") && !input.contains("<S ") {
        return input.to_string();
    }

    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();

    while i < chars.len() {
        if i + 2 < chars.len() && chars[i] == '<' && chars[i + 1] == 'S' {
            if let Some(tag_end) = input[i..].find('>') {
                let tag_start = i;
                let content_start = i + tag_end + 1;

                if let Some(close_pos) = input[content_start..].find("</S>") {
                    let content = &input[content_start..content_start + close_pos];
                    let tag = &input[tag_start..content_start];

                    let clean = content
                        .replace("_x000D__x000A_", "\n")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&amp;", "&")
                        .replace("&quot;", "\"");

                    let trimmed = clean.trim();
                    if !trimmed.is_empty() {
                        if tag.contains("S=\"Error\"") {
                            result.push_str("[ERROR] ");
                        } else if tag.contains("S=\"verbose\"") {
                            result.push_str("[VERBOSE] ");
                        } else if tag.contains("S=\"warning\"") {
                            result.push_str("[WARNING] ");
                        }
                        result.push_str(trimmed);
                        result.push('\n');
                    }

                    i = content_start + close_pos + 4;
                    continue;
                }
            }
        }

        if i + 10 < chars.len() && input[i..].starts_with("<ToString>") {
            let content_start = i + 10;
            if let Some(close_pos) = input[content_start..].find("</ToString>") {
                let content = &input[content_start..content_start + close_pos];
                let trimmed = content.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('<') {
                    result.push_str(trimmed);
                    result.push('\n');
                }
                i = content_start + close_pos + 11;
                continue;
            }
        }

        if i + 4 < chars.len() && input[i..].starts_with("<SD>") {
            let content_start = i + 4;
            if let Some(close_pos) = input[content_start..].find("</SD>") {
                let content = &input[content_start..content_start + close_pos];
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    result.push_str("[PROGRESS] ");
                    result.push_str(trimmed);
                    result.push('\n');
                }
                i = content_start + close_pos + 5;
                continue;
            }
        }

        i += 1;
    }

    if result.trim().is_empty() {
        let mut clean = String::new();
        let mut in_tag = false;
        for c in input.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                clean.push(c);
            }
        }
        return clean;
    }

    result
}

// =============================================================================
// SERVER STATE
// =============================================================================
#[derive(Clone)]
struct ServerState {
    name: String,
    ip: String,
    username: String,
    os_type: OsType,
    output: Arc<Mutex<String>>,
    is_running: Arc<Mutex<bool>>,
    status: Arc<Mutex<String>>,
    auth_failed: Arc<Mutex<bool>>,
}

impl ServerState {
    fn new(server: &Server) -> Self {
        Self {
            name: server.name.clone(),
            ip: server.ip.clone(),
            username: server.username.clone(),
            os_type: server.os_type.clone(),
            output: Arc::new(Mutex::new(String::new())),
            is_running: Arc::new(Mutex::new(false)),
            status: Arc::new(Mutex::new("Ready".to_string())),
            auth_failed: Arc::new(Mutex::new(false)),
        }
    }

    fn append_output(&self, text: &str) {
        let mut output = self.output.lock().unwrap();
        let clean = parse_clixml(text);
        output.push_str(&clean);
        if !clean.ends_with('\n') {
            output.push('\n');
        }
    }

    fn clear_output(&self) {
        self.output.lock().unwrap().clear();
    }

    fn set_status(&self, status: &str) {
        *self.status.lock().unwrap() = status.to_string();
    }

    fn set_running(&self, running: bool) {
        *self.is_running.lock().unwrap() = running;
    }

    fn is_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }

    fn get_output(&self) -> String {
        self.output.lock().unwrap().clone()
    }

    fn get_status(&self) -> String {
        self.status.lock().unwrap().clone()
    }

    fn set_auth_failed(&self, failed: bool) {
        *self.auth_failed.lock().unwrap() = failed;
    }

    fn auth_failed(&self) -> bool {
        *self.auth_failed.lock().unwrap()
    }
}

// =============================================================================
// MAIN APP STATE
// =============================================================================
struct ServerManagerApp {
    servers: Vec<ServerState>,
    config_error: Option<String>,
    selected_tab: usize,
    passwords: HashMap<String, String>,
    password_input: String,
    password_needed_for: Option<String>,
    password_error: Option<String>,
    pending_command: Option<PendingCommand>,
    last_command: Option<PendingCommand>,  // Store last command for retry
}

#[derive(Clone)]
struct PendingCommand {
    command: String,
    os_filter: Option<OsType>,
}

impl ServerManagerApp {
    fn new() -> Self {
        let (config_error, servers) = match config::load_config("servers.toml") {
            Ok(cfg) => {
                let servers: Vec<ServerState> = cfg.servers.iter().map(ServerState::new).collect();
                (None, servers)
            }
            Err(e) => (Some(e.to_string()), Vec::new()),
        };

        Self {
            servers,
            config_error,
            selected_tab: 0,
            passwords: HashMap::new(),
            password_input: String::new(),
            password_needed_for: None,
            password_error: None,
            pending_command: None,
            last_command: None,
        }
    }

    fn get_missing_passwords(&self, os_filter: Option<&OsType>) -> Vec<String> {
        let mut missing = Vec::new();
        for server in &self.servers {
            if let Some(os) = os_filter {
                if &server.os_type != os {
                    continue;
                }
            }
            if !self.passwords.contains_key(&server.username) && !missing.contains(&server.username) {
                missing.push(server.username.clone());
            }
        }
        missing
    }

    fn check_auth_failures(&mut self) {
        for server in &self.servers {
            if server.auth_failed() {
                server.set_auth_failed(false);
                let username = server.username.clone();
                self.passwords.remove(&username);
                self.password_error = Some(format!("Wrong password for '{}'. Please try again.", username));
                self.password_needed_for = Some(username);
                self.password_input.clear();

                // Set up pending command to retry the last command
                if let Some(ref last) = self.last_command {
                    self.pending_command = Some(last.clone());
                }
                break;
            }
        }
    }

    fn run_command(&mut self, command: &str, os_filter: Option<OsType>) {
        // Store as last command for potential retry
        self.last_command = Some(PendingCommand {
            command: command.to_string(),
            os_filter: os_filter.clone(),
        });

        for server in &self.servers {
            if let Some(ref os) = os_filter {
                if &server.os_type != os {
                    continue;
                }
            }

            if server.is_running() {
                continue;
            }

            let password = match self.passwords.get(&server.username) {
                Some(pw) => pw.clone(),
                None => continue,
            };

            let server_state = server.clone();
            let ip = server.ip.clone();
            let username = server.username.clone();
            let cmd = command.to_string();

            server_state.clear_output();
            server_state.set_running(true);
            server_state.set_status("Connecting...");
            server_state.append_output(&format!(">>> Connecting to {}@{}", username, ip));

            thread::spawn(move || {
                let output_clone = server_state.output.clone();

                server_state.set_status("Running...");

                let result = ssh::connect_and_execute_with_callback(
                    &ip,
                    &username,
                    &password,
                    &cmd,
                    move |line| {
                        let mut output = output_clone.lock().unwrap();
                        let clean = parse_clixml(line);
                        output.push_str(&clean);
                        if !clean.ends_with('\n') {
                            output.push('\n');
                        }
                    },
                );

                match result {
                    Ok(_) => {
                        server_state.append_output("---");
                        server_state.append_output(">>> Done");
                        server_state.set_status("Done");
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        server_state.append_output("---");
                        server_state.append_output(&format!(">>> ERROR: {}", error_msg));

                        if error_msg.contains("Authentication failed") {
                            server_state.set_auth_failed(true);
                            server_state.set_status("Auth Failed");
                        } else {
                            server_state.set_status("Error");
                        }
                    }
                }

                server_state.set_running(false);
            });
        }
    }

    fn start_command(&mut self, command: &str, os_filter: Option<OsType>) {
        let missing = self.get_missing_passwords(os_filter.as_ref());

        if missing.is_empty() {
            self.run_command(command, os_filter);
        } else {
            self.pending_command = Some(PendingCommand {
                command: command.to_string(),
                os_filter,
            });
            self.password_needed_for = Some(missing[0].clone());
            self.password_error = None;
            self.password_input.clear();
        }
    }

    fn submit_password(&mut self) {
        if let Some(username) = self.password_needed_for.take() {
            self.passwords.insert(username, self.password_input.clone());
            self.password_input.clear();
            self.password_error = None;

            if let Some(pending) = self.pending_command.take() {
                let missing = self.get_missing_passwords(pending.os_filter.as_ref());
                if missing.is_empty() {
                    self.run_command(&pending.command, pending.os_filter);
                } else {
                    self.pending_command = Some(pending);
                    self.password_needed_for = Some(missing[0].clone());
                }
            }
        }
    }
}

// =============================================================================
// UI RENDERING
// =============================================================================
impl eframe::App for ServerManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        self.check_auth_failures();

        // Password Dialog
        if self.password_needed_for.is_some() {
            egui::Window::new("Authentication")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .min_width(320.0)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(12.0);

                        if let Some(ref error) = self.password_error {
                            ui.colored_label(egui::Color32::from_rgb(220, 90, 90), error);
                            ui.add_space(8.0);
                        }

                        let username = self.password_needed_for.clone().unwrap();
                        ui.label(egui::RichText::new(format!("Password for: {}", username))
                            .size(15.0)
                            .color(egui::Color32::from_rgb(200, 200, 205)));
                        ui.add_space(12.0);

                        let response = ui.add_sized(
                            [280.0, 28.0],
                            egui::TextEdit::singleline(&mut self.password_input)
                                .password(true)
                                .hint_text("Enter password...")
                        );

                        if self.password_input.is_empty() {
                            response.request_focus();
                        }

                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.submit_password();
                        }

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.add_space(70.0);
                            if ui.add_sized([70.0, 26.0], egui::Button::new("OK")).clicked() {
                                self.submit_password();
                            }
                            ui.add_space(8.0);
                            if ui.add_sized([70.0, 26.0], egui::Button::new("Cancel")).clicked() {
                                self.password_needed_for = None;
                                self.pending_command = None;
                                self.password_input.clear();
                                self.password_error = None;
                            }
                        });
                        ui.add_space(8.0);
                    });
                });
        }

        // Top Panel
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(45, 45, 50))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Server Manager")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(210, 210, 215)));
                    ui.add_space(15.0);

                    if let Some(ref err) = self.config_error {
                        ui.colored_label(egui::Color32::from_rgb(220, 90, 90), err);
                    } else {
                        ui.label(egui::RichText::new(format!("{} servers", self.servers.len()))
                            .color(egui::Color32::from_rgb(140, 140, 150)));
                    }
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Test All").clicked() {
                        self.start_command(commands::test_cmd(), None);
                    }

                    if ui.button("Info Linux").clicked() {
                        self.start_command(commands::info_cmd_linux(), Some(OsType::Linux));
                    }

                    if ui.button("Info Windows").clicked() {
                        self.start_command(commands::info_cmd_windows(), Some(OsType::Windows));
                    }

                    ui.separator();

                    if ui.button("Update Linux").clicked() {
                        self.start_command(commands::update_linux_cmd(), Some(OsType::Linux));
                    }

                    if ui.button("Update Windows").clicked() {
                        self.start_command(commands::update_windows_cmd(), Some(OsType::Windows));
                    }

                    ui.separator();

                    if ui.button("Clear").clicked() {
                        for server in &self.servers {
                            server.clear_output();
                            server.set_status("Ready");
                        }
                    }
                });
            });

        // Left Panel - Server List
        egui::SidePanel::left("server_list")
            .min_width(200.0)
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(40, 40, 44))
                .inner_margin(egui::Margin::symmetric(8.0, 8.0)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Servers")
                    .size(14.0)
                    .color(egui::Color32::from_rgb(170, 170, 180)));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, server) in self.servers.iter().enumerate() {
                        let status = server.get_status();
                        let is_running = server.is_running();
                        let is_selected = self.selected_tab == i;

                        // Softer status colors
                        let status_color = if is_running {
                            egui::Color32::from_rgb(200, 170, 80)  // Soft yellow
                        } else if status == "Done" {
                            egui::Color32::from_rgb(100, 180, 100)  // Soft green
                        } else if status == "Error" || status == "Auth Failed" {
                            egui::Color32::from_rgb(200, 100, 100)  // Soft red
                        } else {
                            egui::Color32::from_rgb(120, 120, 130)  // Gray
                        };

                        let bg = if is_selected {
                            egui::Color32::from_rgb(55, 60, 70)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        egui::Frame::none()
                            .fill(bg)
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(status_color, "●");

                                    let os_color = match server.os_type {
                                        OsType::Linux => egui::Color32::from_rgb(200, 140, 60),
                                        OsType::Windows => egui::Color32::from_rgb(100, 140, 200),
                                    };
                                    let os_char = match server.os_type {
                                        OsType::Linux => "L",
                                        OsType::Windows => "W",
                                    };
                                    ui.colored_label(os_color, os_char);

                                    let name_color = if is_selected {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(230, 230, 235)
                                    };
                                    if ui.selectable_label(
                                        is_selected,
                                        egui::RichText::new(&server.name).color(name_color)
                                    ).clicked() {
                                        self.selected_tab = i;
                                    }
                                });
                            });
                        ui.add_space(2.0);
                    }
                });
            });

        // Main Panel - Output
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(35, 35, 40))
                .inner_margin(egui::Margin::symmetric(12.0, 10.0)))
            .show(ctx, |ui| {
                if self.servers.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("No servers. Check servers.toml")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(140, 140, 150)));
                    });
                    return;
                }

                if self.selected_tab >= self.servers.len() {
                    self.selected_tab = 0;
                }

                let server = &self.servers[self.selected_tab];

                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&server.name)
                        .size(16.0)
                        .color(egui::Color32::from_rgb(210, 210, 215)));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(format!("{}@{}", server.username, server.ip))
                        .color(egui::Color32::from_rgb(130, 130, 140))
                        .monospace());
                    ui.add_space(10.0);

                    let status = server.get_status();
                    let status_color = if server.is_running() {
                        egui::Color32::from_rgb(200, 170, 80)
                    } else if status == "Done" {
                        egui::Color32::from_rgb(100, 180, 100)
                    } else if status == "Error" || status == "Auth Failed" {
                        egui::Color32::from_rgb(200, 100, 100)
                    } else {
                        egui::Color32::from_rgb(120, 120, 130)
                    };

                    ui.colored_label(status_color, &status);
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Output
                let output = server.get_output();
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(28, 28, 32))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut output.as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(28)
                                        .text_color(egui::Color32::from_rgb(190, 190, 195))
                                );
                            });
                    });
            });
    }
}
