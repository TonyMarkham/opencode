use eframe::egui;
use serde::Deserialize;
use std::sync::{Arc, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

use crate::discovery::process::{ServerInfo, check_health, discover, stop_pid};
use crate::discovery::spawn::spawn_and_wait;
use crate::startup::auth::{AuthSyncState, sync_api_keys_to_server};
use crate::types::agent::AgentInfo;

fn dbg_log(msg: impl AsRef<str>) {
    eprintln!("[egui-debug] {}", msg.as_ref());
}

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

    // Auth sync state
    auth_sync_state: AuthSyncState,

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
    models_config: crate::config::models::ModelsConfig,
    show_settings: bool,
    base_url_input: String,
    directory_input: String,

    // Model discovery UI state
    show_model_discovery: bool,
    discovery_provider: Option<String>,
    discovery_models: Vec<crate::client::providers::DiscoveredModel>,
    discovery_error: Option<String>,
    discovery_in_progress: bool,
    discovery_search: String,

    // Permission handling
    pending_permissions: Vec<PermissionInfo>,

    // Agents
    agents: Vec<AgentInfo>,
    show_subagents: bool,
    agents_pane_collapsed: bool,
    default_agent: String,

    // Markdown rendering
    commonmark_cache: egui_commonmark::CommonMarkCache,
}

#[derive(Default, Clone)]
pub(crate) struct Tab {
    title: String,
    session_id: Option<String>,
    directory: Option<String>,
    messages: Vec<DisplayMessage>,
    active_assistant: Option<String>,
    input: String,
    selected_model: Option<(String, String)>, // (provider, model_id)
    pub(crate) selected_agent: Option<String>,
    cancelled_messages: Vec<String>,
    cancelled_calls: Vec<String>,
    cancelled_after: Option<i64>,
    suppress_incoming: bool,
    last_send_at: i64,
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
    id: String,
    name: String,
    status: String,
    call_id: Option<String>,
    input: serde_json::Value,
    output: Option<String>,
    error: Option<String>,
    metadata: serde_json::Map<String, serde_json::Value>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    logs: Vec<String>,
}

enum UiMsg {
    ServerConnected(ServerInfo),
    ServerError(String),
    SessionCreated {
        tab_idx: usize,
        id: String,
        title: String,
        directory: String,
    },
    GlobalEvent(serde_json::Value),
    #[allow(dead_code)]
    PermissionRequest(PermissionInfo),
    // Auth sync events
    AuthSyncComplete(AuthSyncState),
    // Model discovery events
    ModelsDiscovered(Vec<crate::client::providers::DiscoveredModel>),
    ModelDiscoveryError(String),
    // Agent events
    AgentsLoaded(Vec<AgentInfo>),
    AgentsFailed(String),
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
    #[allow(dead_code)]
    pattern: Option<Vec<String>>,
    #[serde(rename = "sessionID")]
    session_id: String,
    #[serde(rename = "messageID")]
    message_id: String,
    #[serde(rename = "callID")]
    call_id: Option<String>,
    title: String,
    #[allow(dead_code)]
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
        let models_config = crate::config::models::ModelsConfig::load();
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
            auth_sync_state: AuthSyncState::default(),
            audio_tx: None,
            audio_enabled: false,
            recording_state: RecordingState::Idle,
            renaming_tab: None,
            rename_buffer: String::new(),
            rename_text_selected: false,
            config: config.clone(),
            models_config: models_config,
            show_settings: false,
            base_url_input: config.server.last_base_url.unwrap_or_default(),
            directory_input: config.server.directory_override.clone().unwrap_or_default(),
            show_model_discovery: false,
            discovery_provider: None,
            discovery_models: Vec::new(),
            discovery_error: None,
            discovery_in_progress: false,
            discovery_search: String::new(),
            pending_permissions: Vec::new(),
            agents: Vec::new(),
            show_subagents: false,
            agents_pane_collapsed: false,
            default_agent: "build".to_string(),
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
                std::env::current_exe()
                    .ok()
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
        ctx: &egui::Context,
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
        let mut auto_rejects: Vec<(String, String)> = Vec::new();

