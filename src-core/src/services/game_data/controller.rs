use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use steam_vent::ConnectionTrait;
use steam_vent_proto::steammessages_publishedfile_steamclient::CPublishedFile_GetDetails_Request;

use super::{safe_file_name, API};
use crate::services::steam_pics::{self, ControllerConfig};
use crate::services::steam_session::{call_with_retry, SteamSession};

const CM_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_VDF_BYTES: usize = 4 * 1024 * 1024;
const MAX_GLYPH_BYTES: usize = 512 * 1024;
const MAX_GLYPHS: usize = 512;
const GLYPHS_API: &str = "https://api.github.com/repos/Detanup01/gbe_fork/contents/post_build/steam_settings.EXAMPLE/controller.EXAMPLE/glyphs";
const GLYPHS_REF: &str = "dev";
const GLYPH_CACHE: &str = "controller_glyphs";
const CONTROLLER_DIR: &str = "controller";
const GLYPH_DIR: &str = "glyphs";
const USER_AGENT: &str = "SteamManifestDownloader";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControllerInfo {
    pub layout: bool,
    pub support: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ControllerResult {
    pub source: String,
    pub sets: usize,
    pub glyphs: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Text(String),
    Obj(Vec<(String, Node)>),
}

impl Node {
    fn entries(&self) -> &[(String, Node)] {
        match self {
            Node::Obj(list) => list,
            Node::Text(_) => &[],
        }
    }

    fn get(&self, key: &str) -> Option<&Node> {
        self.entries().iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    fn all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.entries().iter().filter(move |(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    fn text(&self) -> Option<&str> {
        match self {
            Node::Text(s) => Some(s),
            Node::Obj(_) => None,
        }
    }

    fn str_of(&self, key: &str) -> &str {
        self.get(key).and_then(Node::text).unwrap_or("")
    }
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

enum Token {
    Str(String),
    Open,
    Close,
}

impl<'a> Lexer<'a> {
    fn next(&mut self) -> Option<Token> {
        loop {
            let c = *self.chars.peek()?;
            if c.is_whitespace() {
                self.chars.next();
            } else if c == '/' {
                self.chars.next();
                if self.chars.peek() == Some(&'/') {
                    for n in self.chars.by_ref() {
                        if n == '\n' {
                            break;
                        }
                    }
                }
            } else if c == '[' {
                for n in self.chars.by_ref() {
                    if n == ']' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        match self.chars.next()? {
            '{' => Some(Token::Open),
            '}' => Some(Token::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = self.chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match self.chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => break,
                        },
                        other => s.push(other),
                    }
                }
                Some(Token::Str(s))
            }
            first => {
                let mut s = String::from(first);
                while let Some(&c) = self.chars.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    self.chars.next();
                }
                Some(Token::Str(s))
            }
        }
    }
}

fn parse_obj(lex: &mut Lexer, depth: usize) -> Option<Vec<(String, Node)>> {
    if depth > 64 {
        return None;
    }
    let mut out = Vec::new();
    loop {
        let key = match lex.next() {
            None | Some(Token::Close) => return Some(out),
            Some(Token::Open) => return None,
            Some(Token::Str(k)) => k,
        };
        match lex.next()? {
            Token::Str(v) => out.push((key, Node::Text(v))),
            Token::Open => out.push((key, Node::Obj(parse_obj(lex, depth + 1)?))),
            Token::Close => return None,
        }
    }
}

pub fn parse_vdf(text: &str) -> Option<Node> {
    let mut lex = Lexer { chars: text.trim_start_matches('\u{feff}').chars().peekable() };
    parse_obj(&mut lex, 0).map(Node::Obj)
}

type Bindings = Vec<(String, Vec<String>)>;

fn digital_key(input: &str) -> Option<&'static str> {
    Some(match input.to_ascii_lowercase().as_str() {
        "button_a" => "A",
        "button_b" => "B",
        "button_x" => "X",
        "button_y" => "Y",
        "dpad_north" => "DUP",
        "dpad_south" => "DDOWN",
        "dpad_east" => "DRIGHT",
        "dpad_west" => "DLEFT",
        "button_escape" => "START",
        "button_menu" => "BACK",
        "left_bumper" => "LBUMPER",
        "right_bumper" => "RBUMPER",
        "button_back_left" => "A",
        "button_back_right" => "X",
        "button_back_left_upper" => "B",
        "button_back_right_upper" => "Y",
        _ => return None,
    })
}

fn stick_dpad_key(left: bool, input: &str) -> Option<&'static str> {
    Some(match (left, input.to_ascii_lowercase().as_str()) {
        (true, "dpad_north") => "DLJOYUP",
        (true, "dpad_south") => "DLJOYDOWN",
        (true, "dpad_west") => "DLJOYLEFT",
        (true, "dpad_east") => "DLJOYRIGHT",
        (true, "click") => "LSTICK",
        (false, "dpad_north") => "DRJOYUP",
        (false, "dpad_south") => "DRJOYDOWN",
        (false, "dpad_west") => "DRJOYLEFT",
        (false, "dpad_east") => "DRJOYRIGHT",
        (false, "click") => "RSTICK",
        _ => return None,
    })
}

fn slot<'a>(bindings: &'a mut Bindings, action: &str) -> Option<&'a mut Vec<String>> {
    bindings.iter_mut().find(|(a, _)| a == action).map(|(_, v)| v)
}

