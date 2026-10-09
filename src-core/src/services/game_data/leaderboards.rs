#[derive(Debug, Clone, PartialEq)]
pub struct Leaderboard {
    pub name: String,
    pub sort_method: u8,
    pub display_type: u8,
}

fn unescape(raw: &str) -> String {
    let s = raw.trim();
    let s = s
        .strip_prefix("<![CDATA[")
        .and_then(|r| r.strip_suffix("]]>"))
        .map(str::to_string)
        .unwrap_or_else(|| {
            s.replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
                .replace("&#39;", "'")
                .replace("&amp;", "&")
        });
    s.trim().to_string()
}

fn tag<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{}>", name);
    let close = format!("</{}>", name);
    let start = block.find(&open)? + open.len();
    let len = block[start..].find(&close)?;
    Some(&block[start..start + len])
}

pub fn parse(xml: &str) -> Vec<Leaderboard> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<leaderboard>") {
        let body = &rest[start + "<leaderboard>".len()..];
        let Some(end) = body.find("</leaderboard>") else { break };
        let block = &body[..end];
        rest = &body[end..];
        let Some(name) = tag(block, "name").map(unescape) else { continue };
        if name.is_empty() || name.contains(['=', '\n', '\r']) {
            continue;
        }
        let num = |t: &str| tag(block, t).map(unescape).and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
        out.push(Leaderboard {
            name,
            sort_method: num("sortmethod").min(2),
            display_type: num("displaytype").min(3),
        });
    }
    out.dedup_by(|a, b| a.name == b.name);
    out
}

pub fn render(list: &[Leaderboard]) -> String {
    list.iter()
        .map(|l| format!("{}={}={}\n", l.name, l.sort_method, l.display_type))
        .collect()
}

pub async fn fetch(http: &reqwest::Client, app_id: &str, notes: &mut Vec<String>) -> Vec<Leaderboard> {
    let url = format!("https://steamcommunity.com/stats/{}/leaderboards/?xml=1", app_id);
    let body = match http.get(&url).send().await.and_then(|r| r.error_for_status()) {
        Ok(resp) => resp.text().await.ok(),
        Err(_) => None,
    };
    match body {
        Some(text) if text.contains("<response") => parse(&text),
        _ => {
            notes.push("gameData.leaderboardsFailed".into());
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_community_leaderboards() {
        let xml = "<?xml version=\"1.0\"?><response><appID>620</appID><leaderboardCount>3</leaderboardCount>\
            <leaderboard><lbid>1</lbid><name><![CDATA[best_time]]></name><display_name><![CDATA[Best]]></display_name><sortmethod>1</sortmethod><displaytype>3</displaytype></leaderboard>\
            <leaderboard><name>kills &amp; deaths</name><sortmethod>2</sortmethod><displaytype>1</displaytype></leaderboard>\
            <leaderboard><name>bad=name</name><sortmethod>1</sortmethod><displaytype>1</displaytype></leaderboard></response>";
        let list = parse(xml);
        assert_eq!(list.len(), 2);
        assert_eq!(render(&list), "best_time=1=3\nkills & deaths=2=1\n");
        assert!(parse("<response></response>").is_empty());
    }
}
