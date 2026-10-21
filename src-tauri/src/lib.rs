mod catalog;
mod commands;
mod docker;
mod error;
mod exec;
mod github;
mod gitsync;
mod ops;
mod script;
mod secrets;
mod state;
mod store;
mod updates;
mod version;

use tauri::Manager;

/// Apps launched from the Finder / Start menu get a minimal PATH: add the
/// usual locations of docker and git so they are found.
fn extend_path() {
    if cfg!(windows) {
        // A Docker Desktop installed while the launcher runs is not yet in
        // the PATH this process inherited.
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let docker_bin = format!(r"{pf}\Docker\Docker\resources\bin");
        let current = std::env::var("PATH").unwrap_or_default();
        if !current
            .split(';')
            .any(|p| p.eq_ignore_ascii_case(&docker_bin))
        {
            std::env::set_var("PATH", format!("{current};{docker_bin}"));
        }
        return;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let extra = [
        "/usr/local/bin".to_string(),
        "/opt/homebrew/bin".to_string(),
        "/Applications/Docker.app/Contents/Resources/bin".to_string(),
        format!("{home}/.docker/bin"),
        // Rancher Desktop and OrbStack put their docker CLI here.
        format!("{home}/.rd/bin"),
        format!("{home}/.orbstack/bin"),
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];
    let current = std::env::var("PATH").unwrap_or_default();
    let mut parts: Vec<String> = current
        .split(':')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    for p in extra {
        if !parts.contains(&p) {
            parts.push(p);
        }
    }
    std::env::set_var("PATH", parts.join(":"));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    extend_path();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = state::AppState::load(app.handle())?;
            app.manage(state);
            updates::spawn_background(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_overview,
            commands::check_updates,
            commands::get_changelog,
            commands::check_launcher_update,
            commands::install_launcher_update,
            commands::save_settings,
            commands::github_status,
            commands::set_github_token,
            commands::clear_github_token,
            commands::check_host,
            commands::docker_start,
            commands::docker_install,
            commands::docker_install_cancel,
            commands::default_install_dir,
            commands::install_app,
            commands::run_action,
            commands::forget_installation,
            commands::set_auto_update,
            commands::installation_status,
            commands::installation_logs,
            commands::get_op_log,
            commands::cancel_op,
            commands::generate_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the launcher");
}
