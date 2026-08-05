use crate::cli::tui::app::TuiApp;
use crate::cli::tui::capability::Capability;
use crate::cli::tui::event::AppMessage;
use std::collections::HashMap;
use tokio::sync::mpsc;

type CmdHandler = Box<dyn Fn(&str, &mut TuiApp, &mpsc::UnboundedSender<AppMessage>) + Send + Sync>;

static HANDLERS: std::sync::OnceLock<HashMap<&'static str, CmdHandler>> =
    std::sync::OnceLock::new();

pub fn init() {
    let mut h: HashMap<&'static str, CmdHandler> = HashMap::new();

    // ── Server / Cluster subprocess commands ──
    h.insert(
        "server.start",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["server", "start"],
                "Server started",
                "Start failed",
            );
        }),
    );
    h.insert(
        "server.stop",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["server", "stop"],
                "Server stopped",
                "Stop failed",
            );
        }),
    );
    h.insert(
        "server.restart",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["server", "restart"],
                "Server restarted",
                "Restart failed",
            );
        }),
    );
    h.insert(
        "cluster.leave",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["cluster", "leave"],
                "Left cluster",
                "Leave failed",
            );
        }),
    );
    h.insert(
        "cluster.rebalance",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["cluster", "rebalance"],
                "Cluster rebalanced",
                "Rebalance failed",
            );
        }),
    );
    h.insert(
        "maintenance.on",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["maintenance", "on"],
                "Maintenance mode enabled",
                "Enable failed",
            );
        }),
    );
    h.insert(
        "maintenance.off",
        Box::new(|_, app, tx| {
            run_primusdb(
                app,
                tx,
                &["maintenance", "off"],
                "Maintenance mode disabled",
                "Disable failed",
            );
        }),
    );
    h.insert(
        "backup.create",
        Box::new(|_, app, tx| {
            let tx = tx.clone();
            app.add_event("Creating backup...".to_string());
            tokio::spawn(async move {
                match std::process::Command::new("primusdb")
                    .args(["backup", "create"])
                    .output()
                {
                    Ok(o) if o.status.success() => {
                        let msg = String::from_utf8_lossy(&o.stdout).trim().to_string();
                        let _ = tx.send(AppMessage::BackupCreated(msg));
                    }
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                        let _ = tx.send(AppMessage::BackupCreated(format!("Error: {}", err)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::BackupCreated(format!("Error: {}", e)));
                    }
                }
            });
        }),
    );

    // ── Doctor diagnostics ──
    h.insert(
        "doctor",
        Box::new(|_, app, tx| {
            app.add_event("Running doctor diagnostics...".to_string());
            let tx = tx.clone();
            tokio::spawn(async move {
                match std::process::Command::new("primusdb")
                    .args(["doctor", "--tui"])
                    .output()
                {
                    Ok(o) => {
                        let out = if o.status.success() {
                            String::from_utf8_lossy(&o.stdout).trim().to_string()
                        } else {
                            String::from_utf8_lossy(&o.stderr).trim().to_string()
                        };
                        let _ = tx.send(AppMessage::DoctorResult(out));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::DoctorResult(format!("Error: {}", e)));
                    }
                }
            });
        }),
    );

    // ── Backup verify — needs an ID from args ──
    h.insert(
        "backup.verify",
        Box::new(|args, app, tx| {
            let id = args.trim();
            if id.is_empty() {
                app.add_event("Usage: :backup verify <id>".to_string());
                return;
            }
            app.add_event(format!("Verifying backup {}...", id));
            let id = id.to_string();
            let tx = tx.clone();
            tokio::spawn(async move {
                match std::process::Command::new("primusdb")
                    .args(["backup", "verify", &id])
                    .output()
                {
                    Ok(o) if o.status.success() => {
                        let msg = String::from_utf8_lossy(&o.stdout).trim().to_string();
                        let _ = tx.send(AppMessage::BackupVerified(msg));
                    }
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                        let _ = tx.send(AppMessage::BackupVerified(format!("Error: {}", err)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::BackupVerified(format!("Error: {}", e)));
                    }
                }
            });
        }),
    );

    let _ = HANDLERS.set(h);
}

fn run_primusdb(
    app: &mut TuiApp,
    tx: &mpsc::UnboundedSender<AppMessage>,
    args: &[&str],
    success_msg: &'static str,
    fail_prefix: &'static str,
) {
    app.add_event(format!("{}...", success_msg));
    let tx = tx.clone();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::spawn(async move {
        match std::process::Command::new("primusdb").args(&args).output() {
            Ok(o) if o.status.success() => {
                let extra = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let msg = if extra.is_empty() {
                    success_msg.to_string()
                } else {
                    format!("{}: {}", success_msg, extra)
                };
                let _ = tx.send(AppMessage::ClusterStarted(msg));
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                let _ = tx.send(AppMessage::ClusterError(format!(
                    "{}: {}",
                    fail_prefix, err
                )));
            }
            Err(e) => {
                let _ = tx.send(AppMessage::ClusterError(format!("{}: {}", fail_prefix, e)));
            }
        }
    });
}

pub fn dispatch(action: &str, app: &mut TuiApp, tx: &mpsc::UnboundedSender<AppMessage>) -> bool {
    let Some(handlers) = HANDLERS.get() else {
        return false;
    };

    let cap_id = action.strip_prefix("capability:").unwrap_or(action);
    let (id, args) = if let Some(pos) = cap_id.find(':') {
        let (base, rest) = cap_id.split_at(pos);
        (base, &rest[1..])
    } else {
        (cap_id, "")
    };

    if let Some(handler) = handlers.get(id) {
        handler(args, app, tx);
        true
    } else {
        false
    }
}

pub fn is_registered(id: &str) -> bool {
    HANDLERS.get().is_some_and(|h| h.contains_key(id))
}

// ── Plugin Architecture ──

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn register_capabilities(&self) -> Vec<Capability>;
}

pub struct PluginRegistry {
    pub plugins: Vec<Box<dyn Plugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name();
        self.plugins.push(plugin);
        eprintln!("[plugin] Registered: {}", name);
    }
}
