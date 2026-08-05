use crate::cli::tui::panels::query_editor::QueryEditor;

pub struct QueryTabs {
    pub tabs: Vec<QueryTab>,
    pub active: usize,
}

pub struct QueryTab {
    pub name: String,
    pub editor: QueryEditor,
    pub last_result: Option<String>,
}

impl Default for QueryTabs {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryTabs {
    pub fn new() -> Self {
        Self {
            tabs: vec![QueryTab {
                name: "Query 1".into(),
                editor: QueryEditor::new(),
                last_result: None,
            }],
            active: 0,
        }
    }

    pub fn add_tab(&mut self) {
        let n = self.tabs.len() + 1;
        self.tabs.push(QueryTab {
            name: format!("Query {}", n),
            editor: QueryEditor::new(),
            last_result: None,
        });
        self.active = self.tabs.len() - 1;
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active);
            self.active = self.active.min(self.tabs.len() - 1);
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = if self.active == 0 {
                self.tabs.len() - 1
            } else {
                self.active - 1
            };
        }
    }

    pub fn active_editor(&self) -> &QueryEditor {
        &self.tabs[self.active].editor
    }

    pub fn active_editor_mut(&mut self) -> &mut QueryEditor {
        &mut self.tabs[self.active].editor
    }

    pub fn set_result(&mut self, result: String) {
        if let Some(t) = self.tabs.get_mut(self.active) {
            t.last_result = Some(result);
        }
    }
}
