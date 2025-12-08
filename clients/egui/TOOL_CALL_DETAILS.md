# Enhanced Tool Call Details Display

## Problem

Currently, the collapsible tool call section in EGUI only shows:
```
🔧 2 tool call(s)
  bash ✅ success
  read ✅ success
```

This is too vague. Users can't see:
- What parameters were passed to each tool
- What the tool returned
- Why a tool failed (if error)
- Execution time or other metadata

## Goal

Provide detailed, transparent information about what the AI is doing via tool calls. This helps with:
- **Debugging** - See exactly what commands ran
- **Trust** - Understand what the AI is accessing
- **Learning** - See how the AI uses tools
- **Auditing** - Track what operations were performed

## Proposed UI

### Collapsed State (Current)
```
🔧 2 tool call(s)
```

### Expanded State (Enhanced)
```
🔧 2 tool call(s)
  
  ┌─ bash ✅ success (1.2s)
  │  Command: ls -la /tmp
  │  Output: total 24
  │          drwxr-xr-x  5 user  wheel  160 Dec  8 03:00 .
  │          drwxr-xr-x  6 root  wheel  192 Dec  8 03:00 ..
  │          ... [Show More]
  
  ┌─ webfetch ⏳ in progress (0.5s elapsed)
  │  URL: https://example.com
  │  Format: markdown
  │  [Attempting ethical fetch: https://example.com]
  │  [Got 403, falling back to stealth mode]
  │  ... waiting for response
```

### Error State
```
┌─ bash ❌ error (0.3s)
│  Command: rm /protected/file.txt
│  Error: Permission denied (exit code 1)
│  Output: rm: cannot remove '/protected/file.txt': Permission denied
```

## Data Structure Changes

### Current ToolCall Struct
```rust
#[derive(Clone)]
struct ToolCall {
    name: String,
    status: String,
}
```

### Enhanced ToolCall Struct
```rust
#[derive(Clone, Debug, serde::Deserialize)]
struct ToolCall {
    id: String,
    name: String,
    status: String,
    
    // Parameters passed to the tool
    #[serde(default)]
    input: serde_json::Value,
    
    // Tool output (success case)
    #[serde(default)]
    output: Option<String>,
    
    // Error message (error case)
    #[serde(default)]
    error: Option<String>,
    
    // Metadata from tool execution
    #[serde(default)]
    metadata: serde_json::Map<String, serde_json::Value>,
    
    // Timestamps
    #[serde(default)]
    started_at: Option<i64>,
    
    #[serde(default)]
    finished_at: Option<i64>,
    
    // Console logs from tool (e.g., webfetch retry logs)
    #[serde(default)]
    logs: Vec<String>,
}
```

## SSE Event Parsing Updates

### Current Parsing (app.rs lines 373-386)
```rust
} else if part_type == Some("tool") {
    let tool_name = part.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown");
    let state = part.get("state").and_then(|v| v.get("status")).and_then(|v| v.as_str()).unwrap_or("unknown");
    if let Some(msg) = tab.messages.last_mut() {
        if let Some(existing) = msg.tool_calls.iter_mut().find(|t| t.name == tool_name) {
            existing.status = state.to_string();
        } else {
            msg.tool_calls.push(ToolCall {
                name: tool_name.to_string(),
                status: state.to_string(),
            });
        }
    }
}
```

### Enhanced Parsing
```rust
} else if part_type == Some("tool") {
    let tool_id = part.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let tool_name = part.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown");
    let state = part.get("state").and_then(|v| v.as_object());
    
    if let Some(msg) = tab.messages.last_mut() {
        if let Some(existing) = msg.tool_calls.iter_mut().find(|t| t.id == tool_id) {
            // Update existing tool call
            if let Some(state_obj) = state {
                if let Some(status) = state_obj.get("status").and_then(|v| v.as_str()) {
                    existing.status = status.to_string();
                }
                if let Some(output) = state_obj.get("output").and_then(|v| v.as_str()) {
                    existing.output = Some(output.to_string());
                }
                if let Some(error) = state_obj.get("error").and_then(|v| v.as_str()) {
                    existing.error = Some(error.to_string());
                }
            }
        } else {
            // Create new tool call
            let input = part.get("input").cloned().unwrap_or(serde_json::Value::Null);
            let status = state.and_then(|s| s.get("status")).and_then(|v| v.as_str()).unwrap_or("unknown");
            
            msg.tool_calls.push(ToolCall {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                status: status.to_string(),
                input,
                output: None,
                error: None,
                metadata: Default::default(),
                started_at: Some(chrono::Utc::now().timestamp_millis()),
                finished_at: None,
                logs: Vec::new(),
            });
        }
    }
}
```

