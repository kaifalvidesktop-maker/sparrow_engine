use super::omnibox;
use crate::SparrowEngine;

pub struct Tab {
    pub id: u32,
    pub engine: SparrowEngine,
    pub back: Vec<String>,
    pub forward: Vec<String>,
}

pub struct Tabs {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u32,
}

impl Tabs {
    pub fn new() -> Self {
        let mut t = Tabs { tabs: vec![], active: 0, next_id: 1 };
        t.open();
        t
    }
    pub fn open(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab { id, engine: SparrowEngine::new(), back: vec![], forward: vec![] });
        self.active = self.tabs.len() - 1;
        id
    }
    pub fn close(&mut self, id: u32) {
        self.tabs.retain(|t| t.id != id);
        if self.tabs.is_empty() { self.open(); }
        self.active = self.active.min(self.tabs.len() - 1);
    }
    pub fn switch(&mut self, id: u32) {
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) { self.active = i; }
    }
    pub fn list(&self) -> &[Tab] { &self.tabs }
    pub fn active(&mut self) -> &mut Tab {
        let i = self.active.min(self.tabs.len() - 1);
        &mut self.tabs[i]
    }
    /// Address bar text -> navigate active tab.
    pub fn navigate(&mut self, input: &str) -> Result<(), String> {
        let url = omnibox::resolve(input);
        let t = self.active();
        let prev = t.engine.url().to_string();
        t.engine.load_url(&url)?;
        if prev != "about:blank" { t.back.push(prev); }
        t.forward.clear();
        Ok(())
    }
    pub fn go_back(&mut self) -> Result<(), String> {
        let t = self.active();
        let Some(u) = t.back.pop() else { return Ok(()) };
        t.forward.push(t.engine.url().to_string());
        t.engine.load_url(&u)
    }
    pub fn go_forward(&mut self) -> Result<(), String> {
        let t = self.active();
        let Some(u) = t.forward.pop() else { return Ok(()) };
        t.back.push(t.engine.url().to_string());
        t.engine.load_url(&u)
    }
}
