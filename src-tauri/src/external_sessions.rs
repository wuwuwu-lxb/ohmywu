use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ohmywu_domain::chrono_now;
use ohmywu_session::{SessionManager, SessionSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRouteInput {
    pub provider: String,
    pub account_id: String,
    pub chat_type: String,
    pub peer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRouteStateView {
    pub route_key: String,
    pub provider: String,
    pub account_id: String,
    pub chat_type: String,
    pub peer_id: String,
    pub peer_name: Option<String>,
    pub active_session_id: Option<String>,
    pub active_session_name: Option<String>,
    pub session_ids: Vec<String>,
    pub sessions: Vec<ExternalSessionSummaryView>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSessionSummaryView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ExternalSessionIndex {
    #[serde(default)]
    routes: Vec<ExternalRouteBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExternalRouteBinding {
    key: ExternalRouteKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peer_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_session_id: Option<String>,
    #[serde(default)]
    session_ids: Vec<String>,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ExternalRouteKey {
    provider: String,
    account_id: String,
    chat_type: String,
    peer_id: String,
}

pub struct ExternalSessionStore {
    file_path: PathBuf,
    write_lock: Mutex<()>,
}

impl ExternalSessionStore {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        let dir = data_dir.join("integrations");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Create integrations dir: {}", e))?;
        let file_path = dir.join("external_sessions.json");
        if !file_path.exists() {
            save_index(&file_path, &ExternalSessionIndex::default())?;
        }
        Ok(Self {
            file_path,
            write_lock: Mutex::new(()),
        })
    }

    pub fn resolve_or_create_session(
        &self,
        route: ExternalRouteInput,
        session_manager: &SessionManager,
    ) -> Result<(SessionSummary, bool), String> {
        let route = normalize_route(route);
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut index = load_index(&self.file_path)?;
        let binding = ensure_binding(&mut index, &route);
        refresh_binding(binding, &route, session_manager);

        if let Some(session_id) = binding.active_session_id.clone()
            && let Some(summary) = session_manager.get_session_summary(&session_id)?
        {
            binding.updated_at = chrono_now();
            save_index(&self.file_path, &index)?;
            return Ok((summary, false));
        }

        let summary = create_routed_session(session_manager, &route, None)?;
        binding.active_session_id = Some(summary.id.clone());
        if !binding.session_ids.iter().any(|item| item == &summary.id) {
            binding.session_ids.push(summary.id.clone());
        }
        binding.updated_at = chrono_now();
        save_index(&self.file_path, &index)?;
        Ok((summary, true))
    }

    pub fn create_new_session(
        &self,
        route: ExternalRouteInput,
        name: Option<&str>,
        session_manager: &SessionManager,
    ) -> Result<SessionSummary, String> {
        let route = normalize_route(route);
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut index = load_index(&self.file_path)?;
        let binding = ensure_binding(&mut index, &route);
        refresh_binding(binding, &route, session_manager);

        let summary = create_routed_session(session_manager, &route, name)?;
        binding.active_session_id = Some(summary.id.clone());
        binding.session_ids.retain(|item| session_manager.session_exists(item));
        if !binding.session_ids.iter().any(|item| item == &summary.id) {
            binding.session_ids.push(summary.id.clone());
        }
        binding.updated_at = chrono_now();
        save_index(&self.file_path, &index)?;
        Ok(summary)
    }

    pub fn remove_session_references(
        &self,
        session_id: &str,
        session_manager: &SessionManager,
    ) -> Result<(), String> {
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut index = load_index(&self.file_path)?;
        let mut changed = false;

        for binding in &mut index.routes {
            let original_len = binding.session_ids.len();
            binding
                .session_ids
                .retain(|item| item != session_id && session_manager.session_exists(item));
            if binding.session_ids.len() != original_len {
                changed = true;
            }
            if binding.active_session_id.as_deref() == Some(session_id) {
                binding.active_session_id = binding.session_ids.last().cloned();
                changed = true;
            }
        }

        let original_route_len = index.routes.len();
        index
            .routes
            .retain(|binding| !binding.session_ids.is_empty() || binding.active_session_id.is_some());
        if index.routes.len() != original_route_len {
            changed = true;
        }

        if changed {
            save_index(&self.file_path, &index)?;
        }
        Ok(())
    }

    pub fn get_route_state(
        &self,
        route: ExternalRouteInput,
        session_manager: &SessionManager,
    ) -> Result<Option<ExternalRouteStateView>, String> {
        let route = normalize_route(route);
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut index = load_index(&self.file_path)?;
        let view = {
            let Some(binding) = index.routes.iter_mut().find(|item| item.key == route.key()) else {
                return Ok(None);
            };
            refresh_binding(binding, &route, session_manager);
            binding_to_view(binding, session_manager)?
        };
        save_index(&self.file_path, &index)?;
        Ok(Some(view))
    }

    pub fn list_route_states(
        &self,
        provider_filter: Option<&str>,
        session_manager: &SessionManager,
    ) -> Result<Vec<ExternalRouteStateView>, String> {
        let provider_filter = provider_filter
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let mut index = load_index(&self.file_path)?;
        let mut views = Vec::new();

        for binding in &mut index.routes {
            if let Some(filter) = &provider_filter
                && binding.key.provider != *filter
            {
                continue;
            }
            let route = ExternalRouteInput {
                provider: binding.key.provider.clone(),
                account_id: binding.key.account_id.clone(),
                chat_type: binding.key.chat_type.clone(),
                peer_id: binding.key.peer_id.clone(),
                peer_name: binding.peer_name.clone(),
            };
            refresh_binding(binding, &route, session_manager);
            views.push(binding_to_view(binding, session_manager)?);
        }

        save_index(&self.file_path, &index)?;
        views.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(views)
    }
}

fn refresh_binding(
    binding: &mut ExternalRouteBinding,
    route: &ExternalRouteInput,
    session_manager: &SessionManager,
) {
    binding.peer_name = route.peer_name.clone();
    binding
        .session_ids
        .retain(|item| session_manager.session_exists(item));
    if binding
        .active_session_id
        .as_deref()
        .is_some_and(|id| !session_manager.session_exists(id))
    {
        binding.active_session_id = binding.session_ids.last().cloned();
    }
}

fn ensure_binding<'a>(
    index: &'a mut ExternalSessionIndex,
    route: &ExternalRouteInput,
) -> &'a mut ExternalRouteBinding {
    let key = route.key();
    if let Some(position) = index.routes.iter().position(|item| item.key == key) {
        return &mut index.routes[position];
    }
    index.routes.push(ExternalRouteBinding {
        key,
        peer_name: route.peer_name.clone(),
        active_session_id: None,
        session_ids: Vec::new(),
        updated_at: chrono_now(),
    });
    index
        .routes
        .last_mut()
        .expect("binding must exist after push")
}

fn create_routed_session(
    session_manager: &SessionManager,
    route: &ExternalRouteInput,
    name: Option<&str>,
) -> Result<SessionSummary, String> {
    let trimmed_name = name.map(str::trim).filter(|item| !item.is_empty());
    let session_name = trimmed_name
        .map(ToString::to_string)
        .unwrap_or_else(|| default_session_name(route));
    session_manager.create_session(&session_name, Some(&default_session_category(route)))
}

fn default_session_name(route: &ExternalRouteInput) -> String {
    let provider = provider_label(&route.provider);
    let peer = route
        .peer_name
        .as_deref()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| short_peer_label(&route.peer_id));
    let timestamp = chrono_now()
        .get(11..16)
        .map(str::to_string)
        .unwrap_or_else(|| "00:00".into());
    format!("{} · {} · {}", provider, peer, timestamp)
}

fn default_session_category(route: &ExternalRouteInput) -> String {
    format!("{} · {}", provider_label(&route.provider), chat_type_label(&route.chat_type))
}

fn short_peer_label(peer_id: &str) -> String {
    let trimmed = peer_id.trim();
    if trimmed.is_empty() {
        return "未命名会话".into();
    }
    let chars = trimmed.chars().count();
    if chars <= 10 {
        return trimmed.to_string();
    }
    let start = trimmed.chars().take(4).collect::<String>();
    let end = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{}…{}", start, end)
}

fn binding_to_view(
    binding: &ExternalRouteBinding,
    session_manager: &SessionManager,
) -> Result<ExternalRouteStateView, String> {
    let mut sessions = Vec::new();
    for session_id in &binding.session_ids {
        if let Some(summary) = session_manager.get_session_summary(session_id)? {
            sessions.push(ExternalSessionSummaryView {
                id: summary.id,
                name: summary.name,
                category: summary.category,
                updated_at: summary.updated_at,
            });
        }
    }
    let active_session_name = binding
        .active_session_id
        .as_deref()
        .and_then(|session_id| sessions.iter().find(|item| item.id == session_id))
        .map(|item| item.name.clone());

    Ok(ExternalRouteStateView {
        route_key: format!(
            "{}:{}:{}:{}",
            binding.key.provider, binding.key.account_id, binding.key.chat_type, binding.key.peer_id
        ),
        provider: binding.key.provider.clone(),
        account_id: binding.key.account_id.clone(),
        chat_type: binding.key.chat_type.clone(),
        peer_id: binding.key.peer_id.clone(),
        peer_name: binding.peer_name.clone(),
        active_session_id: binding.active_session_id.clone(),
        active_session_name,
        session_ids: binding.session_ids.clone(),
        sessions,
        updated_at: binding.updated_at.clone(),
    })
}

fn provider_label(provider: &str) -> &str {
    match provider {
        "wechat" | "wechat_openclaw" | "openclaw-weixin" | "wechat-clawbot" => "微信",
        _ => "外部",
    }
}

fn chat_type_label(chat_type: &str) -> &str {
    match chat_type {
        "group" => "群聊",
        _ => "私聊",
    }
}

fn normalize_route(mut route: ExternalRouteInput) -> ExternalRouteInput {
    route.provider = route.provider.trim().to_ascii_lowercase();
    route.account_id = route.account_id.trim().to_string();
    route.chat_type = if route.chat_type.trim().eq_ignore_ascii_case("group") {
        "group".into()
    } else {
        "dm".into()
    };
    route.peer_id = route.peer_id.trim().to_string();
    route.peer_name = route
        .peer_name
        .take()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty());
    route
}

impl ExternalRouteInput {
    fn key(&self) -> ExternalRouteKey {
        ExternalRouteKey {
            provider: self.provider.clone(),
            account_id: self.account_id.clone(),
            chat_type: self.chat_type.clone(),
            peer_id: self.peer_id.clone(),
        }
    }
}

fn load_index(path: &Path) -> Result<ExternalSessionIndex, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Read external session index: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse external session index: {}", e))
}

fn save_index(path: &Path, index: &ExternalSessionIndex) -> Result<(), String> {
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Serialize external session index: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("Write external session index: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Rename external session index: {}", e))?;
    Ok(())
}
