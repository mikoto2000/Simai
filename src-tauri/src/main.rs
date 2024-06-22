// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{path::Path, sync::mpsc};
use std::{process, thread};

use notify::event::ModifyKind;
use notify::{Config, EventKind, PollWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use serde_json::value;
use tauri::api::cli::ArgData;

use tauri::{App, AppHandle, Manager};

#[derive(Deserialize)]
struct TargetFile {
    path: String,
}

fn start_watch(
    app_handle: &AppHandle,
    file_path: &str,
    stop_rx: &mpsc::Receiver<()>,
) -> notify::Result<()> {
    // チャンネルの停止依頼を空にする
    loop {
        if !stop_rx.try_recv().is_ok() {
            break;
        }
    }

    let path = Path::new(file_path).as_ref();
    let (tx, rx) = mpsc::channel();

    let _watcher: Box<dyn Watcher> = if str::starts_with(file_path, "\\\\") {
        println!("polling!");
        let config = Config::default().with_poll_interval(Duration::from_secs(1));
        let mut poll_watcher = PollWatcher::new(tx, config)?;
        poll_watcher.watch(path, RecursiveMode::NonRecursive)?;
        Box::new(poll_watcher)
    } else {
        println!("recommended!");
        let mut recommended_watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })?;
        recommended_watcher.watch(path, RecursiveMode::NonRecursive)?;
        Box::new(recommended_watcher)
    };

    // 初回の描画
    let file_contents = get_file_content(file_path);
    app_handle.emit_all("update_md", file_contents).unwrap();

    // 監視イベント受信処理処理スレッド
    let app_handle = app_handle.clone();
    thread::spawn(move || {
        while let Ok(res) = rx.recv() {
            match res {
                Ok(event) => {
                    let path = event.paths[0].clone();
                    let path = path.into_os_string().into_string().unwrap();
                    // フロントエンドでは
                    // JSON -> オブジェクト -> 文字列 と解釈していくので 2 回エスケープされる
                    // 2 回のエスケープで想定通りとなるように、ここでバックスラッシュを増やす
                    // TODO 他のエスケープ記号はどうしよう...
                    let path_string = path.replace("\\", "\\\\");
                    let path = "{\"path\":\"".to_string() + &path_string + "\"}";
                    println!("raw path: {:?}", path);
                    let target_file = serde_json::from_str::<TargetFile>(path.as_str()).unwrap();
                    println!("deserialized path: {:?}", target_file.path);
                    println!("event.kind: {:?}", event.kind);
                    match event.kind {
                        // Linux だと、Modify の中に ModifyKind がある構造
                        // Windows だと、 Modify の中に Any がある構造
                        // 両方で 1 度だけ発火させるために ModifyKind
                        // がある場合には何もしないようにしている。
                        EventKind::Modify(ModifyKind::Data(_)) => {}
                        EventKind::Modify(_) => {
                            println!("Change: {:?}", path);

                            let file_contents = get_file_content(&target_file.path);
                            println!("file_contents: {:?}", file_contents);

                            app_handle.emit_all("update_md", file_contents).unwrap();
                        }
                        _ => {}
                    }
                }
                Err(error) => println!("Error: {error:?}"),
            }
        }
    });

    // ファイル監視終了イベント受信ループ
    // このスレッドが終了しないようにループを作り、
    loop {
        // stop_rx チャンネルをチェックしてシグナルを受信したらループを終了
        if stop_rx.recv().is_ok() {
            println!("Shutting down watcher.");

            // チャンネルの停止依頼を空にする
            loop {
                if !stop_rx.try_recv().is_ok() {
                    break;
                }
            }

            break;
        }
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let file_path = parse_args(&app).unwrap();
            let app_handle = Arc::new(Mutex::new(app.handle()));

            // シミュレーション用の送受信チャンネル
            let (stop_tx, stop_rx) = std::sync::mpsc::channel();
            let stop_rx = Arc::new(Mutex::new(stop_rx));

            if file_path != "" {
                let app_handle = Arc::clone(&app_handle);
                let stop_rx = Arc::clone(&stop_rx);
                thread::spawn(move || {
                    let app_handle_lock = app_handle.lock().unwrap();
                    let stop_rx_lock = stop_rx.lock().unwrap();
                    start_watch(&app_handle_lock, &file_path, &stop_rx_lock).unwrap();
                });
            };

            // グローバルリスナーの設定
            app.listen_global("start_watch", move |event| {
                let app_handle = Arc::clone(&app_handle);
                let stop_rx = Arc::clone(&stop_rx);
                println!("start_watch");
                thread::spawn(move || {
                    let app_handle_lock = app_handle.lock().unwrap();
                    let stop_rx_lock = stop_rx.lock().unwrap();
                    let file_path = event.payload().unwrap().to_string();
                    let target_file = serde_json::from_str::<TargetFile>(&file_path).unwrap();
                    start_watch(&app_handle_lock, &target_file.path, &stop_rx_lock).unwrap();
                });
            });

            app.listen_global("stop_watch", move |_| {
                stop_tx.send(()).unwrap();
                println!("stoped.");
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 戻り値: Markdown ファイルパス
fn parse_args(app: &App) -> Result<String, Box<tauri::Error>> {
    match app.get_cli_matches() {
        Ok(matches) => {
            // ヘルプの表示
            if let Some(x) = matches.args.get("help") {
                println!("{}", x.value.as_str().unwrap());
                process::exit(0);
            }

            // バージョンの表示
            if let Some(_) = matches.args.get("version") {
                let version = app.config().package.version.clone();
                println!("{}", version.unwrap());
                process::exit(0);
            }

            // ひとまず matches 確認
            println!("{:?}", matches);

            // args を取得
            let args = matches.args;

            // ファイルパスを取得して返却
            let file_path = match get_value(&args, "file").as_str() {
                Some(value) => value.to_string(),
                _ => "".to_string(),
            };

            Ok(file_path)
        }
        Err(err) => {
            // エラー時はエラーを表示した終了
            println!("{:?}", err);
            Err(Box::new(err))
        }
    }
}

// args から value を取得するための関数
fn get_value(args: &HashMap<String, ArgData>, key: &str) -> value::Value {
    let option_arg_data = args.get(key);
    let option_data_wraped = option_arg_data.unwrap();
    let option_value = &option_data_wraped.value;

    return option_value.clone();
}

fn get_file_content(path: &str) -> String {
    println!("{:?}", path);
    let mut file = File::open(path).unwrap();
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).unwrap();
    return file_contents;
}
