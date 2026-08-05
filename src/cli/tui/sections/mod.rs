pub mod backups;
pub mod config_studio;
pub mod dashboard;
pub mod document_workspace;
pub mod engines;
pub mod federation;
pub mod files;
pub mod governor;
pub mod help;
pub mod integrated_terminal;
pub mod metrics_logs;
pub mod monitoring;
pub mod namespaces;
pub mod nodes;
pub mod notebook;
pub mod queries;
pub mod rag_workspace;
pub mod report_builder;
pub mod security_center;
pub mod settings;
pub mod table_explorer;

use crate::cli::tui::app::{NavSection, TuiApp};
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn render_section(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    // Always prefer workspace registry — render and return immediately
    if let Some(workspace) = app.workspaces.get(&app.current_section) {
        workspace.render(frame, area, app);
        return;
    }

    // Legacy fallback — only reached for sections without a registered workspace
    match app.current_section {
        NavSection::Dashboard => dashboard::render_dashboard(frame, area, app),
        NavSection::QueryConsole => queries::render_queries(frame, area, app),
        NavSection::DatabasesEngines => engines::render_engines(frame, area, app),
        NavSection::Namespaces => namespaces::render_namespaces(frame, area, app),
        NavSection::Cluster => nodes::render_nodes(frame, area, app),
        NavSection::Federation => federation::render_federation(frame, area, app),
        NavSection::Governor => governor::render_governor(frame, area, app),
        NavSection::BackupRestore => backups::render_backups(frame, area, app),
        NavSection::MetricsLogs => metrics_logs::render_metrics_view(frame, area, app),
        NavSection::ConfigurationStudio => config_studio::render_config_studio(frame, area, app),
        NavSection::TableExplorer => table_explorer::render_table_explorer(frame, area, app),
        NavSection::ReportBuilder => report_builder::render_report_builder(frame, area, app),
        NavSection::Notebook => notebook::render_notebook(frame, area, app),
        NavSection::RAGWorkspace => rag_workspace::render_rag_workspace(frame, area, app),
        NavSection::SecurityCenter => security_center::render_security_center(frame, area, app),
        NavSection::DocumentWorkspace => {
            document_workspace::render_document_workspace(frame, area, app)
        }
        NavSection::IntegratedTerminal => {
            integrated_terminal::render_integrated_terminal(frame, area, app)
        }
        NavSection::Monitoring => monitoring::render_monitoring(frame, area, app),
        NavSection::Settings => settings::render_settings(frame, area, app),
        NavSection::FileBrowser => files::render_file_browser(frame, area, app),
        NavSection::Help => help::render_help_page(frame, area),
    }
}
