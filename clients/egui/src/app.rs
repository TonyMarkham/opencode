use eframe::egui;
use std::sync::{Arc, mpsc};
use tokio::runtime::Runtime;
use serde::Deserialize;

use crate::discovery::process::{ServerInfo, check_health, discover, stop_pid};
use crate::discovery::spawn::spawn_and_wait;

pub struct OpenCodeApp {
    // Multi-session tabs (server-backed sessions in later milestones)
    tabs: Vec<Tab>,
    active: usize,

    // Server state
    server: Option<ServerInfo>,
    server_error: Option<String>,
    server_in_flight: bool,
    discovery_started: bool,

    // Async runtime + UI channel
    runtime: Option<Arc<Runtime>>,
    ui_rx: Option<mpsc::Receiver<UiMsg>>,
    ui_tx: Option<mpsc::Sender<UiMsg>>,

    // API client
    client: Option<crate::client::api::OpencodeClient>,
    
    // Audio task
    audio_tx: Option<mpsc::Sender<AudioCmd>>,
    audio_enabled: bool,
    recording_state: RecordingState,

    // Rename state
    renaming_tab: Option<usize>,
    rename_buffer: String,
    rename_text_selected: bool,

    // Config and settings
    config: crate::config::AppConfig,
    show_settings: bool,
    base_url_input: String,
    directory_input: String,
    
    // Permission handling
    pending_permissions: Vec<PermissionInfo>,
    
    // Markdown rendering
    commonmark_cache: egui_commonmark::CommonMarkCache,
}

#[derive(Default, Clone)]
struct Tab {
    title: String,
    session_id: Option<String>,
    directory: Option<String>,
    messages: Vec<DisplayMessage>,
    input: String,
}

#[derive(Clone)]
struct DisplayMessage {
    message_id: String,
    role: String,
    text_parts: Vec<String>,
    tool_calls: Vec<ToolCall>,
}

#[derive(Clone)]
struct ToolCall {
    name: String,
    status: String,
    call_id: Option<String>,
}

enum UiMsg {
    ServerConnected(ServerInfo),
    ServerError(String),
    SessionCreated { tab_idx: usize, id: String, title: String, directory: String },
    GlobalEvent(serde_json::Value),
    PermissionRequest(PermissionInfo),
    // Audio events
    RecordingStarted,
    RecordingStopped,
    Transcription(String),
    AudioError(String),
}

#[derive(Clone, Debug, Deserialize)]
struct PermissionInfo {
    id: String,
    #[serde(rename = "type")]
    perm_type: String,
    pattern: Option<Vec<String>>,
    #[serde(rename = "sessionID")]
    session_id: String,
    #[serde(rename = "messageID")]
    message_id: String,
    #[serde(rename = "callID")]
    call_id: Option<String>,
    title: String,
    metadata: serde_json::Value,
    time: PermissionTime,
}

#[derive(Clone, Debug, Deserialize)]
struct PermissionTime {
    created: u64,
}

enum AudioCmd {
    StartRecording,
    StopRecording,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordingState {
    Idle,
    Recording,
}

impl OpenCodeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders for colored emoji support via egui-twemoji
        egui_extras::install_image_loaders(&cc.egui_ctx);
        
        // Load config and apply UI preferences
        let config = crate::config::AppConfig::load();
        config.ui.apply_to_context(&cc.egui_ctx);
        
        Self {
            tabs: Vec::new(),
            active: 0,
            server: None,
            server_error: None,
            server_in_flight: false,
            discovery_started: false,
            runtime: None,
            ui_rx: None,
            ui_tx: None,
            client: None,
            audio_tx: None,
            audio_enabled: false,
            recording_state: RecordingState::Idle,
            renaming_tab: None,
            rename_buffer: String::new(),
            rename_text_selected: false,
            config: config.clone(),
            show_settings: false,
            base_url_input: config.server.last_base_url.unwrap_or_default(),
            directory_input: config.server.directory_override.clone().unwrap_or_default(),
            pending_permissions: Vec::new(),
            commonmark_cache: egui_commonmark::CommonMarkCache::default(),
        }
    }

    fn start_server_discovery(&mut self, ctx: &egui::Context) {
        // Lazy-init runtime on first call
        if self.runtime.is_none() {
            let rt = Arc::new(Runtime::new().expect("tokio runtime"));
            let (tx, rx) = mpsc::channel();
            self.runtime = Some(rt.clone());
            self.ui_rx = Some(rx);
            self.ui_tx = Some(tx.clone());
            
            // Start audio task if model is configured or auto-detected
            let model_path = if let Some(configured_path) = &self.config.audio.whisper_model_path {
                Some(std::path::PathBuf::from(configured_path))
            } else {
                // Auto-detect model relative to executable (for cargo make dev)
                std::env::current_exe().ok()
                    .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                    .map(|exe_dir| exe_dir.join("models").join("ggml-base.en.bin"))
                    .filter(|path| path.exists())
            };
            
            if let Some(path) = model_path {
                eprintln!("Starting audio task with model: {}", path.display());
                self.start_audio_task(&rt, tx.clone(), path, ctx);
            } else {
                eprintln!("No Whisper model found. Run 'cargo make dev' to auto-setup.");
            }
        }

        self.server_in_flight = true;
        self.discovery_started = true;
        
        let tx = self.ui_tx.as_ref().unwrap().clone();
        let rt = self.runtime.as_ref().unwrap().clone();
        let egui_ctx = ctx.clone();
        
        rt.spawn(async move {
            let msg = try_discover_or_spawn().await;
            let _ = tx.send(msg);
            egui_ctx.request_repaint();
        });
    }
    
    fn start_audio_task(
        &mut self, 
        runtime: &Arc<Runtime>, 
        ui_tx: mpsc::Sender<UiMsg>,
        model_path: std::path::PathBuf,
        ctx: &egui::Context
    ) {
        
        let (audio_tx, audio_rx) = mpsc::channel::<AudioCmd>();
        self.audio_tx = Some(audio_tx);
        
        let egui_ctx = ctx.clone();
        runtime.spawn(async move {
            run_audio_task(audio_rx, ui_tx, model_path, egui_ctx).await;
        });
    }

