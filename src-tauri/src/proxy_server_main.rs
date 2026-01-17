use std::sync::Arc;

use antigravity_tools_lib::{
    modules,
    proxy::{
        monitor::ProxyMonitor, AxumServer, ProxySecurityConfig, TokenManager, ZaiDispatchMode,
    },
};

/// Headless entrypoint for running the Antigravity proxy server without Tauri/GUI.
///
/// This binary:
/// - Loads existing configuration from ~/.antigravity_tools/gui_config.json
/// - Uses the same accounts and routing logic as the desktop app
/// - Starts the Axum HTTP proxy server and blocks until it stops
#[tokio::main]
async fn main() {
    // Initialize logging as in the GUI app
    modules::logger::init_logger();

    if let Err(e) = run_headless_proxy().await {
        eprintln!("Failed to start Antigravity proxy server: {}", e);
        // Also log via tracing, in case stdout isn't captured
        tracing::error!("Failed to start Antigravity proxy server: {}", e);
        std::process::exit(1);
    }
}

async fn run_headless_proxy() -> Result<(), String> {
    // 1. Load full app config (including proxy config) from ~/.antigravity_tools/gui_config.json
    let mut app_config = modules::config::load_app_config()
        .map_err(|e| format!("load_app_config failed: {}", e))?;
    let mut proxy_cfg = app_config.proxy.clone();

    // 2. Create monitor (no Tauri AppHandle in headless mode)
    let monitor = Arc::new(ProxyMonitor::new(1000, None));
    monitor.set_enabled(proxy_cfg.enable_logging);

    // 3. Initialize TokenManager using the same data dir as the GUI
    let data_dir = modules::account::get_data_dir()
        .map_err(|e| format!("get_data_dir failed: {}", e))?;
    // Ensure accounts dir exists (even if you only use z.ai)
    let _ = modules::account::get_accounts_dir();

    let token_manager = Arc::new(TokenManager::new(data_dir.clone()));
    // Apply scheduling config from GUI
    token_manager
        .update_sticky_config(proxy_cfg.scheduling.clone())
        .await;

    // 4. Load accounts from disk
    let active_accounts = token_manager
        .load_accounts()
        .await
        .map_err(|e| format!("load_accounts failed: {}", e))?;

    // If no local accounts and z.ai is disabled, refuse to start
    let zai_enabled = proxy_cfg.zai.enabled
        && !matches!(proxy_cfg.zai.dispatch_mode, ZaiDispatchMode::Off);
    if active_accounts == 0 && !zai_enabled {
        return Err(
            "No active accounts found and z.ai is disabled. Please add accounts via the GUI \
             on another machine, then copy ~/.antigravity_tools to this server."
                .to_string(),
        );
    }

    // 5. Start Axum server
    let (server, handle) = AxumServer::start(
        proxy_cfg.get_bind_address().to_string(),
        proxy_cfg.port,
        token_manager.clone(),
        proxy_cfg.custom_mapping.clone(),
        proxy_cfg.request_timeout,
        proxy_cfg.upstream_proxy.clone(),
        ProxySecurityConfig::from_proxy_config(&proxy_cfg),
        proxy_cfg.zai.clone(),
        monitor.clone(),
        proxy_cfg.experimental.clone(),
    )
    .await
    .map_err(|e| format!("AxumServer::start failed: {}", e))?;

    // Keep the server instance alive so shutdown channel is not dropped.
    let _server = server;

    tracing::info!(
        "Headless Antigravity proxy server listening on http://{}:{}",
        proxy_cfg.get_bind_address(),
        proxy_cfg.port
    );

    // 6. Persist any proxy config changes back to gui_config.json (optional, but keeps formats aligned)
    app_config.proxy = proxy_cfg;
    if let Err(e) = modules::config::save_app_config(&app_config) {
        tracing::warn!("Failed to save updated app config: {}", e);
    }

    // 7. Wait for the HTTP server task to finish
    handle
        .await
        .map_err(|e| format!("Proxy server task terminated with error: {}", e))?;

    Ok(())
}