## Rendering Implementation

### Enhanced render_message (app.rs)

Replace the current tool call rendering (lines 464-494) with:

```rust
// Tool calls (collapsible with enhanced details)
if !msg.tool_calls.is_empty() {
    ui.add_space(8.0);
    
    // Check if any tools are still in progress
    let any_in_progress = msg.tool_calls.iter().any(|t| 
        t.status != "success" && t.status != "error" && t.status != "completed"
    );
    
    ui.horizontal(|ui| {
        if any_in_progress {
            ui.spinner();
        }
        
        let header_text = format!("🔧 {} tool call(s)", msg.tool_calls.len());
        egui::CollapsingHeader::new(header_text)
            .id_salt(&msg.message_id)
            .show(ui, |ui| {
                for tool in &msg.tool_calls {
                    render_tool_call_detail(ui, tool);
                }
            });
    });
}

fn render_tool_call_detail(ui: &mut egui::Ui, tool: &ToolCall) {
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());
        
        // Header: name + status + duration
        ui.horizontal(|ui| {
            let status_icon = match tool.status.as_str() {
                "success" | "completed" => "✅",
                "error" => "❌",
                _ => "⏳",
            };
            
            ui.strong(&tool.name);
            ui.add_space(4.0);
            egui_twemoji::EmojiLabel::new(status_icon).show(ui);
            ui.label(&tool.status);
            
            // Duration
            if let (Some(start), Some(end)) = (tool.started_at, tool.finished_at) {
                let duration_ms = end - start;
                ui.label(format!("({:.1}s)", duration_ms as f64 / 1000.0));
            } else if let Some(_start) = tool.started_at {
                ui.label("(in progress)");
            }
        });
        
        ui.add_space(4.0);
        
        // Input parameters (pretty-printed)
        if !tool.input.is_null() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Parameters:").strong());
            });
            
            ui.indent("tool_params", |ui| {
                if let Some(obj) = tool.input.as_object() {
                    for (key, value) in obj {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", key));
                            ui.monospace(format_json_value(value));
                        });
                    }
                } else {
                    ui.monospace(format!("{:#}", tool.input));
                }
            });
        }
        
        ui.add_space(4.0);
        
        // Logs (for tools like webfetch that emit progress)
        if !tool.logs.is_empty() {
            egui::CollapsingHeader::new("Logs")
                .id_salt(format!("{}_logs", tool.id))
                .show(ui, |ui| {
                    for log in &tool.logs {
                        ui.label(egui::RichText::new(log).small().monospace().color(egui::Color32::GRAY));
                    }
                });
        }
        
        ui.add_space(4.0);
        
        // Output or Error
        if let Some(error) = &tool.error {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Error:").strong().color(egui::Color32::RED));
            });
            ui.indent("tool_error", |ui| {
                ui.label(egui::RichText::new(error).color(egui::Color32::LIGHT_RED).monospace());
            });
        } else if let Some(output) = &tool.output {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Output:").strong());
                ui.add_space(4.0);
                if ui.small_button("📋 Copy").clicked() {
                    ui.ctx().copy_text(output.clone());
                }
            });
            
            // Truncate long output
            let preview_len = 200;
            let preview = if output.len() > preview_len {
                format!("{}... [Show More]", &output[..preview_len])
            } else {
                output.clone()
            };
            
            ui.indent("tool_output", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.monospace(&preview);
                    });
            });
        }
    });
    
    ui.add_space(4.0);
}

fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => format!("{}", other),
    }
}
```

## Benefits

✅ **Transparency** - See exactly what AI is doing  
✅ **Debugging** - Quickly identify why tools fail  
✅ **Learning** - Understand how AI uses tools  
✅ **Auditing** - Review what commands/URLs were accessed  
✅ **Trust** - Build confidence by seeing the details

## Implementation Priority

**MEDIUM** - Nice-to-have enhancement that significantly improves UX but not blocking core functionality.

Should be implemented after:
- Permission handling (higher priority - security)
- Session restoration (higher priority - data loss prevention)

## Files to Modify

1. **src/app.rs**:
   - Update `ToolCall` struct (~20 LOC)
   - Enhance SSE parsing for tool events (~40 LOC)
   - Add `render_tool_call_detail()` function (~100 LOC)
   - Add `format_json_value()` helper (~10 LOC)

**Total**: ~170 LOC, low risk (purely additive to existing tool display)
