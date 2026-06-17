pub mod aiml;
pub mod dashboard;
pub mod databases;
pub mod diagnostics;
pub mod engines;
pub mod governor;
pub mod graph;
pub mod instances;
pub mod namespaces;
pub mod nodes;
pub mod restores;
pub mod roles;
pub mod settings;
pub mod tables;
pub mod users;
pub mod vector_indexes;

use crate::cli::tui::app::{NavSection, TuiApp};
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn render_section(frame: &mut Frame, area: Rect, app: &mut TuiApp) {
    match app.current_section {
        NavSection::Dashboard => dashboard::render_dashboard(frame, area, app),
        NavSection::Instances => instances::render_instances(frame, area, app),
        NavSection::Clusters => super::render::render_clusters(frame, area, app),
        NavSection::Queries => super::render::render_queries(frame, area, app),
        NavSection::Engines => engines::render_engines(frame, area, app),
        NavSection::Databases => databases::render_databases(frame, area, app),
        NavSection::Namespaces => namespaces::render_namespaces(frame, area, app),
        NavSection::Users => users::render_users(frame, area, app),
        NavSection::Diagnostics => diagnostics::render_diagnostics(frame, area, app),
        NavSection::Settings => settings::render_settings(frame, area, app),
        NavSection::Backups => super::render::render_backups(frame, area, app),
        NavSection::Migrations => super::render::render_migrations(frame, area, app),
        NavSection::Metrics => super::render::render_metrics_view(frame, area, app),
        NavSection::Logs => super::render::render_logs(frame, area, app),
        NavSection::Help => super::render::render_help_page(frame, area),
        NavSection::Nodes => nodes::render_nodes(frame, area, app),
        NavSection::TablesCollections => tables::render_tables(frame, area, app),
        NavSection::VectorIndexes => vector_indexes::render_vector_indexes(frame, area, app),
        NavSection::Graph => graph::render_graph(frame, area, app),
        NavSection::AIML => aiml::render_aiml(frame, area, app),
        NavSection::Restores => restores::render_restores(frame, area, app),
        NavSection::Roles => roles::render_roles(frame, area, app),
        NavSection::Governor => governor::render_governor(frame, area, app),
    }
}