/// Drain the UI message channel fed by background tasks (e.g., SSE subscription).
/// This is not network polling; it only drains already-received events.
fn drain_ui_msgs(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.ui_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
UiMsg::ServerConnected(info) => {
                        // Create client and subscribe to SSE
                        let base = info.base_url.clone();
                        match crate::client::api::OpencodeClient::new(&base) {
                            Ok(mut c) => {
                                // Apply directory override to header if configured
                                if let Some(dir) = &self.config.server.directory_override {
                                    c.directory = Some(std::path::PathBuf::from(dir));
                                }
                                self.client = Some(c)
                            },
                            Err(e) => self.server_error = Some(e.to_string()),
                        }
                        if let Some(rt) = &self.runtime {
                            let tx2 = self.ui_tx.as_ref().unwrap().clone();
                            let egui_ctx = ctx.clone();
                            let base_for_sse = base.clone();
                            rt.spawn(async move {
                                if let Ok(mut rx) = crate::client::events::subscribe_global(&base_for_sse).await {
                                    while let Some(ev) = rx.recv().await {
                                        let _ = tx2.send(UiMsg::GlobalEvent(ev.payload.clone()));
                                        egui_ctx.request_repaint();
                                    }
                                }
                            });
                        }
                        // Save base_url to config
                        self.config.server.last_base_url = Some(base.clone());
                        self.base_url_input = base;
                        self.config.save();
                        
                        self.server = Some(info);
                        self.server_error = None;
                        self.server_in_flight = false;
                    }
                    UiMsg::ServerError(err) => {
                        self.server_error = Some(err);
                        self.server = None;
                        self.server_in_flight = false;
                    }
UiMsg::SessionCreated { tab_idx, id, title, directory } => {
                        if let Some(tab) = self.tabs.get_mut(tab_idx) {
                            tab.title = title;
                            tab.session_id = Some(id);
                            tab.directory = Some(directory);
                        }
                    }
                    UiMsg::GlobalEvent(payload) => {
                        // Permission handling
                        let event_type = payload.get("type").and_then(|v| v.as_str());
                        match event_type {
                            Some("permission.updated") => {
                                if let Some(props) = payload.get("properties") {
                                    match serde_json::from_value::<PermissionInfo>(props.clone()) {
                                        Ok(info) => {
                                            self.pending_permissions.push(info);
                                        }
                                        Err(_e) => { }
                                    }
                                }
                                // Do not route permission events into chat rendering
                                continue;
                            }
                            Some("permission.replied") => {
                                if let Some(props) = payload.get("properties") {
                                    let pid = props.get("permissionID").and_then(|v| v.as_str());
                                    let sid = props.get("sessionID").and_then(|v| v.as_str());
                                    let resp = props.get("response").and_then(|v| v.as_str()).unwrap_or("?");
                                    if let (Some(pid), Some(sid)) = (pid, sid) {
                                        if let Some(idx) = self
                                            .pending_permissions
                                            .iter()
                                            .position(|p| p.id == pid && p.session_id == sid)
                                        {
                                            self.pending_permissions.remove(idx);
                                        }
                                    }
                                }
                                continue;
                            }
                            _ => {}
                        }

                        // Extract sessionID from event for chat updates
                        let sid_opt = payload
                            .get("properties")
                            .and_then(|p| p.get("part"))
                            .and_then(|part| part.get("sessionID"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                payload
                                    .get("properties")
                                    .and_then(|p| p.get("info"))
                                    .and_then(|info| info.get("sessionID"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            });
                        
                        if let Some(sid) = sid_opt {
                            if let Some(tab) = self
                                .tabs
                                .iter_mut()
                                .find(|t| t.session_id.as_deref() == Some(&sid))
                            {
                                Self::handle_event(tab, &payload);
                            }
                        }
                    }
                    UiMsg::RecordingStarted => {
                        self.audio_enabled = true;
                        // Add notification message to chat history (client-side only)
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            tab.messages.push(DisplayMessage {
                                message_id: format!("audio_rec_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                                role: "system".to_string(),
                                text_parts: vec!["🎙 Recording...".to_string()],  // Studio microphone emoji (no variation selector)
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::RecordingStopped => {
                        // Add processing message to chat history (client-side only)
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            tab.messages.push(DisplayMessage {
                                message_id: format!("audio_proc_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                                role: "system".to_string(),
                                text_parts: vec!["Processing audio...".to_string()],
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::Transcription(text) => {
                        self.audio_enabled = false;
                        // Insert transcription into active tab's input field
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            if !tab.input.is_empty() {
                                tab.input.push(' ');
                            }
                            tab.input.push_str(&text);
                            // Add success message to chat history (client-side only)
                            tab.messages.push(DisplayMessage {
                                message_id: format!("audio_done_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                                role: "system".to_string(),
                                text_parts: vec!["✅ Transcription complete".to_string()],  // Check mark button emoji
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::PermissionRequest(info) => {
                        self.pending_permissions.push(info);
                    }
                    UiMsg::AudioError(err) => {
                        self.audio_enabled = false;
                        self.recording_state = RecordingState::Idle;
                        // Add error message to chat history (client-side only)
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            tab.messages.push(DisplayMessage {
                                message_id: format!("audio_err_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                                role: "system".to_string(),
                                text_parts: vec![format!("⚠ Audio: {}", err)],  // Warning emoji (no variation selector)
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    fn action_reconnect(&mut self, ctx: &egui::Context) {
        if self.server_in_flight || self.runtime.is_none() {
            return;
        }
        self.server_in_flight = true;
        let tx = self.ui_tx.as_ref().unwrap().clone();
        let rt = self.runtime.as_ref().unwrap().clone();
        let egui_ctx = ctx.clone();
        rt.spawn(async move {
            let msg = try_discover_or_spawn().await;
            let _ = tx.send(msg);
            egui_ctx.request_repaint();
        });
    }

    fn handle_event(tab: &mut Tab, payload: &serde_json::Value) {
        let event_type = payload.get("type").and_then(|v| v.as_str());
        
        match event_type {
            Some("message.updated") => {
                // New message started - only create if ID doesn't exist
                if let Some(props) = payload.get("properties") {
                    if let Some(info) = props.get("info") {
                        let message_id = info.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let role = info.get("role").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        
                        // Only create if this message ID doesn't already exist
                        if !tab.messages.iter().any(|m| m.message_id == message_id) {
                            tab.messages.push(DisplayMessage {
                                message_id: message_id,
                                role: role,
                                text_parts: Vec::new(),
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                }
            }
            Some("message.part.updated") => {
                // Part of a message - text events contain full accumulated content, not deltas
                if let Some(props) = payload.get("properties") {
                    if let Some(part) = props.get("part") {
                        let part_type = part.get("type").and_then(|v| v.as_str());
                        if part_type == Some("text") {
                            let text = part.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            if let Some(msg) = tab.messages.last_mut() {
                                // Clear and replace - each event has the full text so far
                                msg.text_parts.clear();
                                msg.text_parts.push(text.to_string());
                            }
                        } else if part_type == Some("tool") {
                            let tool_name = part.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let state = part.get("state").and_then(|v| v.get("status")).and_then(|v| v.as_str()).unwrap_or("unknown");
                            let call_id = part.get("callID").and_then(|v| v.as_str()).map(|s| s.to_string());
                            if let Some(msg) = tab.messages.last_mut() {
                                // Prefer matching by call_id when present, otherwise fallback to name
                                if let Some(existing) = msg.tool_calls.iter_mut().find(|t| {
                                    if let (Some(a), Some(b)) = (&t.call_id, &call_id) { a == b } else { t.name == tool_name }
                                }) {
                                    existing.status = state.to_string();
                                    if existing.call_id.is_none() { existing.call_id = call_id.clone(); }
                                } else {
                                    msg.tool_calls.push(ToolCall {
                                        name: tool_name.to_string(),
                                        status: state.to_string(),
                                        call_id: call_id,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn render_message(
        &mut self,
        ui: &mut egui::Ui, 
        msg: &DisplayMessage,
        session_id: Option<&str>,
    ) {
        let message_id = msg.message_id.clone();
        let available_width = ui.available_width();
        let bubble_max_width = available_width * 0.75;
        
        // Determine colors and alignment
        let (bg_color, align_right) = match msg.role.as_str() {
            "user" => (egui::Color32::from_rgb(60, 100, 180), true),
            "assistant" => (egui::Color32::from_rgb(70, 70, 70), false),
            _ => (egui::Color32::from_rgb(100, 70, 120), false),
        };
        
        ui.add_space(8.0);
        
        // Combine text parts into a single markdown string
        let full_text = msg.text_parts.join("");
        
        ui.horizontal(|ui| {
            if align_right {
                // User messages: right-aligned
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    egui::Frame::new()
                        .fill(bg_color)
                        .corner_radius(10.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.set_max_width(bubble_max_width);
                            if !full_text.is_empty() {
                                egui_commonmark::CommonMarkViewer::new()
                                    .show(ui, &mut self.commonmark_cache, &full_text);
                            }
                        });
                    
                    ui.add_space(6.0);
                    
                    if ui.button("Copy").clicked() {
                        ui.ctx().copy_text(full_text.clone());
                    }
                });
            } else {
                // Assistant/system messages: left-aligned
                egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(10.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_max_width(bubble_max_width);
                        if !full_text.is_empty() {
                            // For system messages, use EmojiLabel to render colored emojis
                            // For assistant messages, use CommonMarkViewer for markdown support
                            if msg.role == "system" {
                                egui_twemoji::EmojiLabel::new(&full_text).show(ui);
                            } else {
                                egui_commonmark::CommonMarkViewer::new()
                                    .show(ui, &mut self.commonmark_cache, &full_text);
                            }
                        } else if msg.role == "assistant" && msg.tool_calls.is_empty() {
                            // Show spinner only when no text AND no tools (truly waiting for response)
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Thinking...");
                            });
                        }
                        
                        // Tool calls (collapsible)
                        if !msg.tool_calls.is_empty() {
                            ui.add_space(8.0);
                            
                            // Check if any tools are still in progress
                            let any_in_progress = msg.tool_calls.iter().any(|t| 
                                t.status != "success" && t.status != "error" && t.status != "completed"
                            );
                            
                            // Check if any tool has a pending permission
                            let has_pending_perm = session_id.is_some() && msg.tool_calls.iter().any(|tool| {
                                if let Some(call_id) = &tool.call_id {
                                    self.pending_permissions.iter().any(|p| 
                                        p.session_id == session_id.unwrap() && p.call_id.as_deref() == Some(call_id.as_str())
                                    )
                                } else {
                                    false
                                }
                            });
                            
                            ui.horizontal(|ui| {
                                if any_in_progress {
                                    ui.spinner();
                                }
                                // Tool calls header with wrench emoji
                                let header_text = format!("🔧 {} tool call(s)", msg.tool_calls.len());
                                egui::CollapsingHeader::new(header_text)
                                    .id_salt(&message_id)
                                    .default_open(has_pending_perm)  // Auto-expand if permission pending
                                    .show(ui, |ui| {
                                    for tool in &msg.tool_calls {
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                let status_icon = match tool.status.as_str() {
                                                    "success" | "completed" => "✅",
                                                    "error" => "❌",
                                                    _ => "⏳",
                                                };
                                                ui.label(&tool.name);
                                                egui_twemoji::EmojiLabel::new(format!("{} {}", status_icon, tool.status)).show(ui);
                                            });

                                            // Inline permission card for this tool if pending
                                            if let (Some(sid), Some(call)) = (session_id, &tool.call_id) {
                                                let perm_opt = self
                                                    .pending_permissions
                                                    .iter()
                                                    .find(|p| p.session_id == sid && p.call_id.as_deref() == Some(call.as_str()))
                                                    .cloned();
                                                if let Some(perm) = perm_opt {
                                                    ui.add_space(4.0);
                                                    egui::Frame::none()
                                                        .fill(egui::Color32::from_gray(40))
                                                        .inner_margin(egui::Margin::symmetric(8i8, 6i8))
                                                        .show(ui, |ui| {
                                                            ui.label(format!("Permission required: {}", perm.title));
                                                            ui.small(format!("Type: {}", perm.perm_type));
                                                            ui.add_space(6.0);
                                                            ui.horizontal(|ui| {
                                                                if ui.button("❌ Reject").clicked() {
                                                                    self.action_respond_permission(perm.session_id.clone(), perm.id.clone(), "reject");
                                                                    if let Some(idx) = self.pending_permissions.iter().position(|p| p.id == perm.id) { self.pending_permissions.remove(idx); }
                                                                }
                                                                if ui.button("✅ Allow Once").clicked() {
                                                                    self.action_respond_permission(perm.session_id.clone(), perm.id.clone(), "once");
                                                                    if let Some(idx) = self.pending_permissions.iter().position(|p| p.id == perm.id) { self.pending_permissions.remove(idx); }
                                                                }
                                                                if ui.button("✅ Always Allow").clicked() {
                                                                    self.action_respond_permission(perm.session_id.clone(), perm.id.clone(), "always");
                                                                    if let Some(idx) = self.pending_permissions.iter().position(|p| p.id == perm.id) { self.pending_permissions.remove(idx); }
                                                                }
                                                            });
                                                        });
                                                }
                                            }
                                        });
                                    }
                                });
                            });
                        }
                    });
                
                ui.add_space(6.0);
                
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(full_text.clone());
                }
            }
        });
        
        ui.add_space(4.0);
    }
    
    fn action_start_only(&mut self, ctx: &egui::Context) {
        if self.server_in_flight || self.runtime.is_none() {
            return;
        }
        self.server_in_flight = true;
        let tx = self.ui_tx.as_ref().unwrap().clone();
        let rt = self.runtime.as_ref().unwrap().clone();
        let egui_ctx = ctx.clone();
        rt.spawn(async move {
            let msg = match spawn_and_wait().await {
                Ok(info) => UiMsg::ServerConnected(info),
                Err(e) => UiMsg::ServerError(e.to_string()),
            };
            let _ = tx.send(msg);
            egui_ctx.request_repaint();
        });
    }
}

impl OpenCodeApp {
    fn action_respond_permission(&mut self, session_id: String, perm_id: String, response: &str) {
        if let (Some(client), Some(rt)) = (&self.client, &self.runtime) {
            let c = client.clone();
            let resp = response.to_string();
            rt.spawn(async move {
                let _ = c.respond_permission(&session_id, &perm_id, &resp).await;
            });
        }
    }
}

async fn try_discover_or_spawn() -> UiMsg {
    match discover() {
        Ok(Some(info)) => {
            if check_health(&info.base_url).await {
                UiMsg::ServerConnected(info)
            } else {
                match spawn_and_wait().await {
                    Ok(info) => UiMsg::ServerConnected(info),
                    Err(e) => UiMsg::ServerError(e.to_string()),
                }
            }
        }
        Ok(None) => match spawn_and_wait().await {
            Ok(info) => UiMsg::ServerConnected(info),
            Err(e) => UiMsg::ServerError(e.to_string()),
        },
        Err(e) => UiMsg::ServerError(e.to_string()),
    }
}

impl eframe::App for OpenCodeApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // Push-to-talk state machine
        // State transitions:
        // - (Idle, key_down) -> Recording + send StartRecording
        // - (Recording, key_up) -> Idle + send StopRecording
        // - All other transitions ignored (prevents double-triggers)
        
        // Only process if audio task is running
        if self.audio_tx.is_none() {
            // Debug: Check if AltRight is being pressed
            for event in &raw_input.events {
                if let egui::Event::Key { key, pressed, .. } = event {
                    let key_name = format!("{:?}", key);
                    if key_name == "AltRight" && *pressed {
                        eprintln!("AltRight pressed but audio task not running (no model configured)");
                    }
                }
            }
            return;
        }
        
        for event in &raw_input.events {
            if let egui::Event::Key { key, pressed, repeat, .. } = event {
                // Ignore key repeats
                if *repeat {
                    continue;
                }
                
                // Check if this is our push-to-talk key
                let key_name = format!("{:?}", key);
                if key_name != self.config.audio.push_to_talk_key {
                    continue;
                }
                
                // State machine
                match (self.recording_state, *pressed) {
                    (RecordingState::Idle, true) => {
                        // Key pressed - start recording
                        self.recording_state = RecordingState::Recording;
                        if let Some(tx) = &self.audio_tx {
                            let _ = tx.send(AudioCmd::StartRecording);
                        }
                    }
                    (RecordingState::Recording, false) => {
                        // Key released - stop recording
                        self.recording_state = RecordingState::Idle;
                        if let Some(tx) = &self.audio_tx {
                            let _ = tx.send(AudioCmd::StopRecording);
                        }
                    }
                    _ => {
                        // Ignore other transitions (key repeats, etc.)
                    }
                }
            }
        }
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Start server discovery on first frame (lazy init)
        if !self.discovery_started {
            self.start_server_discovery(ctx);
        }
        
        // Drain async messages (SSE-fed channel)
        self.drain_ui_msgs(ctx);
        
        // Auto-create first tab when client is ready
        if self.tabs.is_empty() && self.client.is_some() && self.runtime.is_some() && self.ui_tx.is_some() {
            let tab_idx = 0;
self.tabs.push(Tab { 
                title: "(creating…)".to_string(), 
                session_id: None,
                directory: None, 
                messages: Vec::new(),
                input: String::new(),
            });
            self.active = 0;
            
            let txc = self.ui_tx.as_ref().unwrap().clone();
            let c = self.client.as_ref().unwrap().clone();
            let egui_ctx = ctx.clone();
            let rt = self.runtime.as_ref().unwrap().clone();
            
            rt.spawn(async move {
match c.create_session(None).await {
                    Ok(info) => { let _ = txc.send(UiMsg::SessionCreated { tab_idx, id: info.id, title: info.title, directory: info.directory }); }
                    Err(e) => { let _ = txc.send(UiMsg::ServerError(e.to_string())); }
                }
                egui_ctx.request_repaint();
            });
        }

        // Top: Tabs + Server panel
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Tabs
                let mut to_close: Option<usize> = None;
                let mut rename_action: Option<(usize, String)> = None;
                let mut cancel_rename = false;
                
                for (i, tab) in self.tabs.iter().enumerate() {
                    let selected = self.active == i;
                    
                    // Group tab label and close button together
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            
                            // Check if this tab is being renamed
                            if self.renaming_tab == Some(i) {
                                let text_edit = egui::TextEdit::singleline(&mut self.rename_buffer);
                                let response = text_edit.show(ui).response;
                                
                                // Request focus and select all text on first frame only
                                let id = response.id;
                                response.request_focus();
                                if response.has_focus() && !self.rename_text_selected {
                                    // Select all text (only once)
                                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                                        let text_len = self.rename_buffer.len();
                                        state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                                            egui::text::CCursor::new(0),
                                            egui::text::CCursor::new(text_len),
                                        )));
                                        state.store(ui.ctx(), id);
                                        self.rename_text_selected = true;
                                    }
                                }
                                
                                // Confirm on Enter or Tab
                                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                let tab_pressed = ui.input(|i| i.key_pressed(egui::Key::Tab));
                                // Confirm on losing focus (click elsewhere)
                                let lost_focus = response.lost_focus();
                                // Cancel on Escape
                                let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                
                                if escape_pressed {
                                    cancel_rename = true;
                                } else if enter_pressed || tab_pressed || lost_focus {
                                    if !self.rename_buffer.trim().is_empty() {
                                        rename_action = Some((i, self.rename_buffer.clone()));
                                    }
                                    cancel_rename = true;
                                }
                            } else {
                                let response = ui.selectable_label(selected, &tab.title);
                                
                                if response.clicked() {
                                    self.active = i;
                                }
                                
                                // Right-click context menu
                                response.context_menu(|ui| {
                                    if ui.button("Rename").clicked() {
                                        self.renaming_tab = Some(i);
                                        self.rename_buffer = tab.title.clone();
                                        self.rename_text_selected = false;
                                        ui.close();
                                    }
                                });
                            }
                            
                            if ui.small_button("X").clicked() {
                                to_close = Some(i);
                            }
                        });
                    });
                }
                
                // Apply deferred actions
                if let Some((idx, new_title)) = rename_action {
                    if let Some(tab) = self.tabs.get_mut(idx) {
                        tab.title = new_title;
                    }
                }
                if cancel_rename {
                    self.renaming_tab = None;
                    self.rename_buffer.clear();
                    self.rename_text_selected = false;
                }
                if let Some(idx) = to_close {
                    self.tabs.remove(idx);
                    if self.active >= self.tabs.len() && self.active > 0 {
                        self.active = self.tabs.len() - 1;
                    }
                    // Cancel rename if we closed the tab being renamed
                    if self.renaming_tab == Some(idx) {
                        self.renaming_tab = None;
                        self.rename_buffer.clear();
                        self.rename_text_selected = false;
                    } else if let Some(r) = self.renaming_tab {
                        if r > idx {
                            self.renaming_tab = Some(r - 1);
                        }
                    }
                }
                if ui.button("+").clicked() {
                    let tab_idx = self.tabs.len();
self.tabs.push(Tab { 
                        title: "(creating…)".to_string(), 
                        session_id: None,
                        directory: None, 
                        messages: Vec::new(),
                        input: String::new(),
                    });
                    self.active = tab_idx;
                    if let (Some(rt), Some(tx), Some(client)) = (&self.runtime, &self.ui_tx, &self.client) {
                        let txc = tx.clone();
                        let c = client.clone();
                        let egui_ctx = ctx.clone();
                        rt.spawn(async move {
match c.create_session(None).await {
                                Ok(info) => { let _ = txc.send(UiMsg::SessionCreated { tab_idx, id: info.id, title: info.title, directory: info.directory }); }
                                Err(e) => { let _ = txc.send(UiMsg::ServerError(e.to_string())); }
                            }
                            egui_ctx.request_repaint();
                        });
                    }
                }

                ui.separator();

                // Server status (minimal)
                if let Some(info) = &self.server {
                    ui.label(format!("Server: {} (PID {})", info.base_url, info.pid));
                } else if self.server_in_flight {
                    ui.label("Server: connecting…");
                } else {
                    ui.label("Server: not connected");
                }
                
                // Settings button
                if ui.button("⚙ Settings").clicked() {
                    self.show_settings = !self.show_settings;
                }
            });
        });

        // Settings Window - handle actions with deferred execution
        let mut reconnect_requested = false;
        let mut start_requested = false;
        let mut stop_requested = false;
        
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .default_width(600.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Server Preferences Section
                        ui.collapsing("Server Preferences", |ui| {
                            ui.heading("Server Connection");
                            ui.separator();
                            
                            // Manual URL override
                            ui.horizontal(|ui| {
                                ui.label("Base URL:");
                                ui.text_edit_singleline(&mut self.base_url_input);
                            });
                            ui.small("Leave empty for auto-discovery");

                            ui.add_space(8.0);

                            // Directory override
                            ui.horizontal(|ui| {
                                ui.label("Directory override:");
                                ui.text_edit_singleline(&mut self.directory_input);
                            });
                            ui.small("Optional. Sends as x-opencode-directory header.");
                            
                            ui.add_space(8.0);
                            
                            // Auto-start toggle
                            ui.checkbox(&mut self.config.server.auto_start, "Auto-start server on launch");
                            
                            ui.add_space(8.0);
                            ui.separator();
                            
                            // Discovery diagnostics
                            if let Some(info) = &self.server {
                                ui.label(format!("Connected: {} (PID {})", info.base_url, info.pid));
                                ui.label(format!("Owned: {}", info.owned));
                            } else {
                                ui.label("Status: Not connected");
                            }
                            let dir_label = if self.directory_input.trim().is_empty() { "(none)".to_string() } else { self.directory_input.clone() };
                            ui.label(format!("Directory header: {}", dir_label));
                            
                            ui.add_space(8.0);
                            
                            // Server actions
                            ui.horizontal(|ui| {
                                if ui.button("Reconnect").clicked() {
                                    reconnect_requested = true;
                                }
                                
                                if ui.button("Start Server").clicked() {
                                    start_requested = true;
                                }
                                
                                if let Some(info) = &self.server {
                                    if info.owned && ui.button("Stop Server").clicked() {
                                        stop_requested = true;
                                    }
                                }
                            });
                            
                            ui.add_space(8.0);
                            
                            // Save button for server settings
                            if ui.button("Save Server Settings").clicked() {
                                // Update config from input
                                if self.base_url_input.trim().is_empty() {
                                    self.config.server.last_base_url = None;
                                } else {
                                    self.config.server.last_base_url = Some(self.base_url_input.clone());
                                }
                                // Update directory override
                                if self.directory_input.trim().is_empty() {
                                    self.config.server.directory_override = None;
                                } else {
                                    self.config.server.directory_override = Some(self.directory_input.clone());
                                }
                                // Apply to live client
                                if let Some(c) = &mut self.client {
                                    c.directory = self.config.server.directory_override.as_ref().map(|s| std::path::PathBuf::from(s));
                                }
                                self.config.save();
                            }
                        });
                        
                        ui.add_space(16.0);
                        
                        // UI Preferences Section
                        ui.collapsing("UI Preferences", |ui| {
                            ui.heading("Appearance");
                            ui.separator();
                            
                            // Font size preset
                            ui.label("Font Size:");
                            let mut font_changed = false;
                            ui.horizontal(|ui| {
                                if ui.radio_value(&mut self.config.ui.font_size, crate::config::FontSizePreset::Small, "Small").clicked() {
                                    font_changed = true;
                                }
                                if ui.radio_value(&mut self.config.ui.font_size, crate::config::FontSizePreset::Standard, "Standard").clicked() {
                                    font_changed = true;
                                }
                                if ui.radio_value(&mut self.config.ui.font_size, crate::config::FontSizePreset::Large, "Large").clicked() {
                                    font_changed = true;
                                }
                            });
                            
                            // Apply font changes immediately
                            if font_changed {
                                self.config.ui.apply_to_context(ctx);
                                self.config.save();
                            }

                            ui.add_space(8.0);

                            // Base font size (pt)
                            ui.label("Base font (pt):");
                            let resp = ui.add(
                                egui::Slider::new(&mut self.config.ui.base_font_points, 10.0..=24.0)
                                    .text("Base (pt)")
                            );
                            if resp.changed() {
                                self.config.ui.apply_to_context(ctx);
                                self.config.save();
                            }
                            
                            ui.add_space(8.0);
                            
                            // Chat density
                            ui.label("Chat Density:");
                            let mut density_changed = false;
                            ui.horizontal(|ui| {
                                if ui.radio_value(&mut self.config.ui.chat_density, crate::config::ChatDensity::Compact, "Compact").clicked() {
                                    density_changed = true;
                                }
                                if ui.radio_value(&mut self.config.ui.chat_density, crate::config::ChatDensity::Normal, "Normal").clicked() {
                                    density_changed = true;
                                }
                                if ui.radio_value(&mut self.config.ui.chat_density, crate::config::ChatDensity::Comfortable, "Comfortable").clicked() {
                                    density_changed = true;
                                }
                            });
                            
                            // Save density changes
                            if density_changed {
                                self.config.save();
                            }
                        });
                    });
                });
        }
        
        // Execute deferred actions
        if reconnect_requested {
            self.action_reconnect(ctx);
        }
        if start_requested {
            self.action_start_only(ctx);
        }
        if stop_requested {
            if let Some(info) = &self.server {
                if stop_pid(info.pid) {
                    self.server = None;
                    self.server_in_flight = false;
                }
            }
        }

        // Bottom: Input area
        egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
            if !self.tabs.is_empty() {
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    let has_session = tab.session_id.is_some();
                    let session_id = tab.session_id.clone();
                    let blocked = session_id.as_ref().and_then(|sid| self.pending_permissions.iter().find(|p| p.session_id == *sid)).is_some();
                    
                    ui.horizontal(|ui| {
                        // Text input
                        let text_height = ui.text_style_height(&egui::TextStyle::Body) * 3.0;
                        let available_width = ui.available_width() - 100.0; // Leave room for button
                        
                        egui::ScrollArea::vertical()
                            .max_height(text_height * 3.0)
                            .show(ui, |ui| {
                                let _response = ui.add_enabled(
                                    has_session && !blocked,
                                    egui::TextEdit::multiline(&mut tab.input)
                                        .desired_width(available_width)
                                        .desired_rows(3)
                                );
                                
                                // Send on Cmd+Enter (macOS)
                                let send_key = ui.input(|i| {
                                    i.modifiers.command && i.key_pressed(egui::Key::Enter)
                                });
                                
                                let send_enabled = has_session && !blocked && !tab.input.trim().is_empty();
                                if send_key && send_enabled {
                                    if let (Some(client), Some(sid)) = (&self.client, &tab.session_id) {
                                        let text = tab.input.clone();
                                        tab.input.clear();
                                        let c = client.clone();
                                        let sid = sid.clone();
                                        if let Some(rt) = &self.runtime {
                                            rt.spawn(async move {
                                                let _ = c.send_message(&sid, &text).await;
                                            });
                                        }
                                    }
                                }
                            });
                        
                        
                        // Send button and hint
                        ui.vertical(|ui| {
                            let send_enabled = has_session && !blocked && !tab.input.trim().is_empty();
                            if ui.add_enabled(send_enabled, egui::Button::new("Send")).clicked() {
                                if let (Some(client), Some(sid)) = (&self.client, &tab.session_id) {
                                    let text = tab.input.clone();
                                    tab.input.clear();
                                    let c = client.clone();
                                    let sid = sid.clone();
                                    if let Some(rt) = &self.runtime {
                                        rt.spawn(async move {
                                            let _ = c.send_message(&sid, &text).await;
                                        });
                                    }
                                }
                            }
                            if !has_session {
                                ui.small("(Wait...)");
                            } else if blocked {
                                ui.small("Permission pending — respond in tool bubble");
                            } else if self.audio_tx.is_some() {
                                ui.small("⌘+Enter | AltRight: Record");
                            } else {
                                ui.small("⌘+Enter");
                            }
                        });
                    });
                }
            }
        });

        // Center: Chat UI with messages
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.heading("OpenCode EGUI (M4)");
                    if let Some(info) = &self.server {
                        ui.label(format!("| {} (PID {})", info.base_url, info.pid));
                    }
                    // Show current directory context: override > active tab's session directory
                    let current_dir: Option<&str> = if let Some(override_dir) = self.config.server.directory_override.as_deref() {
                        Some(override_dir)
                    } else if let Some(tab) = self.tabs.get(self.active) {
                        tab.directory.as_deref()
                    } else {
                        None
                    };
                    if let Some(dir) = current_dir {
                        ui.label(format!("| Dir: {}", dir));
                    }
                });
                ui.separator();
                
                // Messages area
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.tabs.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label("Click + to create a new session");
                            });
                        } else if let Some(tab) = self.tabs.get(self.active) {
                            let spacing = self.config.ui.chat_density.message_spacing();
                            let (session_id_opt, messages_copy) = (tab.session_id.clone(), tab.messages.clone());
                            drop(tab);
                            for msg in &messages_copy {
                                self.render_message(ui, msg, session_id_opt.as_deref());
                                ui.add_space(spacing);
                            }
                        }
                    });
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Shutdown audio task first to unblock the recv loop
        if let Some(tx) = &self.audio_tx {
            let _ = tx.send(AudioCmd::Shutdown);
        }
        
        // Stop server if owned
        if let Some(s) = &self.server {
            if s.owned {
                let _ = stop_pid(s.pid);
            }
        }
    }
}

