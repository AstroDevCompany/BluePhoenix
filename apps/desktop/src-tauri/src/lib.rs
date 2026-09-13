mod ai;
mod ai_store;
mod command_scan;
mod commands;
mod db;
mod documents;
mod error;
mod git;
mod jobs;
#[cfg(target_os = "macos")]
mod macos;
mod models;
mod native;
mod project_facts;
mod secrets;
mod state;
mod sync;
mod updater;

use crate::db::Db;
use crate::native::data_dir;
use crate::state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("bluephoenix=info")
        .with_target(false)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            #[cfg(desktop)]
            {
                let _ = app
                    .handle()
                    .plugin(tauri_plugin_updater::Builder::new().build());
                let _ = app.handle().plugin(tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    None,
                ));
            }
            let dir = data_dir().expect("data dir");
            let db = Db::open(&dir.join("bluephoenix.sqlite")).expect("open sqlite");
            let http = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .expect("http");
            app.manage(AppState { db, http });
            crate::secrets::init(&dir);
            crate::jobs::spawn(app.handle().clone());
            #[cfg(desktop)]
            {
                crate::commands::sync_launch_at_startup(app.handle());
            }

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    let _ = window.set_decorations(true);
                    let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
                    let _ = window.set_title("");
                    crate::macos::install(&window);
                }
                let _ = window.set_shadow(true);
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
            }

            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
                let show = MenuItem::with_id(app, "show", "Show BluePhoenix", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;
                TrayIconBuilder::with_id("tray")
                    .tooltip("BluePhoenix")
                    .icon(tauri::include_image!("icons/32x32.png"))
                    .icon_as_template(false)
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::get_settings,
            commands::save_settings,
            commands::list_categories,
            commands::set_category_enabled,
            commands::create_custom_category,
            commands::list_tags,
            commands::create_custom_tag,
            commands::list_projects,
            commands::get_project,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::running_timer,
            commands::start_timer,
            commands::stop_timer,
            commands::pause_timer,
            commands::add_manual_entry,
            commands::update_time_entry,
            commands::delete_time_entry,
            commands::list_time_entries,
            commands::list_todos,
            commands::create_todo,
            commands::set_todo_status,
            commands::delete_todo,
            commands::todo_kinds,
            commands::list_links,
            commands::upsert_link,
            commands::delete_link,
            commands::list_project_files,
            commands::add_project_file,
            commands::remove_project_file,
            commands::list_folder,
            commands::scan_project_folder,
            commands::cached_files,
            commands::open_path,
            commands::reveal_path,
            commands::open_url,
            commands::open_vscode,
            commands::open_terminal,
            commands::list_commands,
            commands::upsert_command,
            commands::delete_command,
            commands::run_command,
            commands::git_status,
            commands::git_run,
            commands::list_versions,
            commands::add_version,
            commands::list_topics,
            commands::upsert_topic,
            commands::delete_topic,
            commands::list_lessons,
            commands::upsert_lesson,
            commands::delete_lesson,
            commands::list_exams,
            commands::upsert_exam,
            commands::delete_exam,
            commands::university_dashboard,
            commands::list_activity,
            commands::list_achievements,
            commands::achievement_catalog,
            commands::global_progress,
            commands::search,
            commands::project_context,
            commands::export_data,
            commands::import_data,
            commands::complete_onboarding,
            commands::sync_status,
            commands::check_for_updates,
            commands::local_version,
            commands::pick_folder,
            commands::pick_file,
            commands::enqueue_index_job,
            commands::poll_jobs,
            commands::store_encrypted_secret,
            ai::ai_status,
            ai::ai_save_settings,
            ai::ai_set_key,
            ai::ai_clear_key,
            ai::ai_reveal_key,
            ai::ai_test_connection,
            ai::ai_chat_stream,
            ai::ai_list_conversations,
            ai::ai_list_messages,
            ai::ai_new_conversation,
            ai::ai_clear_conversation,
            ai::ai_project_bundle,
            ai::ai_interpret_search,
            ai::ai_prioritize_todos,
            ai::ai_apply_todo_priorities,
            ai::ai_generate_prompt,
            ai::ai_summarize_changelog,
            ai::ai_generate_changelog,
            ai::ai_todos_from_changelog,
            ai::ai_suggest_commit,
            ai::ai_inspect_software_folder,
            ai::ai_scan_commands,
            ai::git_log,
            ai::git_diff,
            ai::list_document_records,
            commands::auth_register,
            commands::auth_login,
            commands::auth_logout,
            commands::auth_forgot,
            commands::push_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BluePhoenix");
}
