use crate::dro_dom::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color(pub u8, pub u8, pub u8);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Display {
    Block,
    Inline,
    None,
}

#[derive(Clone, Debug)]
pub struct Style {
    pub display: Display,
    pub color: Color,
    pub bg: Option<Color>,
    pub size: f32,
    pub bold: bool,
    pub margin: [f32; 4], // top right bottom left
    pub pad: [f32; 4],
    pub width: Option<f32>,
}

impl Style {
    fn root() -> Self {
        Style { display: Display::Inline, color: Color(0, 0, 0), bg: None, size: 16.0, bold: false, margin: [0.0; 4], pad: [0.0; 4], width: None }
    }
    fn child_of(p: &Style) -> Self {
        Style { color: p.color, size: p.size, bold: p.bold, ..Style::root() }
    }
}

pub struct Rule {
    tag: Option<String>,
    class: Option<String>,
    id: Option<String>,
    spec: u32,
    decls: Vec<(String, String)>,
}

pub const UA: &str = "html,body,div,p,h1,h2,h3,h4,h5,h6,ul,ol,li,section,article,header,footer,nav,main,form,table,tr,pre,blockquote{display:block}
head,script,style,title,meta,link,img,template,noscript{display:none}
body{margin:8px}p{margin:16px 0 16px 0}ul,ol{margin:16px 0 16px 0;padding:0 0 0 32px}
h1{font-size:32px;font-weight:bold;margin:21px 0 21px 0}h2{font-size:24px;font-weight:bold;margin:19px 0 19px 0}
h3{font-size:19px;font-weight:bold;margin:18px 0 18px 0}h4,h5,h6{font-weight:bold}
b,strong,th{font-weight:bold}a{color:#0000ee}";

pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();
    if let Some(h) = s.strip_prefix('#') {
        if !h.is_ascii() { return None; }
        let v = |i: usize, n: usize| u8::from_str_radix(h.get(i..i + n)?, 16).ok();
        return match h.len() {
            3 => Some(Color(v(0, 1)? * 17, v(1, 1)? * 17, v(2, 1)? * 17)),
            6 => Some(Color(v(0, 2)?, v(2, 2)?, v(4, 2)?)),
            _ => None,
        };
    }
    if let Some(r) = s.strip_prefix("rgb(").or_else(|| s.strip_prefix("rgba(")) {
        let n: Vec<u8> = r.trim_end_matches(')').split(',').take(3).filter_map(|x| x.trim().parse::<f32>().ok()).map(|x| x as u8).collect();
        return if n.len() == 3 { Some(Color(n[0], n[1], n[2])) } else { None };
    }
    Some(match s.as_str() {
        "black" => Color(0, 0, 0),
        "white" => Color(255, 255, 255),
        "red" => Color(255, 0, 0),
        "green" => Color(0, 128, 0),
        "blue" => Color(0, 0, 255),
        "gray" | "grey" => Color(128, 128, 128),
        "yellow" => Color(255, 255, 0),
        "orange" => Color(255, 165, 0),
        "purple" => Color(128, 0, 128),
        _ => return None,
    })
}

fn px(s: &str, em: f32) -> Option<f32> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("px") { return v.trim().parse().ok(); }
    if let Some(v) = s.strip_suffix("rem") { return v.trim().parse::<f32>().ok().map(|x| x * 16.0); }
    if let Some(v) = s.strip_suffix("em") { return v.trim().parse::<f32>().ok().map(|x| x * em); }
    s.parse().ok()
}

fn four(v: &str, em: f32) -> [f32; 4] {
    let p: Vec<f32> = v.split_whitespace().map(|x| px(x, em).unwrap_or(0.0)).collect();
    match p.len() {
        1 => [p[0]; 4],
        2 => [p[0], p[1], p[0], p[1]],
        3 => [p[0], p[1], p[2], p[1]],
        4 => [p[0], p[1], p[2], p[3]],
        _ => [0.0; 4],
    }
}

