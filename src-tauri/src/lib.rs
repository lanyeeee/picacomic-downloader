use anyhow::Context;
use parking_lot::RwLock;
use tauri::{Manager, Wry};

// TODO: 使用 prelude 来消除警告
use crate::commands::*;
use crate::config::Config;
use crate::download_manager::DownloadManager;
use crate::events::{
    DownloadSpeedEvent, DownloadTaskEvent, ExportCbzEvent, ExportPdfEvent, LogEvent,
};
use crate::pica_client::PicaClient;

mod commands;
mod config;
mod download_manager;
mod errors;
mod events;
mod export;
mod extensions;
mod logger;
mod pica_client;
mod responses;
mod types;
mod utils;

fn generate_context() -> tauri::Context<Wry> {
    tauri::generate_context!()
}

// TODO: 添加 Panic doc
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<Wry>::new()
        .commands(tauri_specta::collect_commands![
            greet,
            get_config,
            save_config,
            login,
            get_user_profile,
            search_comic,
            get_comic,
            get_chapter_image,
            download_comic,
            create_download_task,
            pause_download_task,
            resume_download_task,
            cancel_download_task,
            show_path_in_file_manager,
            show_comic_download_dir_in_file_manager,
            get_favorite,
            get_downloaded_comics,
            export_cbz,
            export_pdf,
            get_logs_dir_size,
        ])
        .events(tauri_specta::collect_events![
            DownloadSpeedEvent,
            DownloadTaskEvent,
            ExportCbzEvent,
            ExportPdfEvent,
            LogEvent,
        ]);

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number)
                .formatter(specta_typescript::formatter::prettier)
                .header("// @ts-nocheck"), // 跳过检查
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let app_data_dir = app
                .path()
                .app_data_dir()
                .context("failed to get app data dir")?;

            std::fs::create_dir_all(&app_data_dir)
                .context(format!("failed to create app data dir: {app_data_dir:?}"))?;
            println!("app data dir: {app_data_dir:?}");

            let config = RwLock::new(Config::new(app.handle())?);
            app.manage(config);

            let pica_client = PicaClient::new(app.handle().clone());
            app.manage(pica_client);

            let download_manager = DownloadManager::new(app.handle().clone());
            app.manage(download_manager);

            logger::init(app.handle())?;

            Ok(())
        })
        .run(generate_context())
        .expect("error while running tauri application");
}