fn push_binding(bindings: &mut Bindings, action: &str, binding: &str) {
    match slot(bindings, action) {
        Some(list) => {
            if !list.iter().any(|b| b == binding) {
                list.push(binding.to_string());
            }
        }
        None => bindings.push((action.to_string(), vec![binding.to_string()])),
    }
}

fn push_analog(bindings: &mut Bindings, action: &str, binding: &str, mode: &str) {
    let with_mode = format!("{}={}", binding, mode);
    match slot(bindings, action) {
        Some(list) => {
            if !list.iter().any(|b| b == binding || *b == with_mode) {
                list.insert(0, binding.to_string());
            }
        }
        None => bindings.push((action.to_string(), vec![with_mode])),
    }
}

fn add_inputs(group: &Node, bindings: &mut Bindings, force: Option<&str>, keymap: &dyn Fn(&str) -> Option<&'static str>) {
    let Some(inputs) = group.get("inputs") else { return };
    for (input, activators) in inputs.entries() {
        for (_, activator) in activators.entries() {
            for (_, press) in activator.entries() {
                for (_, section) in press.entries() {
                    for (k, v) in section.entries() {
                        if !k.eq_ignore_ascii_case("binding") {
                            continue;
                        }
                        let Some(text) = v.text() else { continue };
                        let parts: Vec<&str> = text.split_whitespace().collect();
                        let action = match parts.first().map(|p| p.to_ascii_lowercase()) {
                            Some(p) if p == "game_action" && parts.len() > 2 => parts[2],
                            Some(p) if p == "xinput_button" && parts.len() > 1 => parts[1],
                            _ => continue,
                        };
                        let action = action.strip_suffix(',').unwrap_or(action);
                        let binding = force.or_else(|| keymap(input));
                        if let Some(b) = binding {
                            push_binding(bindings, action, b);
                        }
                    }
                }
            }
        }
    }
}

