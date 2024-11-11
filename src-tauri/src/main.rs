#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{path::Path, sync::mpsc};
use std::{process, thread};

use notify::event::ModifyKind;
use notify::{Config, EventKind, PollWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::value;
use tauri::api::cli::ArgData;

use tauri::{App, AppHandle, Manager};

#[derive(Deserialize)]
struct TargetFile {
    path: String,
}

#[derive(Clone, Serialize)]
struct UpdateFile {
    path: String,
    content: String,
}

#[derive(Deserialize)]
struct TcpConfig {
    address: String,
    port: u16,
}

fn start_watch(
    app_handle: Arc<Mutex<AppHandle>>,
    file_path: &str,
    stop_rx: &mpsc::Receiver<()>,
    event_name: &str,
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
    let app_handle_lock = app_handle.lock().unwrap();
    let emit_object = UpdateFile {
        path: file_path.to_string(),
        content: file_contents.to_string(),
    };
    app_handle_lock.emit_all(event_name, emit_object).unwrap();
    drop(app_handle_lock);

    // 監視イベント受信処理処理スレッド
    {
        let event_name = event_name.to_string();
        let file_path = file_path.to_string().clone();
        thread::spawn(move || {
            let event_name = event_name.clone();
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
                        let target_file =
                            serde_json::from_str::<TargetFile>(path.as_str()).unwrap();
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

                                let app_handle_lock = app_handle.lock().unwrap();
                                let emit_object = UpdateFile {
                                    path: file_path.to_string(),
                                    content: file_contents.to_string(),
                                };
                                app_handle_lock
                                    .emit_all(&event_name, emit_object)
                                    .unwrap();
                                drop(app_handle_lock);
                            }
                            _ => {}
                        }
                    }
                    Err(error) => println!("Error: {error:?}"),
                }
            }
        });
    }

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

fn handle_client(stream: TcpStream, app_handle: Arc<Mutex<AppHandle>>, address: String, port: u16) {
    let mut buffer = String::new();
    let mut reader = std::io::BufReader::new(stream);
    reader.read_to_string(&mut buffer).unwrap();

    let emit_object = UpdateFile {
        path: "tcp".to_string(),
        content: buffer.clone(),
    };

    let app_handle_lock = app_handle.lock().unwrap();
    app_handle_lock.emit_all("update_md_tcp", emit_object).unwrap();
    drop(app_handle_lock);

    // Restart TCP listener on the same port after receiving a Markdown string
    let app_handle = Arc::clone(&app_handle);
    thread::spawn(move || {
        start_tcp_server(app_handle, address, port);
    });
}

fn start_tcp_server(
    app_handle: Arc<Mutex<AppHandle>>,
    address: String,
    port: u16,
) {
    let listener = TcpListener::bind(format!("{}:{}", address, port)).unwrap();
    println!("TCP server listening on {}:{}", address, port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let app_handle = Arc::clone(&app_handle);
                let address = address.clone();
                let port = port;
                thread::spawn(move || {
                    handle_client(stream, app_handle, address, port);
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
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

            // TCP server stop channel
            let (stop_tcp_tx, stop_tcp_rx) = std::sync::mpsc::channel();
            let stop_tcp_rx = Arc::new(Mutex::new(stop_tcp_rx));

            {
                let app_handle = Arc::clone(&app_handle);
                let stop_rx = Arc::clone(&stop_rx);
                thread::spawn(move || {
                    if file_path != "" {
                        let app_handle = Arc::clone(&app_handle);
                        let stop_rx = Arc::clone(&stop_rx);
                        thread::spawn(move || {
                            let stop_rx_lock = stop_rx.lock().unwrap();
                            start_watch(app_handle, &file_path, &stop_rx_lock, "update_md")
                                .unwrap();
                        });
                    };
                });
            }

            // グローバルリスナーの設定
            {
                let app_handle = Arc::clone(&app_handle);
                let stop_rx = Arc::clone(&stop_rx);
                let stop_tcp_tx = stop_tcp_tx.clone();
                app.listen_global("start_watch_md", move |event| {
                    let app_handle = Arc::clone(&app_handle);
                    let stop_rx = Arc::clone(&stop_rx);
                    let stop_tcp_tx = stop_tcp_tx.clone();
                    println!("start_watch_md");
                    thread::spawn(move || {
                        let stop_rx_lock = stop_rx.lock().unwrap();
                        let file_path = event.payload().unwrap().to_string();
                        let target_file = serde_json::from_str::<TargetFile>(&file_path).unwrap();
                        start_watch(app_handle, &target_file.path, &stop_rx_lock, "update_md")
                            .unwrap();
                        stop_tcp_tx.send(()).unwrap();
                    });
                });
            }

            app.listen_global("stop_watch_md", move |_| {
                stop_tx.send(()).unwrap();
                println!("stoped.");
            });

            let (stop_tx_css, stop_rx_css) = std::sync::mpsc::channel();
            {
                let app_handle = Arc::clone(&app_handle);
                let stop_rx_css = Arc::new(Mutex::new(stop_rx_css));
                let stop_tcp_tx = stop_tcp_tx.clone();
                app.listen_global("start_watch_css", move |event| {
                    let app_handle = Arc::clone(&app_handle);
                    let stop_rx_css = Arc::clone(&stop_rx_css);
                    let stop_tcp_tx = stop_tcp_tx.clone();
                    thread::spawn(move || {
                        let stop_rx_lock = stop_rx_css.lock().unwrap();
                        let file_path = event.payload().unwrap().to_string();
                        let target_file = serde_json::from_str::<TargetFile>(&file_path).unwrap();
                        start_watch(app_handle, &target_file.path, &stop_rx_lock, "update_css")
                            .unwrap();
                        stop_tcp_tx.send(()).unwrap();
                    });
                });

                app.listen_global("stop_watch_css", move |_| {
                    stop_tx_css.send(()).unwrap();
                    println!("stoped.");
                });
            }

            // Start TCP server
            {
                let app_handle = Arc::clone(&app_handle);
                let stop_tcp_rx = Arc::clone(&stop_tcp_rx);
                thread::spawn(move || {
                    start_tcp_server(app_handle, "127.0.0.1".to_string(), 7878);
                });
            }

            app.listen_global("start_tcp_listener", move |event| {
                let app_handle = Arc::clone(&app_handle);
                let stop_tcp_rx = Arc::clone(&stop_tcp_rx);
                let tcp_config: TcpConfig = serde_json::from_str(event.payload().unwrap()).unwrap();
                thread::spawn(move || {
                    start_tcp_server(app_handle, tcp_config.address, tcp_config.port);
                });
            });

            app.listen_global("stop_tcp_listener", move |_| {
                stop_tcp_tx.send(()).unwrap();
                println!("TCP listener stopped.");
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