        if let Some(rx) = &self.ui_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    UiMsg::ServerConnected(info) => {
                        let base = info.base_url.clone();
                        match crate::client::api::OpencodeClient::new(&base) {
                            Ok(mut c) => {
                                if let Some(dir) = &self.config.server.directory_override {
                                    c.directory = Some(std::path::PathBuf::from(dir));
                                }
                                self.client = Some(c)
                            }
                            Err(e) => self.server_error = Some(e.to_string()),
                        }

                        if let Some(rt) = &self.runtime {
                            let tx2 = self.ui_tx.as_ref().unwrap().clone();
                            let egui_ctx = ctx.clone();
                            let base_for_sse = base.clone();
                            rt.spawn(async move {
                                if let Ok(mut rx) =
                                    crate::client::events::subscribe_global(&base_for_sse).await
                                {
                                    while let Some(ev) = rx.recv().await {
                                        let _ = tx2.send(UiMsg::GlobalEvent(ev.payload.clone()));
                                        egui_ctx.request_repaint();
                                    }
                                }
                            });
                        }

                        self.config.server.last_base_url = Some(base.clone());
                        self.base_url_input = base;
                        self.config.save();

                        self.server = Some(info.clone());
                        self.server_error = None;
                        self.server_in_flight = false;

                        if let (Some(rt), Some(tx_agents), Some(client)) =
                            (&self.runtime, &self.ui_tx, &self.client)
                        {
                            let c = client.clone();
                            let tx = tx_agents.clone();
                            let egui_ctx_agents = ctx.clone();
                            rt.spawn(async move {
                                match c.list_agents().await {
                                    Ok(list) => {
                                        let _ = tx.send(UiMsg::AgentsLoaded(list));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(UiMsg::AgentsFailed(e.to_string()));
                                    }
                                }
                                egui_ctx_agents.request_repaint();
                            });
                        }

                        if let Some(rt) = &self.runtime {
                            let tx3 = self.ui_tx.as_ref().unwrap().clone();
                            let egui_ctx2 = ctx.clone();
                            let server_url = info.base_url.clone();
                            rt.spawn(async move {
                                let state = sync_api_keys_to_server(&server_url).await;
                                let _ = tx3.send(UiMsg::AuthSyncComplete(state));
                                egui_ctx2.request_repaint();
                            });
                        }
                    }
                    UiMsg::ServerError(err) => {
                        self.server_error = Some(err);
                        self.server = None;
                        self.server_in_flight = false;
                    }
                    UiMsg::SessionCreated {
                        tab_idx,
                        id,
                        title,
                        directory,
                    } => {
                        if let Some(tab) = self.tabs.get_mut(tab_idx) {
                            tab.title = title;
                            tab.session_id = Some(id);
                            tab.directory = Some(directory);
                        }
                    }
                    UiMsg::GlobalEvent(payload) => {
                        let event_type = payload.get("type").and_then(|v| v.as_str());
                        match event_type {
                            Some("permission.updated") => {
                                if let Some(props) = payload.get("properties") {
                                    if let Ok(info) =
                                        serde_json::from_value::<PermissionInfo>(props.clone())
                                    {
                                        let mut is_cancelled = false;
                                        if let Some(tab) = self
                                            .tabs
                                            .iter()
                                            .find(|t| {
                                                t.session_id.as_deref() == Some(info.session_id.as_str())
                                            })
                                        {
                                            if let Some(call_id) = info.call_id.as_deref() {
                                                if tab.cancelled_calls.iter().any(|c| c == call_id) {
                                                    is_cancelled = true;
                                                }
                                            }

                                            if !is_cancelled {
                                                if tab
                                                    .cancelled_messages
                                                    .iter()
                                                    .any(|m| m == &info.message_id)
                                                {
                                                    is_cancelled = true;
                                                }
                                            }

                                            if !is_cancelled {
                                                if let Some(cutoff) = tab.cancelled_after {
                                                    if info.time.created as i64 <= cutoff {
                                                        is_cancelled = true;
                                                    }
                                                }
                                            }

                                            if !is_cancelled {
                                                if info.time.created as i64 <= tab.last_send_at {
                                                    is_cancelled = true;
                                                }
                                            }

                                            if !is_cancelled {
                                                if tab.suppress_incoming {
                                                    is_cancelled = true;
                                                }
                                            }

                                            if !is_cancelled {
                                                if tab.cancelled_messages.iter().any(|m| m == &info.message_id) {
                                                    is_cancelled = true;
                                                }
                                            }
                                            if !is_cancelled {
                                                if let Some(call_id) = info.call_id.as_deref() {
                                                    if tab.cancelled_calls.iter().any(|c| c == call_id) {
                                                        is_cancelled = true;
                                                    }
                                                }
                                            }

                                            if !is_cancelled {
                                                if tab.suppress_incoming {
                                                    is_cancelled = true;
                                                }
                                            }

                                            // if cancelled and skip_tools_for matched, text is already set in part handler
                                        }

                                        if is_cancelled {
                                            dbg_log(&format!(
                                                "perm auto-reject: sid={} mid={} call={:?} created={}",
                                                info.session_id,
                                                info.message_id,
                                                info.call_id,
                                                info.time.created
                                            ));
                                            auto_rejects.push((
                                                info.session_id.clone(),
                                                info.id.clone(),
                                            ));
                                        } else {
                                            dbg_log(&format!(
                                                "perm queued: sid={} mid={} call={:?} created={}",
                                                info.session_id,
                                                info.message_id,
                                                info.call_id,
                                                info.time.created
                                            ));
                                            self.pending_permissions.push(info);
                                        }
                                    }
                                }
                                continue;
                            }
                            Some("permission.replied") => {
                                if let Some(props) = payload.get("properties") {
                                    let pid = props.get("permissionID").and_then(|v| v.as_str());
                                    let sid = props.get("sessionID").and_then(|v| v.as_str());
                                    let _resp = props
                                        .get("response")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("?");
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
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            tab.messages.push(DisplayMessage {
                                message_id: format!(
                                    "audio_rec_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                ),
                                role: "system".to_string(),
                                text_parts: vec!["🎙 Recording...".to_string()],
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::RecordingStopped => {
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            tab.messages.push(DisplayMessage {
                                message_id: format!(
                                    "audio_proc_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                ),
                                role: "system".to_string(),
                                text_parts: vec!["Processing audio...".to_string()],
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::Transcription(text) => {
                        self.audio_enabled = false;
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            if !tab.input.is_empty() {
                                tab.input.push(' ');
                            }
                            tab.input.push_str(&text);
                            tab.messages.push(DisplayMessage {
                                message_id: format!(
                                    "audio_done_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                ),
                                role: "system".to_string(),
                                text_parts: vec!["✅ Transcription complete".to_string()],
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::PermissionRequest(info) => {
                        self.pending_permissions.push(info);
                    }
                    UiMsg::AuthSyncComplete(state) => {
                        self.auth_sync_state = state;
                    }
                    UiMsg::ModelsDiscovered(models) => {
                        self.discovery_models = models;
                        self.discovery_in_progress = false;
                        self.discovery_error = None;
                    }
                    UiMsg::ModelDiscoveryError(error) => {
                        self.discovery_error = Some(error);
                        self.discovery_in_progress = false;
                    }
                    UiMsg::AgentsLoaded(list) => {
                        self.agents = list;
                        let filtered = Self::filtered_agents(self.show_subagents, &self.agents);
                        let fallback = filtered
                            .first()
                            .map(|agent| agent.name.clone())
                            .unwrap_or_else(|| "build".to_string());
                        self.default_agent = fallback.clone();
                        let default_agent = self.default_agent.clone();
                        for tab in &mut self.tabs {
                            Self::ensure_tab_agent(&default_agent, tab, &filtered);
                        }
                    }
                    UiMsg::AgentsFailed(err) => {
                        dbg_log(&format!("agent fetch failed: {err}"));
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            let msg_id = format!(
                                "agent_err_{}",
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                            );
                            tab.messages.push(DisplayMessage {
                                message_id: msg_id,
                                role: "system".to_string(),
                                text_parts: vec![format!("⚠ Agents: {err}")],
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                    UiMsg::AudioError(err) => {
                        self.audio_enabled = false;
                        self.recording_state = RecordingState::Idle;
                        if let Some(tab) = self.tabs.get_mut(self.active) {
                            tab.messages.push(DisplayMessage {
                                message_id: format!(
                                    "audio_err_{}",
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                ),
                                role: "system".to_string(),
                                text_parts: vec![format!("⚠ Audio: {}", err)],
                                tool_calls: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        for (sid, pid) in auto_rejects {
            self.action_respond_permission(sid, pid, "reject");
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

    pub(crate) fn filtered_agents(show_subagents: bool, agents: &[AgentInfo]) -> Vec<AgentInfo> {
        if show_subagents {
            return agents.to_vec();
        }
        agents
            .iter()
            .filter(|agent| agent.mode.as_deref() != Some("subagent"))
            .cloned()
            .collect()
    }

    pub(crate) fn ensure_tab_agent(default_agent: &str, tab: &mut Tab, filtered: &[AgentInfo]) {
        if let Some(name) = tab.selected_agent.clone() {
            if filtered.iter().any(|agent| agent.name == name) {
                return;
            }
        }
        tab.selected_agent = Some(default_agent.to_string());
    }

    #[cfg(test)]
    pub(crate) fn test_tab_with_agent(agent: Option<String>) -> Tab {
        Tab {
            selected_agent: agent,
            ..Tab::default()
        }
    }

    fn agent_color(hex: &str) -> Option<egui::Color32> {
        let trimmed = hex.strip_prefix('#').unwrap_or(hex);
        if trimmed.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
        let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
        let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
        Some(egui::Color32::from_rgb(r, g, b))
    }

    fn handle_event(tab: &mut Tab, payload: &serde_json::Value) {
        let event_type = payload.get("type").and_then(|v| v.as_str());

        match event_type {
            Some("message.updated") => {
                // New message started - only create if ID doesn't exist
                if let Some(props) = payload.get("properties") {
                    if let Some(info) = props.get("info") {
                        let message_id = info
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let role = info
                            .get("role")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let finish = info.get("finish").and_then(|v| v.as_str());
                        let created = info
                            .get("time")
                            .and_then(|t| t.get("created"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(i64::MAX);

                        if tab.cancelled_messages.iter().any(|m| m == &message_id) {
                            dbg_log(&format!("message.updated drop: msg={} cancelled", message_id));
                            return;
                        }

                        if let Some(cutoff) = tab.cancelled_after {
                            if created <= cutoff {
                                dbg_log(&format!(
                                    "message.updated drop: msg={} created={} cutoff={} (cancelled)",
                                    message_id, created, cutoff
                                ));
                                return;
                            }
                        }
                        if created < tab.last_send_at {
                            dbg_log(&format!(
                                "message.updated drop: msg={} created={} last_send_at={}",
                                message_id, created, tab.last_send_at
                            ));
                            return;
                        }

                        if role == "assistant" {
                            if finish.is_some() {
                                tab.active_assistant = None;
                            } else {
                                tab.active_assistant = Some(message_id.clone());
                            }
                        }
                        if role == "user" {
                            tab.suppress_incoming = false;
                        }

                        if !tab.messages.iter().any(|m| m.message_id == message_id) {
                            dbg_log(&format!(
                                "message.updated accept: msg={} role={} created={} finish={:?}",
                                message_id, role, created, finish
                            ));
                            tab.messages.push(DisplayMessage {
                                message_id: message_id.clone(),
                                role: role.clone(),
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
                        let message_id = part.get("messageID").and_then(|v| v.as_str());
                        if let Some(mid) = message_id {
                            if tab.cancelled_messages.iter().any(|m| m == mid) {
                                dbg_log(&format!(
                                    "part drop: msg={} because cancelled", mid
                                ));
                                return;
                            }
                        } else {
                            dbg_log("part drop: missing message_id");
                            return;
                        }

                        let part_type = part.get("type").and_then(|v| v.as_str());
                        let role = tab
                            .messages
                            .iter()
                            .find(|m| m.message_id.as_str() == message_id.unwrap_or(""))
                            .map(|m| m.role.clone())
                            .unwrap_or_else(|| "unknown".to_string());

                        if let Some(mid) = message_id {
                            if tab.cancelled_messages.iter().any(|m| m == mid) {
                                dbg_log(&format!("part drop: msg={} cancelled", mid));
                                return;
                            }
                        }

                        if tab.suppress_incoming {
                            if role == "assistant" {
                                if part_type == Some("text") {
                                    if let Some(mid) = message_id {
                                        dbg_log(&format!(
                                            "part clearing suppress on assistant text msg={}", mid
                                        ));
                                    }
                                    tab.suppress_incoming = false;
                                } else {
                                    if let Some(mid) = message_id {
                                        dbg_log(&format!(
                                            "part drop: suppress active for assistant msg={} type={:?}",
                                            mid, part_type
                                        ));
                                    }
                                    return;
                                }
                            } else {
                                if let Some(mid) = message_id {
                                    dbg_log(&format!(
                                        "part drop: suppress active for msg={} role={} type={:?}",
                                        mid, role, part_type
                                    ));
                                }
                                return;
                            }
                        }

                        if let Some(call) = part
                            .get("callID")
                            .and_then(|v| v.as_str())
                        {
                            if tab.cancelled_calls.iter().any(|c| c == call) {
                                dbg_log(&format!("part drop: call={} cancelled", call));
                                return;
                            }
                        }

                        if part_type == Some("text") {

                            let text = part.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            if let Some(mid) = message_id {
                                if let Some(msg) = tab.messages.iter_mut().find(|m| m.message_id == mid)
                                {
                                    msg.text_parts.clear();
                                    msg.text_parts.push(text.to_string());
                                }
                            }
                        } else if part_type == Some("tool") {
                            let tool_id =
                                part.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let tool_name = part
                                .get("tool")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let call_id = part
                                .get("callID")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let state = part.get("state");
                            let status = state
                                .and_then(|v| v.get("status"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let output = state
                                .and_then(|v| v.get("output"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let error = state
                                .and_then(|v| v.get("error"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let logs = state
                                .and_then(|v| v.get("logs"))
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|l| l.as_str().map(|s| s.to_string()))
                                        .collect::<Vec<String>>()
                                })
                                .unwrap_or_default();
                            let metadata = state
                                .and_then(|v| v.get("metadata"))
                                .and_then(|v| v.as_object())
                                .cloned()
                                .unwrap_or_default();
                            let started_at = state
                                .and_then(|v| v.get("started_at"))
                                .and_then(|v| v.as_i64());
                            let finished_at = state
                                .and_then(|v| v.get("finished_at"))
                                .and_then(|v| v.as_i64());
                            let input = part
                                .get("input")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null);

                            if let Some(mid) = message_id {
                                if let Some(msg) = tab.messages.iter_mut().find(|m| m.message_id == mid)
                                {
                                    if let Some(call) = call_id.as_deref() {
                                        if tab.cancelled_calls.iter().any(|c| c == call) {
                                            dbg_log(&format!(
                                                "tool part drop: msg={} call={} cancelled",
                                                mid, call
                                            ));
                                            return;
                                        }
                                    }

                                    let existing = msg.tool_calls.iter_mut().find(|t| {
                                        t.id == tool_id
                                            || t.call_id.as_deref() == call_id.as_deref()
                                    });

                                    match existing {
                                        Some(tool) => {
                                            tool.name = tool_name.to_string();
                                            tool.status = status.to_string();
                                            if tool.call_id.is_none() {
                                                tool.call_id = call_id.clone();
                                            }
                                            if !input.is_null() {
                                                tool.input = input.clone();
                                            }
                                            if let Some(val) = output {
                                                tool.output = Some(val);
                                            }
                                            if let Some(val) = error {
                                                tool.error = Some(val);
                                            }
                                            if !metadata.is_empty() {
                                                tool.metadata = metadata.clone();
                                            }
                                            if let Some(start) = started_at {
                                                tool.started_at = Some(start);
                                            }
                                            if let Some(end) = finished_at {
                                                tool.finished_at = Some(end);
                                            }
                                            if !logs.is_empty() {
                                                tool.logs = logs.clone();
                                            }
                                        }
                                        None => {
                                            msg.tool_calls.push(ToolCall {
                                                id: tool_id.to_string(),
                                                name: tool_name.to_string(),
                                                status: status.to_string(),
                                                call_id: call_id,
                                                input: input,
                                                output: output,
                                                error: error,
                                                metadata: metadata,
                                                started_at: started_at,
                                                finished_at: finished_at,
                                                logs: logs,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn cancel_active_response(tab: &mut Tab) {
        if let Some(active_id) = tab.active_assistant.clone() {
            let now_ms = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(dur) => dur.as_millis() as i64,
                Err(_) => 0,
            };

            dbg_log(&format!(
                "stop: active_id={} cancelled_after={}", active_id, now_ms
            ));

            if let Some(msg) = tab.messages.iter_mut().find(|m| m.message_id == active_id) {
                if msg.text_parts.is_empty() {
                    msg.text_parts.push("✖ Cancelled".to_string());
                }

                for tool in &mut msg.tool_calls {
                    if tool.status != "success"
                        && tool.status != "error"
                        && tool.status != "completed"
                        && tool.status != "cancelled"
                    {
                        dbg_log(&format!(
                            "stop: cancelling tool id={} status was {}",
                            tool.id, tool.status
                        ));
                        tool.status = "cancelled".to_string();
                        if tool.finished_at.is_none() {
                            tool.finished_at = Some(now_ms);
                        }
                    }

                    if let Some(call_id) = &tool.call_id {
                        if !tab.cancelled_calls.iter().any(|c| c == call_id) {
                            tab.cancelled_calls.push(call_id.clone());
                        }
                    }
                }
            }

            if !tab
                .cancelled_messages
                .iter()
                .any(|m| m == &active_id)
            {
                tab.cancelled_messages.push(active_id.clone());
            }

            tab.cancelled_after = Some(now_ms);
            tab.suppress_incoming = true;
            tab.active_assistant = None;
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
                                egui_commonmark::CommonMarkViewer::new().show(
                                    ui,
                                    &mut self.commonmark_cache,
                                    &full_text,
                                );
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
                                egui_commonmark::CommonMarkViewer::new().show(
                                    ui,
                                    &mut self.commonmark_cache,
                                    &full_text,
                                );
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

                            let any_in_progress = msg.tool_calls.iter().any(|t| {
                                t.status != "success"
                                    && t.status != "error"
                                    && t.status != "completed"
                                    && t.status != "cancelled"
                            });

                            let has_pending_perm = session_id.is_some()
                                && msg.tool_calls.iter().any(|tool| {
                                    if let Some(call_id) = &tool.call_id {
                                        return self.pending_permissions.iter().any(|p| {
                                            p.session_id == session_id.unwrap()
                                                && p.call_id.as_deref() == Some(call_id.as_str())
                                        });
                                    }
                                    false
                                });

                            ui.horizontal(|ui| {
                                if any_in_progress {
                                    ui.spinner();
                                }
                                let header_text =
                                    format!("🔧 {} tool call(s)", msg.tool_calls.len());
                                egui::CollapsingHeader::new(header_text)
                                    .id_salt(&message_id)
                                    .default_open(has_pending_perm)
                                    .show(ui, |ui| {
                                        for tool in &msg.tool_calls {
                                            self.render_tool_call_detail(ui, tool);

                                            if let (Some(sid), Some(call)) =
                                                (session_id, &tool.call_id)
                                            {
                                                let perm_opt = self
                                                    .pending_permissions
                                                    .iter()
                                                    .find(|p| {
                                                        p.session_id == sid
                                                            && p.call_id.as_deref()
                                                                == Some(call.as_str())
                                                    })
                                                    .cloned();
                                                if let Some(perm) = perm_opt {
                                                    ui.add_space(4.0);
                                                    egui::Frame::default()
                                                        .fill(egui::Color32::from_gray(40))
                                                        .inner_margin(egui::Margin::symmetric(8i8, 6i8))
                                                        .show(ui, |ui| {
                                                            ui.label(format!(
                                                                "Permission required: {}",
                                                                perm.title
                                                            ));
                                                            ui.small(format!(
                                                                "Type: {}",
                                                                perm.perm_type
                                                            ));
                                                            ui.add_space(6.0);
                                                            ui.horizontal(|ui| {
                                                                if ui.button("❌ Reject").clicked()
                                                                {
                                                                    self.action_respond_permission(
                                                                        perm.session_id.clone(),
                                                                        perm.id.clone(),
                                                                        "reject",
                                                                    );
                                                                    if let Some(idx) = self
                                                                        .pending_permissions
                                                                        .iter()
                                                                        .position(|p| {
                                                                            p.id == perm.id
                                                                        })
                                                                    {
                                                                        self.pending_permissions
                                                                            .remove(idx);
                                                                    }
                                                                }
                                                                if ui
                                                                    .button("✅ Allow Once")
                                                                    .clicked()
                                                                {
                                                                    self.action_respond_permission(
                                                                        perm.session_id.clone(),
                                                                        perm.id.clone(),
                                                                        "once",
                                                                    );
                                                                    if let Some(idx) = self
                                                                        .pending_permissions
                                                                        .iter()
                                                                        .position(|p| {
                                                                            p.id == perm.id
                                                                        })
                                                                    {
                                                                        self.pending_permissions
                                                                            .remove(idx);
                                                                    }
                                                                }
                                                                if ui
                                                                    .button("✅ Always Allow")
                                                                    .clicked()
                                                                {
                                                                    self.action_respond_permission(
                                                                        perm.session_id.clone(),
                                                                        perm.id.clone(),
                                                                        "always",
                                                                    );
                                                                    if let Some(idx) = self
                                                                        .pending_permissions
                                                                        .iter()
                                                                        .position(|p| {
                                                                            p.id == perm.id
                                                                        })
                                                                    {
                                                                        self.pending_permissions
                                                                            .remove(idx);
                                                                    }
                                                                }
                                                            });
                                                        });
                                                }
                                            }
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

    fn render_tool_call_detail(&self, ui: &mut egui::Ui, tool: &ToolCall) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                let status_icon = match tool.status.as_str() {
                    "success" | "completed" => "✅",
                    "error" => "❌",
                    _ => "⏳",
                };
                ui.label(&tool.name);
                egui_twemoji::EmojiLabel::new(status_icon).show(ui);
                ui.label(&tool.status);

                if let (Some(start), Some(end)) = (tool.started_at, tool.finished_at) {
                    let duration_ms = end - start;
                    ui.label(format!("({:.1}s)", duration_ms as f64 / 1000.0));
                } else if tool.started_at.is_some() {
                    ui.label("(in progress)");
                }

                if let Some(call_id) = &tool.call_id {
                    ui.small(format!("ID: {call_id}"));
                }
            });

            if let Some(command) = Self::extract_field_as_string(&tool.input, "command") {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Command").strong());
                ui.indent("tool_command", |ui| {
                    ui.horizontal(|ui| {
                        ui.monospace(&command);
                        if ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(command.clone());
                        }
                    });
                });
            }

            if let Some(url) = Self::extract_field_as_string(&tool.input, "url") {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("URL").strong());
                ui.indent("tool_url", |ui| {
                    ui.horizontal(|ui| {
                        ui.monospace(&url);
                        if ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(url.clone());
                        }
                    });
                });
            }

            if !tool.input.is_null() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Parameters").strong());
                ui.indent("tool_params", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            ui.monospace(Self::format_json_value(&tool.input));
                        });
                });
            }

            if !tool.metadata.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Metadata").strong());
                ui.indent("tool_meta", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            ui.monospace(Self::format_json_map(&tool.metadata));
                        });
                });
            }

            if !tool.logs.is_empty() {
                ui.add_space(4.0);
                egui::CollapsingHeader::new("Logs")
                    .id_salt(format!("{}_logs", tool.id))
                    .show(ui, |ui| {
                        for log in &tool.logs {
                            ui.label(egui::RichText::new(log).monospace().small());
                        }
                    });
            }

            if let Some(error) = &tool.error {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Error")
                        .strong()
                        .color(egui::Color32::RED),
                );
                ui.indent("tool_error", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            ui.monospace(error);
                        });
                });
                return;
            }

            if let Some(output) = &tool.output {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Output").strong());
                ui.indent("tool_output", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.monospace(output);
                        });
                    if ui.small_button("Copy output").clicked() {
                        ui.ctx().copy_text(output.clone());
                    }
                });
            }
        });

        ui.add_space(6.0);
    }

    fn extract_field_as_string(value: &serde_json::Value, key: &str) -> Option<String> {
        value
            .as_object()
            .and_then(|obj| obj.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn format_json_value(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => format!("\"{s}\""),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            other => format!("{other}"),
        }
    }

    fn format_json_map(map: &serde_json::Map<String, serde_json::Value>) -> String {
        let mut entries: Vec<String> = map
            .iter()
            .map(|(k, v)| format!("{k}: {}", Self::format_json_value(v)))
            .collect();
        entries.sort();
        entries.join("\n")
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
                        eprintln!(
                            "AltRight pressed but audio task not running (no model configured)"
                        );
                    }
                }
            }
            return;
        }

        for event in &raw_input.events {
            if let egui::Event::Key {
                key,
                pressed,
                repeat,
                ..
            } = event
            {
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
        if self.tabs.is_empty()
            && self.client.is_some()
            && self.runtime.is_some()
            && self.ui_tx.is_some()
        {
            let tab_idx = 0;
            self.tabs.push(Tab {
                title: "(creating…)".to_string(),
                session_id: None,
                directory: None,
                messages: Vec::new(),
                active_assistant: None,
                input: String::new(),
                selected_model: None,
                selected_agent: Some(self.default_agent.clone()),
                cancelled_messages: Vec::new(),
                cancelled_calls: Vec::new(),
                cancelled_after: None,
                suppress_incoming: false,
                last_send_at: 0,
            });
            self.active = 0;

            let txc = self.ui_tx.as_ref().unwrap().clone();
            let c = self.client.as_ref().unwrap().clone();
            let egui_ctx = ctx.clone();
            let rt = self.runtime.as_ref().unwrap().clone();

            rt.spawn(async move {
                match c.create_session(None).await {
                    Ok(info) => {
                        let _ = txc.send(UiMsg::SessionCreated {
                            tab_idx,
                            id: info.id,
                            title: info.title,
                            directory: info.directory,
                        });
                    }
                    Err(e) => {
                        let _ = txc.send(UiMsg::ServerError(e.to_string()));
                    }
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

                let mut model_changed: Option<(usize, Option<(String, String)>)> = None;
                for (i, tab) in self.tabs.iter().enumerate() {
                    let selected = self.active == i;

                    // Group tab label, model selector, and close button together
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
                                    if let Some(mut state) =
                                        egui::TextEdit::load_state(ui.ctx(), id)
                                    {
                                        let text_len = self.rename_buffer.len();
                                        state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(0),
                                                egui::text::CCursor::new(text_len),
                                            ),
                                        ));
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

                            // Model selector dropdown
                            if !self.models_config.get_curated_models().is_empty() {
                                ui.separator();

                                // Display current model or default
                                let current_display =
                                    if let Some((provider, model_id)) = &tab.selected_model {
                                        // Find model name from curated list
                                        self.models_config
                                            .get_curated_models()
                                            .iter()
                                            .find(|m| {
                                                &m.provider == provider && &m.model_id == model_id
                                            })
                                            .map(|m| m.name.clone())
                                            .unwrap_or_else(|| format!("{provider}/{model_id}"))
                                    } else {
                                        match &self.auth_sync_state.status {
                                            crate::startup::auth::AuthSyncStatus::InProgress => {
                                                "⏳".to_string()
                                            }
                                            crate::startup::auth::AuthSyncStatus::Complete => {
                                                "(default)".to_string()
                                            }
                                            crate::startup::auth::AuthSyncStatus::Failed(_) => {
                                                "❌".to_string()
                                            }
                                            _ => "...".to_string(),
                                        }
                                    };

                                egui::ComboBox::from_id_salt(format!("model_selector_{i}"))
                                    .selected_text(current_display)
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        // Option to use default model
                                        if ui
                                            .selectable_label(
                                                tab.selected_model.is_none(),
                                                "(use default)",
                                            )
                                            .clicked()
                                        {
                                            model_changed = Some((i, None));
                                        }

                                        ui.separator();

                                        // Show curated models
                                        for model in self.models_config.get_curated_models() {
                                            let is_selected = tab
                                                .selected_model
                                                .as_ref()
                                                .map(|(p, m)| {
                                                    p == &model.provider && m == &model.model_id
                                                })
                                                .unwrap_or(false);

                                            if ui
                                                .selectable_label(is_selected, &model.name)
                                                .clicked()
                                            {
                                                model_changed = Some((
                                                    i,
                                                    Some((
                                                        model.provider.clone(),
                                                        model.model_id.clone(),
                                                    )),
                                                ));
                                            }
                                        }

                                        ui.separator();

                                        // Link to manage models
                                        if ui.small_button("⚙ Manage Models").clicked() {
                                            self.show_settings = true;
                                            ui.close();
                                        }
                                    });
                            }

                            let agent_display = tab
                                .selected_agent
                                .as_deref()
                                .unwrap_or(self.default_agent.as_str());
                            ui.small(format!("agent: {agent_display}"));

                            if ui.small_button("X").clicked() {
                                to_close = Some(i);
                            }
                        });
                    });
                }

                // Apply deferred actions
                if let Some((idx, model)) = model_changed {
                    if let Some(tab) = self.tabs.get_mut(idx) {
                        tab.selected_model = model;
                    }
                }
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
                        active_assistant: None,
                        input: String::new(),
                        selected_model: None,
                        selected_agent: Some(self.default_agent.clone()),
                        cancelled_messages: Vec::new(),
                        cancelled_calls: Vec::new(),
                        cancelled_after: None,
                        suppress_incoming: false,
                        last_send_at: 0,
                    });
                    self.active = tab_idx;
                    if let (Some(rt), Some(tx), Some(client)) =
                        (&self.runtime, &self.ui_tx, &self.client)
                    {
                        let txc = tx.clone();
                        let c = client.clone();
                        let egui_ctx = ctx.clone();
                        rt.spawn(async move {
                            match c.create_session(None).await {
                                Ok(info) => {
                                    let _ = txc.send(UiMsg::SessionCreated {
                                        tab_idx,
                                        id: info.id,
                                        title: info.title,
                                        directory: info.directory,
                                    });
                                }
                                Err(e) => {
                                    let _ = txc.send(UiMsg::ServerError(e.to_string()));
                                }
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
                            ui.checkbox(
                                &mut self.config.server.auto_start,
                                "Auto-start server on launch",
                            );

                            ui.add_space(8.0);
                            ui.separator();

                            // Discovery diagnostics
                            if let Some(info) = &self.server {
                                ui.label(format!(
                                    "Connected: {} (PID {})",
                                    info.base_url, info.pid
                                ));
                                ui.label(format!("Owned: {}", info.owned));
                            } else {
                                ui.label("Status: Not connected");
                            }
                            let dir_label = if self.directory_input.trim().is_empty() {
                                "(none)".to_string()
                            } else {
                                self.directory_input.clone()
                            };
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
                                    self.config.server.last_base_url =
                                        Some(self.base_url_input.clone());
                                }
                                // Update directory override
                                if self.directory_input.trim().is_empty() {
                                    self.config.server.directory_override = None;
                                } else {
                                    self.config.server.directory_override =
                                        Some(self.directory_input.clone());
                                }
                                // Apply to live client
                                if let Some(c) = &mut self.client {
                                    c.directory = self
                                        .config
                                        .server
                                        .directory_override
                                        .as_ref()
                                        .map(|s| std::path::PathBuf::from(s));
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
                                if ui
                                    .radio_value(
                                        &mut self.config.ui.font_size,
                                        crate::config::FontSizePreset::Small,
                                        "Small",
                                    )
                                    .clicked()
                                {
                                    font_changed = true;
                                }
                                if ui
                                    .radio_value(
                                        &mut self.config.ui.font_size,
                                        crate::config::FontSizePreset::Standard,
                                        "Standard",
                                    )
                                    .clicked()
                                {
                                    font_changed = true;
                                }
                                if ui
                                    .radio_value(
                                        &mut self.config.ui.font_size,
                                        crate::config::FontSizePreset::Large,
                                        "Large",
                                    )
                                    .clicked()
                                {
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
                                egui::Slider::new(
                                    &mut self.config.ui.base_font_points,
                                    10.0..=24.0,
                                )
                                .text("Base (pt)"),
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
                                if ui
                                    .radio_value(
                                        &mut self.config.ui.chat_density,
                                        crate::config::ChatDensity::Compact,
                                        "Compact",
                                    )
                                    .clicked()
                                {
                                    density_changed = true;
                                }
                                if ui
                                    .radio_value(
                                        &mut self.config.ui.chat_density,
                                        crate::config::ChatDensity::Normal,
                                        "Normal",
                                    )
                                    .clicked()
                                {
                                    density_changed = true;
                                }
                                if ui
                                    .radio_value(
                                        &mut self.config.ui.chat_density,
                                        crate::config::ChatDensity::Comfortable,
                                        "Comfortable",
                                    )
                                    .clicked()
                                {
                                    density_changed = true;
                                }
                            });

                            // Save density changes
                            if density_changed {
                                self.config.save();
                            }

                            ui.add_space(8.0);

                            let prev_subagents = self.show_subagents;
                            ui.checkbox(
                                &mut self.show_subagents,
                                "Show subagents in agent list",
                            );
                            if self.show_subagents != prev_subagents {
                                let filtered =
                                    Self::filtered_agents(self.show_subagents, &self.agents);
                                let fallback = filtered
                                    .first()
                                    .map(|agent| agent.name.clone())
                                    .unwrap_or_else(|| "build".to_string());
                                self.default_agent = fallback.clone();
                                let default_agent = self.default_agent.clone();
                                for tab in &mut self.tabs {
                                    Self::ensure_tab_agent(&default_agent, tab, &filtered);
                                }
                            }
                        });

                        ui.add_space(16.0);

                        // Models Section
                        ui.collapsing("Models", |ui| {
                            ui.heading("Curated Models");
                            ui.separator();

                            ui.label("Your curated models:");
                            ui.add_space(8.0);

                            // Display curated models with remove buttons
                            let mut model_to_remove: Option<(String, String)> = None;
                            for model in self.models_config.get_curated_models() {
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "{}  ({}/{})",
                                        model.name, model.provider, model.model_id
                                    ));
                                    if ui.small_button("✖").clicked() {
                                        model_to_remove =
                                            Some((model.provider.clone(), model.model_id.clone()));
                                    }
                                });
                            }

                            // Remove model if requested (deferred to avoid borrow issues)
                            if let Some((provider, model_id)) = model_to_remove {
                                self.models_config
                                    .remove_curated_model(&provider, &model_id);
                                let _ = self.models_config.save();
                            }

                            ui.add_space(8.0);

                            // Add Model button
                            if ui.button("+ Add Model").clicked() {
                                self.show_model_discovery = true;
                            }

                            ui.add_space(16.0);
                            ui.separator();

                            // Default model selector
                            ui.label("Default model for new tabs:");
                            let current_default = self.models_config.models.default_model.clone();
                            let curated_models = self.models_config.get_curated_models().to_vec();
                            egui::ComboBox::from_id_salt("default_model_selector")
                                .selected_text(&current_default)
                                .show_ui(ui, |ui| {
                                    for model in &curated_models {
                                        let model_id =
                                            format!("{}/{}", model.provider, model.model_id);
                                        if ui
                                            .selectable_value(
                                                &mut self.models_config.models.default_model,
                                                model_id.clone(),
                                                &model.name,
                                            )
                                            .clicked()
                                        {
                                            let _ = self.models_config.save();
                                        }
                                    }
                                });

                            ui.add_space(8.0);

                            // Auth sync status display
                            ui.separator();
                            ui.label("API Key Sync Status:");
                            match &self.auth_sync_state.status {
                                crate::startup::auth::AuthSyncStatus::NotStarted => {
                                    ui.label("⏸ Not started");
                                }
                                crate::startup::auth::AuthSyncStatus::InProgress => {
                                    ui.label("⏳ Syncing keys to server...");
                                }
                                crate::startup::auth::AuthSyncStatus::Complete => {
                                    ui.label("✅ Complete");
                                    if !self.auth_sync_state.synced_providers.is_empty() {
                                        ui.small(format!(
                                            "Synced: {}",
                                            self.auth_sync_state.synced_providers.join(", ")
                                        ));
                                    }
                                    if !self.auth_sync_state.failed_providers.is_empty() {
                                        for (provider, error) in
                                            &self.auth_sync_state.failed_providers
                                        {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 100, 100),
                                                format!("❌ {provider}: {error}"),
                                            );
                                        }
                                    }
                                }
                                crate::startup::auth::AuthSyncStatus::Failed(err) => {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 100, 100),
                                        format!("❌ Failed: {err}"),
                                    );
                                }
                            }
                        });
                    });
                });
        }

        // Model Discovery Window
        if self.show_model_discovery {
            let mut close_requested = false;
            egui::Window::new("Add Model")
                .default_width(500.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Step 1: Select Provider (if not selected yet)
                        if self.discovery_provider.is_none() {
                            ui.heading("Select a provider:");
                            ui.separator();
                            ui.add_space(8.0);

                            for provider in self.models_config.get_providers() {
                                if ui.button(&provider.display_name).clicked() {
                                    self.discovery_provider = Some(provider.name.clone());
                                    self.discovery_in_progress = true;
                                    self.discovery_error = None;
                                    self.discovery_models.clear();

                                    // Spawn async task to discover models
                                    if let (Some(rt), Some(tx)) = (&self.runtime, &self.ui_tx) {
                                        let provider_config = provider.clone();
                                        let tx = tx.clone();
                                        let egui_ctx = ctx.clone();

                                        // Get API key from environment
                                        if let Ok(api_key) =
                                            std::env::var(&provider_config.api_key_env)
                                        {
                                            rt.spawn(async move {
                                                let provider_client =
                                                    crate::client::providers::ProviderClient::new()
                                                        .unwrap();
                                                match provider_client
                                                    .discover_models(&provider_config, &api_key)
                                                    .await
                                                {
                                                    Ok(models) => {
                                                        let _ = tx
                                                            .send(UiMsg::ModelsDiscovered(models));
                                                    }
                                                    Err(e) => {
                                                        let _ =
                                                            tx.send(UiMsg::ModelDiscoveryError(
                                                                e.to_string(),
                                                            ));
                                                    }
                                                }
                                                egui_ctx.request_repaint();
                                            });
                                        } else {
                                            self.discovery_error = Some(format!(
                                                "API key not found: {}",
                                                provider_config.api_key_env
                                            ));
                                            self.discovery_in_progress = false;
                                        }
                                    }
                                }
                            }

                            ui.add_space(8.0);
                            if ui.button("Cancel").clicked() {
                                close_requested = true;
                            }
                        }
                        // Step 2: Display discovered models
                        else {
                            let provider_name = self.discovery_provider.as_ref().unwrap().clone();
                            ui.heading(format!("Add Model from {provider_name}"));
                            ui.separator();
                            ui.add_space(8.0);

                            // Show loading spinner or error
                            if self.discovery_in_progress {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Discovering models...");
                                });
                            } else if let Some(error) = &self.discovery_error {
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 100, 100),
                                    format!("Error: {error}"),
                                );
                            } else if !self.discovery_models.is_empty() {
                                // Search box
                                ui.horizontal(|ui| {
                                    ui.label("Search:");
                                    ui.text_edit_singleline(&mut self.discovery_search);
                                });
                                ui.add_space(8.0);

                                // Filter models by search
                                let search_lower = self.discovery_search.to_lowercase();
                                let filtered_models: Vec<_> = self
                                    .discovery_models
                                    .iter()
                                    .filter(|m| {
                                        search_lower.is_empty()
                                            || m.id.to_lowercase().contains(&search_lower)
                                            || m.name.to_lowercase().contains(&search_lower)
                                    })
                                    .cloned()
                                    .collect();

                                ui.label(format!("{} models found:", filtered_models.len()));
                                ui.separator();

                                let mut model_to_add: Option<
                                    crate::client::providers::DiscoveredModel,
                                > = None;
                                egui::ScrollArea::vertical()
                                    .max_height(300.0)
                                    .show(ui, |ui| {
                                        for model in &filtered_models {
                                            ui.horizontal(|ui| {
                                                if ui.button("+").clicked() {
                                                    model_to_add = Some(model.clone());
                                                }
                                                ui.label(format!("{} ({})", model.name, model.id));
                                            });
                                        }
                                    });

                                // Add model if requested (deferred to avoid borrow issues)
                                if let Some(model) = model_to_add {
                                    let curated_model = crate::config::models::CuratedModel::new(
                                        model.name.clone(),
                                        provider_name.clone(),
                                        model.id.clone(),
                                    );
                                    self.models_config.add_curated_model(curated_model);
                                    let _ = self.models_config.save();

                                    // Show success and close
                                    close_requested = true;
                                    self.discovery_provider = None;
                                    self.discovery_models.clear();
                                    self.discovery_search.clear();
                                }
                            }

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Back").clicked() {
                                    self.discovery_provider = None;
                                    self.discovery_models.clear();
                                    self.discovery_error = None;
                                    self.discovery_in_progress = false;
                                    self.discovery_search.clear();
                                }
                                if ui.button("Cancel").clicked() {
                                    close_requested = true;
                                    self.discovery_provider = None;
                                    self.discovery_models.clear();
                                    self.discovery_error = None;
                                    self.discovery_in_progress = false;
                                    self.discovery_search.clear();
                                }
                            });
                        }
                    });

