use crate::cli::tui::app::VERSION;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_help_page(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" Help ")
        .title_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let help_data = help_lines();
    let lines: Vec<Line> = help_data
        .iter()
        .map(|s| {
            if s.starts_with("KEYBINDINGS")
                || s.starts_with("VERSION INFO")
                || s.starts_with("DOCUMENTATION")
            {
                Line::from(Span::styled(
                    s.as_str(),
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(s.as_str(), Style::new().fg(Color::White)))
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn help_lines() -> Vec<String> {
    vec![
        "KEYBINDINGS".to_string(),
        "".to_string(),
        "  q / Ctrl+C    Quit (with confirmation in safe mode)".to_string(),
        "  Tab           Next section".to_string(),
        "  Shift+Tab     Previous section".to_string(),
        "  Up/Down       Navigate list".to_string(),
        "  Enter         Select / Connect".to_string(),
        "  r             Refresh current view".to_string(),
        "  e             Toggle event log viewer".to_string(),
        "  ?             Toggle contextual help".to_string(),
        "  :             Open command palette".to_string(),
        "  Esc           Back / Close help / Close palette".to_string(),
        "  h             Go to Help section".to_string(),
        "  Ctrl+B        Create backup".to_string(),
        "  Ctrl+R        Restore backup (via CLI)".to_string(),
        "  Ctrl+E        Execute query (Queries section)".to_string(),
        "  Ctrl+D        Disconnect (with confirmation)".to_string(),
        "  Ctrl+L        Clear query results / logs".to_string(),
        "  Ctrl+M        Toggle migration wizard".to_string(),
        "".to_string(),
        "MOUSE SUPPORT".to_string(),
        "".to_string(),
        "  Left Click    Sidebar: navigate to section".to_string(),
        "  Left Click    Content: select item in list".to_string(),
        "  Scroll        Scroll content / results".to_string(),
        "  Right Click   Toggle contextual help".to_string(),
        "".to_string(),
        "COMMAND PALETTE".to_string(),
        "".to_string(),
        "  :help              Open this help".to_string(),
        "  :quit              Quit the TUI".to_string(),
        "  :refresh           Refresh current view".to_string(),
        "  :connect <url>     Connect to a server".to_string(),
        "  :disconnect        Disconnect from server".to_string(),
        "  :dashboard         Go to Dashboard".to_string(),
        "  :query             Go to Query Console".to_string(),
        "  :cluster           Go to Cluster".to_string(),
        "  :settings          Go to Settings".to_string(),
        "  :security          Go to Security Center".to_string(),
        "  :document          Go to Document Editor".to_string(),
        "  :terminal          Go to Integrated Terminal".to_string(),
        "  :monitor           Go to Monitoring".to_string(),
        "  :search            Search everywhere".to_string(),
        "  :session next      Switch to next session".to_string(),
        "  :session prev      Switch to previous session".to_string(),
        "  :sessions          Toggle session manager".to_string(),
        "  :backup            Go to Backup & Restore".to_string(),
        "  :backup create     Create a backup".to_string(),
        "  :status            Show server status".to_string(),
        "  :health            Show server health".to_string(),
        "  :doc create        Create a new document".to_string(),
        "  :doc validate      Validate current JSON".to_string(),
        "  :terminal clear    Clear terminal output".to_string(),
        "  :export <format>   Export current data".to_string(),
        "".to_string(),
        "WORKSPACES".to_string(),
        "".to_string(),
        "  Dashboard     \u{2014} Live server overview with metrics, gauges, and health".to_string(),
        "  Query Console \u{2014} SQL/UQL editor with history, scrolling, and results".to_string(),
        "  DB & Engines  \u{2014} Storage engine inspection and database listing".to_string(),
        "  Namespaces    \u{2014} Data isolation namespace management".to_string(),
        "  Cluster       \u{2014} Node topology, Raft status, sharding, replication".to_string(),
        "  Federation    \u{2014} Cross-cluster DataDomains and federated clusters".to_string(),
        "  Governor      \u{2014} Resource execution monitoring and policy enforcement".to_string(),
        "  Backup/Restore\u{2014} Backup creation, listing, verification, and restore".to_string(),
        "  Metrics & Logs\u{2014} Prometheus metrics viewer and system log tail".to_string(),
        "  Config Studio \u{2014} Server configuration CRUD, snapshots, export/import".to_string(),
        "  Table Explorer\u{2014} Browse storage types, tables, schemas, and rows".to_string(),
        "  Report Builder\u{2014} Saved report definitions with query execution".to_string(),
        "  Notebook      \u{2014} Multi-cell notebooks (markdown, SQL, analysis, RAG)".to_string(),
        "  RAG Workspace \u{2014} Vector collection search and similarity exploration".to_string(),
        "  Security      \u{2014} User, role, and permission management (RBAC)".to_string(),
        "  Document Ed.  \u{2014} JSON document editor with validation and patch mode".to_string(),
        "  Terminal      \u{2014} Built-in shell for command execution".to_string(),
        "  Monitoring    \u{2014} Live metrics, alerts, performance, replication, resources".to_string(),
        "".to_string(),
        "TIPS".to_string(),
        "".to_string(),
        "  Type a section name or use Tab to cycle.".to_string(),
        "  The status bar shows connection state, namespace,".to_string(),
        "  database, cluster state, version, and hints.".to_string(),
        "  Press 'e' to view the full event log.".to_string(),
        "  Use the command palette for quick navigation.".to_string(),
        "".to_string(),
        "VERSION INFO".to_string(),
        "".to_string(),
        format!("  PrimusDB v{}", VERSION),
        "  Hybrid \u{2022} Columnar \u{2022} Vector \u{2022} Document \u{2022} Relational \u{2022} Key-Value".to_string(),
        "".to_string(),
        "DOCUMENTATION".to_string(),
        "".to_string(),
        "  https://primusdb.dev/docs".to_string(),
        "  https://primusdb.dev/api".to_string(),
    ]
}
