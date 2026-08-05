use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Resolved layout areas for a workspace.
#[derive(Debug, Clone)]
pub struct ResolvedLayout {
    pub main: Rect,
    pub inspector: Option<Rect>,
    pub tabs: Vec<Rect>,
    pub overlay: Option<Rect>,
    pub status: Rect,
}

/// Resolve a LayoutSpec into concrete Rects.
pub fn resolve_layout(spec: &super::workspace::LayoutSpec, area: Rect) -> ResolvedLayout {
    match spec {
        super::workspace::LayoutSpec::Single => ResolvedLayout {
            main: area,
            inspector: None,
            tabs: vec![area],
            overlay: None,
            status: area,
        },
        super::workspace::LayoutSpec::Horizontal { ratio, .. } => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage((ratio * 100.0) as u16),
                    Constraint::Percentage(((1.0 - ratio) * 100.0) as u16),
                ])
                .split(area);
            ResolvedLayout {
                main: chunks[0],
                inspector: None,
                tabs: vec![chunks[0], chunks[1]],
                overlay: None,
                status: area,
            }
        }
        super::workspace::LayoutSpec::Vertical { ratio, .. } => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage((ratio * 100.0) as u16),
                    Constraint::Percentage(((1.0 - ratio) * 100.0) as u16),
                ])
                .split(area);
            ResolvedLayout {
                main: chunks[0],
                inspector: None,
                tabs: vec![chunks[0], chunks[1]],
                overlay: None,
                status: area,
            }
        }
        super::workspace::LayoutSpec::WithInspector {
            inspector_width, ..
        } => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(40), Constraint::Length(*inspector_width)])
                .split(area);
            ResolvedLayout {
                main: chunks[0],
                inspector: Some(chunks[1]),
                tabs: vec![chunks[0]],
                overlay: None,
                status: area,
            }
        }
        _ => ResolvedLayout {
            main: area,
            inspector: None,
            tabs: vec![area],
            overlay: None,
            status: area,
        },
    }
}

/// Resolve layout for the full application frame.
pub fn resolve_app_layout(
    area: Rect,
    sidebar_width: u16,
    header_height: u16,
    status_height: u16,
) -> AppLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(10),
            Constraint::Length(status_height),
        ])
        .split(area);

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(sidebar_width), Constraint::Min(20)])
        .split(chunks[1]);

    AppLayout {
        header: chunks[0],
        sidebar: middle_chunks[0],
        content: middle_chunks[1],
        status: chunks[2],
    }
}

#[derive(Debug, Clone)]
pub struct AppLayout {
    pub header: Rect,
    pub sidebar: Rect,
    pub content: Rect,
    pub status: Rect,
}