                    // Close button
                    ui.horizontal(|ui| {
                        if ui.button("✖ Close").clicked() {
                            close_requested = true;
                        }
                    });
                });

            if close_requested {
                self.show_model_discovery = false;
            }
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

        let filtered_agents = Self::filtered_agents(self.show_subagents, &self.agents);
        let has_agents = !self.agents.is_empty();
        if !self.tabs.is_empty() && has_agents {
            egui::SidePanel::left("agents_pane").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Agents");
                    if ui
                        .small_button(if self.agents_pane_collapsed { "▸" } else { "▾" })
                        .clicked()
                    {
                        self.agents_pane_collapsed = !self.agents_pane_collapsed;
                    }
                });

                if self.agents_pane_collapsed {
                    return;
                }

                if filtered_agents.is_empty() {
                    ui.label("No primary agents. Enable subagents in Settings.");
                    return;
                }

                if let Some(tab) = self.tabs.get_mut(self.active) {
                    for agent in &filtered_agents {
                        let is_selected = tab
                            .selected_agent
                            .as_deref()
                            .map(|name| name == agent.name.as_str())
                            .unwrap_or(false);
                        let is_sub = agent.mode.as_deref() == Some("subagent");
                        let mut label_text = egui::RichText::new(&agent.name);
                        if is_sub {
                            label_text = label_text.color(egui::Color32::from_gray(150));
                        }
                        ui.horizontal(|ui| {
                            let response = ui.selectable_label(is_selected, label_text);
                            if let Some(color_hex) = &agent.color {
                                if let Some(color) = Self::agent_color(color_hex) {
                                    ui.colored_label(color, "⬤");
                                }
                            }
                            if agent.built_in {
                                ui.small("built-in");
                            }
                            if is_sub {
                                ui.small("subagent");
                            }
                            if response.clicked() {
                                tab.selected_agent = Some(agent.name.clone());
                                dbg_log(&format!("agent selected: {}", agent.name));
                            }
                        });
                    }
                }
            });
        }

        // Bottom: Input area
        egui::TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
            if !self.tabs.is_empty() {
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    let has_session = tab.session_id.is_some();
                    let session_id = tab.session_id.clone();
                    let blocked = session_id
                        .as_ref()
                        .and_then(|sid| {
                            self.pending_permissions
                                .iter()
                                .find(|p| p.session_id == *sid)
                        })
                        .is_some();
                    let streaming = tab.active_assistant.is_some();

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
                                        .desired_rows(3),
                                );

                                // Send on Cmd+Enter (macOS)
                                let send_key = ui.input(|i| {
                                    i.modifiers.command && i.key_pressed(egui::Key::Enter)
                                });

                                let send_enabled = has_session
                                    && !blocked
                                    && !streaming
                                    && !tab.input.trim().is_empty();
                                if send_key && send_enabled {
                                    if let (Some(client), Some(sid)) =
                                        (&self.client, &tab.session_id)
                                    {
                                        tab.suppress_incoming = false;
                                        tab.last_send_at = match SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                        {
                                            Ok(dur) => dur.as_millis() as i64,
                                            Err(_) => 0,
                                        };

                                        let text = tab.input.clone();
                                        let model = tab.selected_model.clone();
                                        let agent = tab
                                            .selected_agent
                                            .clone()
                                            .unwrap_or_else(|| self.default_agent.clone());
                                        tab.input.clear();
                                        let c = client.clone();
                                        let sid = sid.clone();
                                        if let Some(rt) = &self.runtime {
                                            rt.spawn(async move {
                                                let _ = c
                                                    .send_message(&sid, &text, model, Some(agent))
                                                    .await;
                                            });
                                        }
                                    }
                                }

                                if streaming {
                                    ui.add_enabled(false, egui::Button::new("Send"));
                                }
                            });

                        // Send / Stop controls and hint
                        ui.vertical(|ui| {
                            if streaming {
                                if ui
                                    .add_enabled(has_session, egui::Button::new("Stop"))
                                    .clicked()
                                {
                                    let sid_clone = tab.session_id.clone();

                                    if let (Some(client), Some(sid)) = (&self.client, sid_clone) {
                                        Self::cancel_active_response(tab);
                                        let c = client.clone();
                                        let sid_for_abort = sid.clone();
                                        if let Some(rt) = &self.runtime {
                                            rt.spawn(async move {
                                                let _ = c.abort_session(&sid_for_abort).await;
                                                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                                                let _ = c.abort_session(&sid_for_abort).await;
                                            });
                                        }
                                    }
                                }
                            }

                            let send_enabled = has_session
                                && !blocked
                                && !streaming
                                && !tab.input.trim().is_empty();
                            if ui
                                .add_enabled(send_enabled, egui::Button::new("Send"))
                                .clicked()
                            {
                                if let (Some(client), Some(sid)) = (&self.client, &tab.session_id) {
                                    tab.suppress_incoming = false;
                                    tab.last_send_at = match SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                    {
                                        Ok(dur) => dur.as_millis() as i64,
                                        Err(_) => 0,
                                    };

                                     let text = tab.input.clone();
                                     let model = tab.selected_model.clone();
                                     let agent = tab
                                         .selected_agent
                                         .clone()
                                         .unwrap_or_else(|| self.default_agent.clone());
                                     tab.input.clear();
                                     let c = client.clone();
                                     let sid = sid.clone();
                                     if let Some(rt) = &self.runtime {
                                         rt.spawn(async move {
                                             let _ = c
                                                 .send_message(&sid, &text, model, Some(agent))
                                                 .await;
                                         });
                                     }

                                }
                            }
                            if !has_session {
                                ui.small("(Wait...)");
                            }
                            if has_session && blocked {
                                ui.small("Permission pending — respond in tool bubble");
                            }
                            if has_session && streaming {
                                ui.small("Stop to cancel response");
                            }
                            if has_session && !blocked && !streaming {
                                if self.audio_tx.is_some() {
                                    ui.small("⌘+Enter | AltRight: Record");
                                } else {
                                    ui.small("⌘+Enter");
                                }
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
                    let current_dir: Option<&str> = if let Some(override_dir) =
                        self.config.server.directory_override.as_deref()
                    {
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
                            let (session_id_opt, messages_copy) =
                                (tab.session_id.clone(), tab.messages.clone());
                            let _ = tab;
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
            let _ = ui_tx.send(UiMsg::AudioError(format!(
                "Failed to initialize audio: {}",
                e
            )));
            egui_ctx.request_repaint();
            return;
        }
    };

    // Listen for audio commands
    loop {
        match audio_rx.recv() {
            Ok(AudioCmd::StartRecording) => match audio_mgr.start_recording() {
                Ok(_) => {
                    let _ = ui_tx.send(UiMsg::RecordingStarted);
                    egui_ctx.request_repaint();
                }
                Err(e) => {
                    let _ = ui_tx.send(UiMsg::AudioError(e.to_string()));
                    egui_ctx.request_repaint();
                }
            },
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
