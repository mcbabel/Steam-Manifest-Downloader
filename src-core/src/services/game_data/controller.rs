use std::sync::Arc;
use std::time::Duration;

use steam_vent::ConnectionTrait;
use steam_vent_proto::steammessages_publishedfile_steamclient::CPublishedFile_GetDetails_Request;

use super::{safe_file_name, API};
use crate::services::steam_pics::ControllerConfig;
use crate::services::steam_session::{call_with_retry, SteamSession};

const CM_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_VDF_BYTES: usize = 4 * 1024 * 1024;

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
        let files: Vec<(String, String)> = convert(&vdf)
            .into_iter()
            .filter(|(_, b)| !b.is_empty())
            .filter_map(|(name, b)| Some((safe_file_name(&format!("{}.txt", name))?, render(&b))))
            .collect();
        if !files.is_empty() {
            return files;
        }
    }
    notes.push("gameData.controllerFailed".into());
    Vec::new()
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
            // menu navigation
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
    fn keeps_duplicate_keys_and_rejects_broken_files() {
        let vdf = parse_vdf("\"a\" { \"k\" \"1\" \"k\" \"2\" }").unwrap();
        assert_eq!(vdf.get("a").unwrap().all("k").count(), 2);
        assert!(parse_vdf("\"a\" { \"k\" }").is_none());
    }
}
