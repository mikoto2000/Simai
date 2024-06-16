// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{path::Path, sync::mpsc};
use std::{process, thread};

use notify::event::ModifyKind;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::value;
use tauri::api::cli::ArgData;

use tauri::{App, Manager};

fn start_watch(file_path: &str, stop_rx: MutexGuard<mpsc::Receiver<()>>) -> notify::Result<()> {
    let path = Path::new(file_path).as_ref();
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
    watcher.watch(path, RecursiveMode::NonRecursive).unwrap();

    // 監視イベント受信処理処理スレッド
    thread::spawn(move || {
        while let Ok(res) = rx.recv() {
            match res {
                Ok(event) => match event.kind {
                    EventKind::Modify(ModifyKind::Data(data)) => {
                        println!("Change: {data:?}");
                    }
                    _ => {}
                },
                Err(error) => println!("Error: {error:?}"),
            }
        }
    });

    // ファイル監視終了イベント受信ループ
    // このスレッドが終了しないようにループを作り、
    loop {
        // stop_rx チャンネルをチェックしてシグナルを受信したらループを終了
        if stop_rx.try_recv().is_ok() {
            println!("Shutting down watcher.");
            break;
        }
        println!("watching...");
        thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let file_path = parse_args(&app).unwrap();

            // シミュレーション用の送受信チャンネル
            let (stop_tx, stop_rx) = std::sync::mpsc::channel();
            let stop_tx_arc = Arc::new(Mutex::new(stop_tx));
            let stop_rx_arc = Arc::new(Mutex::new(stop_rx));

            let ss = {
                let stop_rx_arc = Arc::clone(&stop_rx_arc);
                Arc::new(Mutex::new(move |file_path: Box<&str>| {
                    start_watch(*file_path, stop_rx_arc.lock().unwrap()).unwrap();
                }))
            };

            let ss2 = {
                let stop_rx_arc = Arc::clone(&stop_rx_arc);
                Arc::new(Mutex::new(move |file_path: Box<&str>| {
                    start_watch(*file_path, stop_rx_arc.lock().unwrap()).unwrap();
                }))
            };

            let stop_watch = {
                let stop_tx_arc = Arc::clone(&stop_tx_arc);
                Arc::new(Mutex::new(move || {
                    stop_tx_arc.lock().unwrap().send(()).unwrap();
                }))
            };

            if file_path != "" {
                let ss = Arc::clone(&ss);
                thread::spawn(move || {
                    let ss = ss.lock();
                    let ss = ss.unwrap();
                    ss(Box::from(file_path.as_str()));
                });
            }

            // グローバルリスナーの設定
            app.listen_global("start_watch", move |event| {
                let ss = Arc::clone(&ss);
                thread::spawn(move || {
                    let file_path = event.payload().unwrap().to_string();

                    let file_path = serde_json::from_str::<&str>(&file_path).unwrap();

                    let ss = ss.lock();
                    let ss = ss.unwrap();
                    ss(Box::from(file_path));
                });
            });

            app.listen_global("stop_watch", {
                let stop_watch = Arc::clone(&stop_watch);
                move |_| {
                    stop_watch.lock().unwrap()();
                }
            });

            app.listen_global("selected_file", move |event| {
                let ss2 = Arc::clone(&ss2);
                let stop_watch = Arc::clone(&stop_watch);
                let _event_payload = event.payload();
                stop_watch.lock().unwrap()();

                thread::spawn(move || {
                    let file_path = event.payload().unwrap().to_string();

                    let file_path = serde_json::from_str::<&str>(&file_path).unwrap();

                    let ss2 = ss2.lock();
                    let ss2 = ss2.unwrap();
                    ss2(Box::from(file_path));
                });
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