async fn run_audio_task(
    audio_rx: mpsc::Receiver<AudioCmd>,
    ui_tx: mpsc::Sender<UiMsg>,
    model_path: std::path::PathBuf,
    egui_ctx: egui::Context,
) {
    use crate::audio::AudioManager;
    
    // Initialize AudioManager
    let mut audio_mgr = match AudioManager::new(&model_path) {
        Ok(mgr) => mgr,
        Err(e) => {
            let _ = ui_tx.send(UiMsg::AudioError(format!("Failed to initialize audio: {}", e)));
            egui_ctx.request_repaint();
            return;
        }
    };
    
    // Listen for audio commands
    loop {
        match audio_rx.recv() {
            Ok(AudioCmd::StartRecording) => {
                match audio_mgr.start_recording() {
                    Ok(_) => {
                        let _ = ui_tx.send(UiMsg::RecordingStarted);
                        egui_ctx.request_repaint();
                    }
                    Err(e) => {
                        let _ = ui_tx.send(UiMsg::AudioError(e.to_string()));
                        egui_ctx.request_repaint();
                    }
                }
            }
            Ok(AudioCmd::StopRecording) => {
                let _ = ui_tx.send(UiMsg::RecordingStopped);
                egui_ctx.request_repaint();
                
                // Stop recording, resample, and transcribe
                // This blocks but runs in dedicated audio task, not UI thread
                match audio_mgr.stop_recording() {
                    Ok(text) => {
                        let _ = ui_tx.send(UiMsg::Transcription(text));
                        egui_ctx.request_repaint();
                    }
                    Err(e) => {
                        let _ = ui_tx.send(UiMsg::AudioError(e.to_string()));
                        egui_ctx.request_repaint();
                    }
                }
            }
            Ok(AudioCmd::Shutdown) => {
                // Clean shutdown - exit task loop
                break;
            }
            Err(_) => break, // Channel closed
        }
    }
}