pub fn convert(vdf: &Node) -> Vec<(String, Bindings)> {
    let Some(mappings) = vdf.get("controller_mappings") else {
        return Vec::new();
    };
    let mut groups: Vec<(&str, &Node)> = Vec::new();
    for g in mappings.all("group") {
        let id = g.str_of("id");
        groups.retain(|(gid, _)| *gid != id);
        groups.push((id, g));
    }
    let group_by_id = |id: &str| groups.iter().find(|(gid, _)| *gid == id).map(|(_, g)| *g);
    let action_sets: Vec<&str> = mappings
        .all("actions")
        .flat_map(|a| a.entries().iter().map(|(k, _)| k.as_str()))
        .collect();
    let mut out: Vec<(String, Bindings)> = Vec::new();
    for preset in mappings.all("preset") {
        let name = preset.str_of("name");
        if name.is_empty() || (!action_sets.contains(&name) && !name.eq_ignore_ascii_case("default")) {
            continue;
        }
        let mut bindings: Bindings = Vec::new();
        let Some(sources) = preset.get("group_source_bindings") else { continue };
        for (id, value) in sources.entries() {
            let Some(text) = value.text() else { continue };
            let parts: Vec<String> = text.split_whitespace().map(|p| p.to_ascii_lowercase()).collect();
            if parts.len() < 2 || parts[1] != "active" {
                continue;
            }
            let Some(group) = group_by_id(id) else { continue };
            let source = parts[0].as_str();
            let mode = group.str_of("mode").to_ascii_lowercase();
            if matches!(source, "switch" | "button_diamond" | "dpad") {
                add_inputs(group, &mut bindings, None, &digital_key);
            }
            if matches!(source, "left_trigger" | "right_trigger") && mode == "trigger" {
                let left = source == "left_trigger";
                for (k, v) in group.entries() {
                    if k.eq_ignore_ascii_case("gameactions") {
                        if let Some(action) = v.get(name).and_then(Node::text) {
                            push_analog(&mut bindings, action, if left { "LTRIGGER" } else { "RTRIGGER" }, "trigger");
                        }
                    } else if k.eq_ignore_ascii_case("inputs") {
                        add_inputs(group, &mut bindings, Some(if left { "DLTRIGGER" } else { "DRTRIGGER" }), &digital_key);
                    }
                }
            }
            if matches!(source, "joystick" | "right_joystick" | "dpad") {
                if mode == "joystick_move" {
                    let analog = match source {
                        "joystick" => "LJOY",
                        "right_joystick" => "RJOY",
                        _ => "DPAD",
                    };
                    for (k, v) in group.entries() {
                        if k.eq_ignore_ascii_case("gameactions") {
                            if let Some(action) = v.get(name).and_then(Node::text) {
                                push_analog(&mut bindings, action, analog, "joystick_move");
                            }
                        } else if k.eq_ignore_ascii_case("inputs") {
                            let click = if source == "joystick" { "LSTICK" } else { "RSTICK" };
                            add_inputs(group, &mut bindings, Some(click), &digital_key);
                        }
                    }
                } else if mode == "dpad" && source != "dpad" {
                    let left = source == "joystick";
                    add_inputs(group, &mut bindings, None, &move |i| stick_dpad_key(left, i));
                }
            }
        }
        out.retain(|(n, _)| n != name);
        out.push((name.to_string(), bindings));
    }
    out
}

pub fn render(bindings: &Bindings) -> String {
    bindings
        .iter()
        .map(|(action, list)| format!("{}={}\n", action, list.join(",")))
        .collect()
}

async fn file_url_from_cm(session: &Arc<SteamSession>, file_id: u64) -> Option<String> {
    call_with_retry(session, 1, CM_TIMEOUT, "Steam workshop query timed out", |conn| async move {
        let req = CPublishedFile_GetDetails_Request {
            publishedfileids: vec![file_id],
            short_description: Some(true),
            language: Some(0),
            ..Default::default()
        };
        conn.service_method(req)
            .await
            .map(|r| {
                r.publishedfiledetails
                    .first()
                    .and_then(|d| d.file_url.clone())
                    .unwrap_or_default()
            })
            .map_err(|e| e.to_string())
    })
    .await
    .ok()
    .filter(|u| u.starts_with("http"))
}

