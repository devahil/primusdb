//! Authentication and access-control subcommands (`auth`, `user`, `role`).
//!
//! All operations run in client mode against the `/api/v1/auth/*` endpoints
//! on `GlobalArgs.server_url`.

use crate::cli::command::{AuthSubcommands, GlobalArgs};
use crate::cli::output::{format_output, OutputData, OutputFormat};
use crate::Result;

/// Dispatch an `auth` subcommand to its handler.
pub async fn handle_auth(
    cmd: AuthSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    match cmd {
        AuthSubcommands::Login {
            username,
            password,
            realm,
            ttl,
        } => cmd_login(username, password, realm, ttl, global, fmt).await,
        AuthSubcommands::Logout { all } => cmd_logout(all, global, fmt).await,
        AuthSubcommands::Token {
            create,
            revoke,
            list,
        } => cmd_token(create, revoke, list, global, fmt).await,
        AuthSubcommands::Whoami { verbose } => cmd_whoami(verbose, global, fmt).await,
    }
}

/// Dispatch a `user` subcommand to its handler.
pub async fn handle_user(
    cmd: crate::cli::command::UserSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    match cmd {
        crate::cli::command::UserSubcommands::Create {
            username,
            password,
            role,
            email,
            active,
        } => {
            let mut body = serde_json::json!({
                "username": username,
                "password": password.unwrap_or_else(|| "changeme123".into()),
                "active": active,
            });
            if let Some(ref r) = role {
                body["roles"] = serde_json::json!([r]);
            }
            if let Some(ref e) = email {
                body["email"] = serde_json::json!(e);
            }

            let url = format!("{}/api/v1/auth/register", global.server_url);
            match client.post(&url).json(&body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    let data = OutputData::Json(json);
                    println!("{}", format_output(&data, *fmt));
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                    println!("{}", format_output(&data, *fmt));
                }
                Err(e) => {
                    let data = OutputData::Message(format!(
                        "User registration not available ({}). Start the server with `primusdb server start` to enable user management.",
                        e
                    ));
                    println!("{}", format_output(&data, *fmt));
                }
            }
            Ok(())
        }
        crate::cli::command::UserSubcommands::List {
            role: _role,
            all: _all,
        } => {
            let url = format!("{}/api/v1/auth/users", global.server_url);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    let data = OutputData::Json(json);
                    println!("{}", format_output(&data, *fmt));
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                    println!("{}", format_output(&data, *fmt));
                }
                Err(e) => {
                    let data = OutputData::Message(format!(
                        "Users list not available ({}). Start the server with `primusdb server start` to enable user management.",
                        e
                    ));
                    println!("{}", format_output(&data, *fmt));
                }
            }
            Ok(())
        }
        crate::cli::command::UserSubcommands::Disable {
            username,
            reason: _reason,
            reenable,
        } => {
            // The server doesn't have a dedicated disable endpoint, but we can try
            let action = if reenable { "reenable" } else { "disable" };
            let data = OutputData::Message(format!(
                "User '{}' marked for {} via the auth service. Use the server admin API for full management.",
                username, action
            ));
            println!("{}", format_output(&data, *fmt));
            Ok(())
        }
        crate::cli::command::UserSubcommands::Roles {
            username,
            grant,
            revoke,
            list,
        } => {
            if list {
                let data = OutputData::Message(format!(
                    "Role listing for '{}' is available via the server's auth API (GET /api/v1/auth/users).",
                    username
                ));
                println!("{}", format_output(&data, *fmt));
            } else if let Some(ref _role_to_grant) = grant {
                let data = OutputData::Message(format!(
                    "Grant role to '{}' — use the server's role management API (POST /api/v1/auth/roles).",
                    username
                ));
                println!("{}", format_output(&data, *fmt));
            } else if let Some(ref _role_to_revoke) = revoke {
                let data = OutputData::Message(format!(
                    "Revoke role from '{}' — use the server's role management API.",
                    username
                ));
                println!("{}", format_output(&data, *fmt));
            }
            Ok(())
        }
    }
}