fn apply(s: &mut Style, prop: &str, v: &str, parent_size: f32) {
    let v = v.trim().trim_end_matches("!important").trim();
    match prop {
        "display" => s.display = match v { "none" => Display::None, "block" | "flex" | "list-item" | "table" => Display::Block, _ => Display::Inline },
        "color" => if let Some(c) = parse_color(v) { s.color = c },
        "background" | "background-color" => s.bg = parse_color(v.split_whitespace().next().unwrap_or("")),
        "font-size" => if let Some(x) = px(v, parent_size) { s.size = x.max(6.0) },
        "font-weight" => s.bold = v == "bold" || v.parse::<u32>().map(|n| n >= 600).unwrap_or(false),
        "margin" => s.margin = four(v, s.size),
        "padding" => s.pad = four(v, s.size),
        "width" => s.width = px(v, s.size),
        _ => {}
    }
}

pub fn parse(css: &str) -> Vec<Rule> {
    let mut clean = String::new();
    let mut rest = css;
    while let Some(i) = rest.find("/*") {
        clean.push_str(&rest[..i]);
        rest = match rest[i..].find("*/") { Some(j) => &rest[i + j + 2..], None => "" };
    }
    clean.push_str(rest);
    let mut rules = vec![];
    for chunk in clean.split('}') {
        let Some((sels, body)) = chunk.split_once('{') else { continue };
        let decls: Vec<(String, String)> = body
            .split(';')
            .filter_map(|d| d.split_once(':'))
            .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
            .collect();
        for sel in sels.split(',') {
            let sel = sel.trim();
            if sel.is_empty() || sel.starts_with('@') || !sel.chars().all(|c| c.is_alphanumeric() || "-_.#".contains(c)) { continue; }
            let (mut tag, mut class, mut id) = (String::new(), None, None);
            let mut mode = 't';
            let mut buf = String::new();
            let flush = |mode: char, buf: &mut String, tag: &mut String, class: &mut Option<String>, id: &mut Option<String>| {
                if buf.is_empty() { return; }
                match mode { 't' => *tag = buf.clone(), '.' => *class = Some(buf.clone()), _ => *id = Some(buf.clone()) }
                buf.clear();
            };
            for c in sel.chars() {
                if c == '.' || c == '#' {
                    flush(mode, &mut buf, &mut tag, &mut class, &mut id);
                    mode = c;
                } else {
                    buf.push(c);
                }
            }
            flush(mode, &mut buf, &mut tag, &mut class, &mut id);
            let spec = id.is_some() as u32 * 100 + class.is_some() as u32 * 10 + !tag.is_empty() as u32;
            rules.push(Rule { tag: if tag.is_empty() { None } else { Some(tag.to_lowercase()) }, class, id, spec, decls: decls.clone() });
        }
    }
    rules
}

fn matches(dom: &Dom, n: NodeId, r: &Rule) -> bool {
    if let Some(t) = &r.tag { if dom.tag(n) != Some(t.as_str()) { return false; } }
    if let Some(i) = &r.id { if dom.attr(n, "id") != Some(i.as_str()) { return false; } }
    if let Some(c) = &r.class {
        if !dom.attr(n, "class").map(|v| v.split_whitespace().any(|x| x == c)).unwrap_or(false) { return false; }
    }
    true
}

pub fn compute(dom: &Dom, rules: &[Rule]) -> Vec<Option<Style>> {
    let mut out: Vec<Option<Style>> = vec![None; dom.nodes.len()];
    let mut stack: Vec<(NodeId, Style, usize)> = dom.nodes[0].children.iter().map(|&c| (c, Style::root(), 0)).collect();
    while let Some((n, parent, depth)) = stack.pop() {
        if dom.tag(n).is_none() || depth > 256 { continue; }
        let mut s = Style::child_of(&parent);
        let mut m: Vec<&Rule> = rules.iter().filter(|r| matches(dom, n, r)).collect();
        m.sort_by_key(|r| r.spec);
        let mut decls: Vec<(&str, &str)> = m.iter().flat_map(|r| r.decls.iter().map(|(k, v)| (k.as_str(), v.as_str()))).collect();
        let inline: Vec<(String, String)> = dom.attr(n, "style").unwrap_or("").split(';').filter_map(|d| d.split_once(':')).map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string())).collect();
        decls.extend(inline.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        for (k, v) in decls.iter().filter(|(k, _)| *k == "font-size") { apply(&mut s, k, v, parent.size); }
        for (k, v) in decls.iter().filter(|(k, _)| *k != "font-size") { apply(&mut s, k, v, parent.size); }
        if dom.attr(n, "hidden").is_some() { s.display = Display::None; }
        for &c in &dom.nodes[n].children { stack.push((c, s.clone(), depth + 1)); }
        out[n] = Some(s);
    }
    out
}