async fn file_url_from_web(http: &reqwest::Client, file_id: u64) -> Option<String> {
    let resp = http
        .post(format!("{}/ISteamRemoteStorage/GetPublishedFileDetails/v1/", API))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("itemcount=1&publishedfileids%5B0%5D={}", file_id))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    let body: serde_json::Value = resp.json().await.ok()?;
    body.pointer("/response/publishedfiledetails/0/file_url")
        .and_then(|u| u.as_str())
        .map(str::to_string)
        .filter(|u| u.starts_with("http"))
}

async fn download(http: &reqwest::Client, url: &str) -> Option<String> {
    let resp = http.get(url).send().await.ok()?.error_for_status().ok()?;
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() > MAX_VDF_BYTES {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

pub async fn fetch(
    http: &reqwest::Client,
    session: &Arc<SteamSession>,
    configs: &[ControllerConfig],
    notes: &mut Vec<String>,
) -> Vec<(String, String)> {
    if configs.is_empty() {
        return Vec::new();
    }
    for cfg in configs {
        let url = match file_url_from_cm(session, cfg.file_id).await {
            Some(u) => Some(u),
            None => file_url_from_web(http, cfg.file_id).await,
        };
        let Some(text) = (match url {
            Some(u) => download(http, &u).await,
            None => None,
        }) else {
            continue;
        };
        let Some(vdf) = parse_vdf(&text) else { continue };
        let files = sets_from_vdf(&vdf);
        if !files.is_empty() {
            return files;
        }
    }
    notes.push("gameData.controllerFailed".into());
    Vec::new()
}

fn sets_from_vdf(vdf: &Node) -> Vec<(String, String)> {
    convert(vdf)
        .into_iter()
        .filter(|(_, b)| !b.is_empty())
        .filter_map(|(name, b)| Some((safe_file_name(&format!("{}.txt", name))?, render(&b))))
        .collect()
}

pub fn sets_from_file(text: &str) -> Result<Vec<(String, String)>, String> {
    let not_layout = || "This file is not a Steam Input layout".to_string();
    let vdf = parse_vdf(text).ok_or_else(not_layout)?;
    if vdf.get("controller_mappings").is_none() {
        return Err(not_layout());
    }
    let sets = sets_from_vdf(&vdf);
    if sets.is_empty() {
        return Err(not_layout());
    }
    Ok(sets)
}

pub async fn info(session: Arc<SteamSession>, app_id: u32) -> Result<ControllerInfo, String> {
    let app = steam_pics::fetch_app_info(session, app_id).await?;
    let support = steam_pics::common_value(&app, "controller_support")
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty() && s != "none");
    Ok(ControllerInfo {
        layout: !steam_pics::controller_configs(&app).is_empty(),
        support,
    })
}

fn glyph_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files
}

fn find_release_glyphs(root: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    dirs.reverse();
    for dir in dirs {
        let candidate = dir.join("controller.EXAMPLE").join(GLYPH_DIR);
        if dir.file_name().is_some_and(|n| n == "steam_settings.EXAMPLE") && !glyph_files(&candidate).is_empty() {
            return Some(candidate);
        }
        if let Some(found) = find_release_glyphs(&dir, depth - 1) {
            return Some(found);
        }
    }
    None
}

