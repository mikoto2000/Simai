// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::{path::Path, sync::mpsc};
use std::{process, thread};

use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher};
use serde_json::value;
use tauri::api::cli::ArgData;

use tauri::{App, AppHandle, Manager};

fn start_watch(app_handle: &AppHandle, file_path: &str, stop_rx: &mpsc::Receiver<()>) -> notify::Result<()> {
    // チャンネルの停止依頼を空にする
    loop {
        if !stop_rx.try_recv().is_ok() {
            break;
        }
    }

    let path = Path::new(file_path).as_ref();
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .unwrap();
    watcher.watch(path, RecursiveMode::NonRecursive).unwrap();

    // 監視イベント受信処理処理スレッド
    let app_handle = app_handle.clone();
    thread::spawn(move || {
        while let Ok(res) = rx.recv() {
            match res {
                Ok(event) => {
                    let path = event.paths[0].clone();
                    match event.kind {
                        EventKind::Modify(ModifyKind::Data(_)) => {
                            println!("Change: {:?}", path);

                            let mut file = File::open(path).unwrap();
                            let mut file_contents = String::new();
                            file.read_to_string(&mut file_contents).unwrap();

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
        .setup(|app| {
            let file_path = parse_args(&app).unwrap();

            // シミュレーション用の送受信チャンネル
            let (stop_tx, stop_rx) = std::sync::mpsc::channel();

            let app_handle = app.handle();
            let ss = Arc::new(Mutex::new(move |file_path: &str| {
                let app_handle = app_handle.clone();
                start_watch(&app_handle, file_path, &stop_rx).unwrap();
            }));

            let stop_watch = {
                move || {
                    stop_tx.send(()).unwrap();
                    println!("stoped.");
                }
            };

            if file_path != "" {
                let ss = Arc::clone(&ss);
                thread::spawn(move || {
                    ss.lock().unwrap()(file_path.as_str());
                });
            };

            // グローバルリスナーの設定
            app.listen_global("start_watch", move |event| {
                let ss = Arc::clone(&ss);
                println!("start_watch");
                thread::spawn(move || {
                    let file_path = event.payload().unwrap().to_string();
                    let file_path = serde_json::from_str::<&str>(&file_path).unwrap();

                    ss.lock().unwrap()(file_path);
                });
            });

            app.listen_global("stop_watch", {
                println!("stop_watch");
                move |_| {
                    stop_watch();
                }
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
