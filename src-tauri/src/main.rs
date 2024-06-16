// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::thread;
use std::{path::Path, sync::mpsc};

use notify::event::ModifyKind;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

fn watch(path: &Path, stop_rx: mpsc::Receiver<()>) -> notify::Result<()> {
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
    let path = Path::new("../README.md");

    tauri::Builder::default()
        .setup(move |_app| {
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            thread::spawn(|| {
                let _ = watch(path, stop_rx);
            });

            thread::spawn(move || {
                thread::sleep(std::time::Duration::from_secs(10));
                let _ = stop_tx.send(());
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