async fn download_glyphs(http: &reqwest::Client, into: &Path) -> Result<usize, String> {
    let list: serde_json::Value = http
        .get(GLYPHS_API)
        .query(&[("ref", GLYPHS_REF)])
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| e.without_url().to_string())?
        .error_for_status()
        .map_err(|e| e.without_url().to_string())?
        .json()
        .await
        .map_err(|e| e.without_url().to_string())?;
    let items = list.as_array().ok_or("unexpected glyph listing")?;
    let tmp = into.with_extension("part");
    let _ = tokio::fs::remove_dir_all(&tmp).await;
    tokio::fs::create_dir_all(&tmp).await.map_err(|e| e.to_string())?;
    let mut count = 0;
    for item in items.iter().take(MAX_GLYPHS) {
        if item.get("type").and_then(|t| t.as_str()) != Some("file") {
            continue;
        }
        let (Some(name), Some(url)) = (
            item.get("name").and_then(|n| n.as_str()).and_then(safe_file_name),
            item.get("download_url").and_then(|u| u.as_str()),
        ) else {
            continue;
        };
        let Ok(resp) = http.get(url).header("User-Agent", USER_AGENT).send().await else { continue };
        let Ok(resp) = resp.error_for_status() else { continue };
        let Ok(bytes) = resp.bytes().await else { continue };
        if bytes.len() > MAX_GLYPH_BYTES {
            continue;
        }
        if tokio::fs::write(tmp.join(&name), &bytes).await.is_ok() {
            count += 1;
        }
    }
    if count == 0 {
        let _ = tokio::fs::remove_dir_all(&tmp).await;
        return Err("no glyphs downloaded".into());
    }
    let _ = tokio::fs::remove_dir_all(into).await;
    tokio::fs::rename(&tmp, into).await.map_err(|e| e.to_string())?;
    Ok(count)
}

pub async fn glyph_source(http: &reqwest::Client, gbe_cache: &Path) -> Option<PathBuf> {
    if let Some(dir) = find_release_glyphs(gbe_cache, 5) {
        return Some(dir);
    }
    let cached = gbe_cache.join(GLYPH_CACHE);
    if !glyph_files(&cached).is_empty() {
        return Some(cached);
    }
    download_glyphs(http, &cached).await.ok().map(|_| cached)
}

pub async fn write(
    http: &reqwest::Client,
    session: Arc<SteamSession>,
    targets: &[String],
    app_id: &str,
    layout_file: Option<&str>,
    gbe_cache: &Path,
) -> Result<ControllerResult, String> {
    let dirs = super::settings_dirs(targets);
    if dirs.is_empty() {
        return Err("no patched library to write game data next to".into());
    }
    let mut result = ControllerResult { source: "none".into(), ..Default::default() };
    let sets = match layout_file.map(str::trim).filter(|f| !f.is_empty()) {
        Some(file) => {
            let bytes = tokio::fs::read(file).await.map_err(|e| format!("read layout file: {}", e))?;
            if bytes.len() > MAX_VDF_BYTES {
                return Err("This file is not a Steam Input layout".into());
            }
            result.source = "file".into();
            sets_from_file(&String::from_utf8_lossy(&bytes))?
        }
        None => {
            let app_num: u32 = app_id.trim().parse().map_err(|_| format!("Invalid app id '{}'", app_id.trim()))?;
            match steam_pics::fetch_app_info(session.clone(), app_num).await {
                Ok(app) => {
                    let configs = steam_pics::controller_configs(&app);
                    if configs.is_empty() {
                        result.notes.push("controller.noLayout".into());
                        Vec::new()
                    } else {
                        result.source = "steam".into();
                        fetch(http, &session, &configs, &mut result.notes).await
                    }
                }
                Err(_) => {
                    result.notes.push("controller.appInfoFailed".into());
                    Vec::new()
                }
            }
        }
    };
    if sets.is_empty() {
        return Ok(result);
    }
    let glyphs = match glyph_source(http, gbe_cache).await {
        Some(dir) => glyph_files(&dir),
        None => {
            result.notes.push("controller.glyphsMissing".into());
            Vec::new()
        }
    };
    let mut copied_ok = true;
    for dir in &dirs {
        let cdir = dir.join(CONTROLLER_DIR);
        tokio::fs::create_dir_all(&cdir)
            .await
            .map_err(|e| format!("create controller folder: {}", e))?;
        for old in glyph_files(&cdir) {
            if old.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt")) {
                let _ = tokio::fs::remove_file(old).await;
            }
        }
        for (name, text) in &sets {
            tokio::fs::write(cdir.join(name), text)
                .await
                .map_err(|e| format!("write {}: {}", name, e))?;
        }
        if glyphs.is_empty() {
            continue;
        }
        let gdir = cdir.join(GLYPH_DIR);
        let _ = tokio::fs::create_dir_all(&gdir).await;
        let mut copied = 0;
        for g in &glyphs {
            if let Some(name) = g.file_name() {
                if tokio::fs::copy(g, gdir.join(name)).await.is_ok() {
                    copied += 1;
                }
            }
        }
        if copied < glyphs.len() {
            copied_ok = false;
        }
        result.glyphs = result.glyphs.max(copied);
    }
    if !copied_ok {
        result.notes.push("controller.glyphsFailed".into());
    }
    result.sets = sets.len();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
