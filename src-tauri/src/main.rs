// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Stdio};
use tauri::{Manager, path::BaseDirectory};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            // Determine the path to the bundled Python executable based on the platform
            let resource_path = "backend/dist/app.exe";

            // Resolve the path to the resource file
            let python_executable = app_handle
                .path()
                .resolve(resource_path, BaseDirectory::Resource)
                .expect("Failed to resolve the Python backend executable path");

            // Launch the Python backend as a subprocess
            Command::new(python_executable)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("Failed to start Python backend");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}