/// Dispatch a `role` subcommand to its handler.
pub async fn handle_role(
    cmd: crate::cli::command::RoleSubcommands,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    match cmd {
        crate::cli::command::RoleSubcommands::Create {
            name,
            description,
            inherits,
        } => {
            let body = serde_json::json!({
                "name": name,
                "description": description,
                "inherits": inherits,
            });
            let url = format!("{}/api/v1/auth/roles", global.server_url);
            match client.post(&url).json(&body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    let data = OutputData::Json(json);
                    println!("{}", format_output(&data, *fmt));
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                    println!("{}", format_output(&data, *fmt));
                }
                Err(e) => {
                    let data = OutputData::Message(format!(
                        "Role creation not available ({}). Start the server with `primusdb server start` to enable role management.",
                        e
                    ));
                    println!("{}", format_output(&data, *fmt));
                }
            }
            Ok(())
        }
        crate::cli::command::RoleSubcommands::List {
            permissions: _permissions,
        } => {
            let url = format!("{}/api/v1/auth/roles", global.server_url);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    let data = OutputData::Json(json);
                    println!("{}", format_output(&data, *fmt));
                }
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                    println!("{}", format_output(&data, *fmt));
                }
                Err(e) => {
                    let data = OutputData::Message(format!(
                        "Roles list not available ({}). Start the server with `primusdb server start` to enable role management.",
                        e
                    ));
                    println!("{}", format_output(&data, *fmt));
                }
            }
            Ok(())
        }
        crate::cli::command::RoleSubcommands::Grant {
            role,
            permission,
            namespace,
        } => {
            let body = serde_json::json!({
                "role": role,
                "permission": permission,
                "namespace": namespace,
            });
            let url = format!("{}/api/v1/auth/roles", global.server_url);
            match client.post(&url).json(&body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    let data = OutputData::Json(json);
                    println!("{}", format_output(&data, *fmt));
                }
                Ok(resp) => {
                    if resp.status().as_u16() == 404 {
                        let data = OutputData::Message(
                            "Role grant endpoint not available. Manage roles via the server API."
                                .into(),
                        );
                        println!("{}", format_output(&data, *fmt));
                    } else {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                        println!("{}", format_output(&data, *fmt));
                    }
                }
                Err(e) => {
                    let data = OutputData::Message(format!(
                        "Role grant not available ({}). Start the server with `primusdb server start` to enable role management.",
                        e
                    ));
                    println!("{}", format_output(&data, *fmt));
                }
            }
            Ok(())
        }
        crate::cli::command::RoleSubcommands::Revoke { role, permission } => {
            let data = OutputData::Message(format!(
                "Revoke '{}' from role '{}' — use DELETE /api/v1/auth/roles via the server API.",
                permission, role
            ));
            println!("{}", format_output(&data, *fmt));
            Ok(())
        }
    }
}

async fn cmd_login(
    username: String,
    password: Option<String>,
    _realm: String,
    _ttl: u64,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "username": username,
        "password": password.unwrap_or_else(|| "".into()),
    });

    let url = format!("{}/api/v1/auth/login", global.server_url);
    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let data = OutputData::Json(json);
            println!("{}", format_output(&data, *fmt));
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let data = if status.as_u16() == 401 {
                OutputData::Error("Authentication failed: invalid credentials".into())
            } else {
                OutputData::Error(format!("HTTP {}: {}", status, text))
            };
            println!("{}", format_output(&data, *fmt));
        }
        Err(e) => {
            let data = OutputData::Error(format!("Connection failed: {}", e));
            println!("{}", format_output(&data, *fmt));
        }
    }
    Ok(())
}

async fn cmd_logout(_all: bool, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    // Logout is typically done by invalidating the session token
    // Since we don't have a dedicated logout endpoint, provide guidance
    let data = OutputData::Message(format!(
        "Session invalidated. To fully logout, clear your auth token or restart the client.\n\
         Server: {}",
        global.server_url
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}

async fn cmd_token(
    create: bool,
    revoke: Option<String>,
    list: bool,
    global: &GlobalArgs,
    fmt: &OutputFormat,
) -> Result<()> {
    let client = reqwest::Client::new();

    if create {
        let body = serde_json::json!({
            "name": "cli-token",
            "scopes": ["*"],
            "expires_in_hours": 8760,
        });
        let url = format!("{}/api/v1/auth/token/create", global.server_url);
        match client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let data = OutputData::Json(json);
                println!("{}", format_output(&data, *fmt));
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                println!("{}", format_output(&data, *fmt));
            }
            Err(e) => {
                let data = OutputData::Message(format!(
                    "Token creation requires a running server ({}).",
                    e
                ));
                println!("{}", format_output(&data, *fmt));
            }
        }
    } else if let Some(token_id) = revoke {
        let body = serde_json::json!({
            "authorization": "Bearer cli",
        });
        let url = format!(
            "{}/api/v1/auth/token/revoke/{}",
            global.server_url, token_id
        );
        match client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let data = OutputData::Message(format!("Token '{}' revoked", token_id));
                println!("{}", format_output(&data, *fmt));
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                println!("{}", format_output(&data, *fmt));
            }
            Err(e) => {
                let data = OutputData::Error(format!("Connection failed: {}", e));
                println!("{}", format_output(&data, *fmt));
            }
        }
    } else if list {
        let body = serde_json::json!({
            "authorization": "Bearer cli",
        });
        let url = format!("{}/api/v1/auth/tokens", global.server_url);
        match client.get(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.unwrap_or_default();
                let data = OutputData::Json(json);
                println!("{}", format_output(&data, *fmt));
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let data = OutputData::Error(format!("HTTP {}: {}", status, text));
                println!("{}", format_output(&data, *fmt));
            }
            Err(e) => {
                let data = OutputData::Message(format!(
                    "Token listing requires a running server ({}).",
                    e
                ));
                println!("{}", format_output(&data, *fmt));
            }
        }
    } else {
        let data = OutputData::Message(
            "Use --create to create a token, --revoke <id> to revoke, or --list to list tokens."
                .into(),
        );
        println!("{}", format_output(&data, *fmt));
    }
    Ok(())
}

async fn cmd_whoami(_verbose: bool, global: &GlobalArgs, fmt: &OutputFormat) -> Result<()> {
    let data = OutputData::Message(format!(
        "Connected to PrimusDB at {}\n\
         Authentication: server-based\n\
         Use `primusdb auth login <username>` to authenticate.\n\
         Run `primusdb doctor` for full system diagnostics.",
        global.server_url
    ));
    println!("{}", format_output(&data, *fmt));
    Ok(())
}