"controller_mappings"
{
    "version"   "3"
    "actions"
    {
        "InGame" { "title" "#Set_Ingame" }
        "Menu" { "title" "#Set_Menu" }
    }
    "group"
    {
        "id" "0"
        "mode" "four_buttons"
        "inputs"
        {
            "button_a" { "activators" { "Full_Press" { "bindings" { "binding" "game_action InGame Jump, #Jump, , " } } } }
            "button_b" { "activators" { "Full_Press" { "bindings" { "binding" "game_action InGame Crouch, #Crouch" } } } }
        }
    }
    "group"
    {
        "id" "1"
        "mode" "joystick_move"
        "gameactions" { "InGame" "Move" }
        "inputs"
        {
            "click" { "activators" { "Full_Press" { "bindings" { "binding" "game_action InGame Sprint," } } } }
        }
    }
    "group"
    {
        "id" "2"
        "mode" "trigger"
        "gameactions" { "InGame" "Fire" }
    }
    "group"
    {
        "id" "3"
        "mode" "dpad"
        "inputs"
        {
            "dpad_north" { "activators" { "Full_Press" { "bindings" { "binding" "game_action Menu Up," } } } }
        }
    }
    "preset"
    {
        "id" "0"
        "name" "InGame"
        "group_source_bindings"
        {
            "0" "button_diamond active"
            "1" "joystick active"
            "2" "right_trigger active"
            "3" "joystick inactive"
        }
    }
    "preset"
    {
        "id" "1"
        "name" "Menu"
        "group_source_bindings" { "3" "right_joystick active" }
    }
}
"##;

    #[test]
    fn converts_a_steam_input_layout() {
        let vdf = parse_vdf(SAMPLE).unwrap();
        let sets = convert(&vdf);
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].0, "InGame");
        assert_eq!(
            render(&sets[0].1),
            "Jump=A\nCrouch=B\nMove=LJOY=joystick_move\nSprint=LSTICK\nFire=RTRIGGER=trigger\n"
        );
        assert_eq!(sets[1].0, "Menu");
        assert_eq!(render(&sets[1].1), "Up=DRJOYUP\n");
    }

    #[test]
    fn a_layout_file_must_be_a_steam_input_layout() {
        assert_eq!(sets_from_file(SAMPLE).unwrap().len(), 2);
        assert!(sets_from_file("\"a\" { \"k\" \"1\" }").is_err());
        assert!(sets_from_file("not a vdf {").is_err());
    }

    #[test]
    fn finds_the_glyphs_of_a_cached_release() {
        let root = std::env::temp_dir().join(format!("smd-glyphs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let glyphs = root.join("release-1").join("release").join("steam_settings.EXAMPLE").join("controller.EXAMPLE").join("glyphs");
        std::fs::create_dir_all(&glyphs).unwrap();
        assert_eq!(find_release_glyphs(&root, 5), None);
        std::fs::write(glyphs.join("button_a.png"), b"png").unwrap();
        assert_eq!(find_release_glyphs(&root, 5), Some(glyphs));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn keeps_duplicate_keys_and_rejects_broken_files() {
        let vdf = parse_vdf("\"a\" { \"k\" \"1\" \"k\" \"2\" }").unwrap();
        assert_eq!(vdf.get("a").unwrap().all("k").count(), 2);
        assert!(parse_vdf("\"a\" { \"k\" }").is_none());
    }
}
