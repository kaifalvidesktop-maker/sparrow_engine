use crate::dro_css::{self, Style};
use crate::dro_dom::*;
use crate::dro_js::JsRuntime;
use crate::dro_layout::{self, Layout, Paint};
use crate::{dro_html, dro_net, dro_paint};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub const CRASH_MSG: &str = "Page crashed, reload?";

pub struct SparrowEngine {
    url: String,
    title: String,
    dom: Dom,
    styles: Vec<Option<Style>>,
    layout: Option<Layout>,
    vp: (u32, u32),
    scroll: f32,
    js: Option<JsRuntime>,
    crashed: bool,
}

impl SparrowEngine {
    pub fn new() -> Self {
        SparrowEngine { url: "about:blank".into(), title: String::new(), dom: Dom::new(), styles: vec![], layout: None, vp: (1024, 768), scroll: 0.0, js: None, crashed: false }
    }

    /// Every public method goes through here: a panic never reaches the shell.
    fn guard<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        if self.crashed { return Err(CRASH_MSG.into()); }
        match catch_unwind(AssertUnwindSafe(|| f(self))) {
            Ok(r) => r,
            Err(_) => { self.crashed = true; Err(CRASH_MSG.into()) }
        }
    }

    pub fn is_crashed(&self) -> bool { self.crashed }
    /// Call after the shell shows "Page crashed, reload?" and user clicks reload.
    pub fn recover(&mut self) { *self = SparrowEngine { vp: self.vp, url: self.url.clone(), ..SparrowEngine::new() }; }
    pub fn url(&self) -> &str { &self.url }
    pub fn title(&self) -> &str { &self.title }
    pub fn viewport(&self) -> (u32, u32) { self.vp }
    pub fn set_viewport(&mut self, w: u32, h: u32) { self.vp = (w.max(1), h.max(1)); self.layout = None; }
    pub fn reload(&mut self) -> Result<(), String> {
        if self.crashed { self.recover(); }
        let u = self.url.clone();
        self.load_url(&u)
    }

    pub fn load_url(&mut self, input: &str) -> Result<(), String> {
        let input = input.to_string();
        self.guard(move |e| {
            if input.trim().is_empty() || input == "about:blank" { return e.install("", "about:blank"); }
            let r = dro_net::fetch(input.trim())?;
            let ct = r.content_type.to_lowercase();
            let text = String::from_utf8_lossy(&r.body).to_string();
            let html = if ct.contains("html") || ct.is_empty() { text }
                else if ct.starts_with("text/") { format!("<pre>{}</pre>", text.replace('&', "&amp;").replace('<', "&lt;")) }
                else { return Err(format!("Cannot display {ct}")); };
            e.install(&html, &r.url)
        })
    }

    fn install(&mut self, html: &str, base: &str) -> Result<(), String> {
        self.url = base.to_string();
        self.dom = dro_html::parse(html)?;
        self.scroll = 0.0;
        self.layout = None;
        self.title = self.dom.find_tag("title").map(|t| self.dom.text_content(t).trim().to_string()).unwrap_or_default();
        let mut rules = dro_css::parse(dro_css::UA);
        let mut scripts = vec![];
        for n in self.dom.descendants(0) {
            match self.dom.tag(n) {
                Some("style") => rules.extend(dro_css::parse(&self.dom.text_content(n))),
                Some("script") if self.dom.attr(n, "src").is_none() => scripts.push(self.dom.text_content(n)),
                Some("link") if self.dom.attr(n, "rel") == Some("stylesheet") => {
                    if let Some(u) = self.dom.attr(n, "href").and_then(|h| dro_net::resolve(base, h)) {
                        if let Ok(r) = dro_net::fetch(&u) { rules.extend(dro_css::parse(&String::from_utf8_lossy(&r.body))); }
                    }
                }
                _ => {}
            }
        }
        self.styles = dro_css::compute(&self.dom, &rules);
        let js = JsRuntime::new()?;
        let js = self.js.insert(js);
        for s in scripts { let _ = js.eval(&s); } // script errors never break the page
        Ok(())
    }

    fn ensure_layout(&mut self) -> Result<(), String> {
        if self.layout.is_none() {
            self.layout = Some(dro_layout::compute(&self.dom, &self.styles, self.vp.0 as f32)?);
        }
        Ok(())
    }

    /// Returns RGBA pixels (viewport width * height * 4).
    pub fn render(&mut self) -> Result<Vec<u8>, String> {
        self.guard(|e| {
            e.ensure_layout()?;
            let l = e.layout.as_ref().ok_or("no layout")?;
            dro_paint::paint(l, e.vp.0, e.vp.1, e.scroll)
        })
    }

    pub fn eval_js(&mut self, code: &str) -> Result<String, String> {
        let code = code.to_string();
        self.guard(move |e| {
            if e.js.is_none() { e.js = Some(JsRuntime::new()?); }
            e.js.as_mut().ok_or("js unavailable")?.eval(&code)
        })
    }

    /// console.log / alert output since last call
    pub fn take_logs(&mut self) -> Vec<String> {
        self.js.as_ref().map(|j| j.take_logs()).unwrap_or_default()
    }

    pub fn scroll_by(&mut self, dy: f32) -> Result<f32, String> {
        self.guard(move |e| {
            e.ensure_layout()?;
            let max = (e.layout.as_ref().map(|l| l.height).unwrap_or(0.0) - e.vp.1 as f32).max(0.0);
            e.scroll = (e.scroll + dy).clamp(0.0, max);
            Ok(e.scroll)
        })
    }

    /// Click at viewport (x, y). Returns resolved link URL if a link was hit.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<String> {
        let l = self.layout.as_ref()?;
        let y = y + self.scroll;
        l.items.iter().rev().find_map(|it| match &it.paint {
            Paint::Word { link: Some(h), .. } if x >= it.x && x <= it.x + it.w && y >= it.y && y <= it.y + it.h => dro_net::resolve(&self.url, h),
            _ => None,
        })
    }
}
