use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Async, futures channel based event watching
pub(crate) async fn watch(file_map: Arc<Mutex<Vec<PathBuf>>>) {
    let path = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");
    println!("watching {}", path);

    futures::executor::block_on(async {
        if let Err(e) = async_watch(path, file_map).await {
            println!("error: {:?}", e)
        }
    });
}
async fn update_file_list(dir: &Path, directories: Arc<Mutex<Vec<PathBuf>>>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                directories.lock().await.push(entry.path().to_path_buf());
            }
        }
    } else {
        println!("Failed to read directory: {:?}", dir);
    }
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch<P: AsRef<Path>>(
    path: P,
    file_map: Arc<Mutex<Vec<PathBuf>>>,
) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
    let mut rename_map = HashMap::new();
    update_file_list(path.as_ref(), Arc::clone(&file_map)).await;

    println!("directory contents: {:?}", file_map);
    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => handle_event(event, &mut rename_map, Arc::clone(&file_map)).await,
            Err(e) => println!("watch error: {:?}", e),
        };
        println!("directory contents: {:?}", file_map);
    }

    Ok(())
}
// Function to handle folder creation and renaming events
async fn handle_event(
    event: Event,
    rename_map: &mut HashMap<PathBuf, PathBuf>,
    directories: Arc<Mutex<Vec<PathBuf>>>,
) {
    match event.kind {
        EventKind::Create(_) => {
            for path in event.paths {
                if path.is_dir() {
                    println!("Folder created: {:?}", path);
                    directories.lock().await.push(path.clone());
                }
            }
        }
        EventKind::Modify(_) => {
            for path in &event.paths {
                if let Some(old_path) = rename_map.values().next().cloned() {
                    println!("Folder renamed from: {:?} to {:?}", old_path, path);
                    let mut dirs = directories.lock().await;
                    if let Some(index) = dirs.iter().position(|x| *x == old_path) {
                        dirs[index] = path.clone();
                    }
                    rename_map.remove(&old_path);
                } else {
                    rename_map.insert(path.clone(), path.clone());
                }
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                println!("Folder removed: {:?}", path);
                let mut dirs = directories.lock().await;
                dirs.retain(|x| *x != path);
            }
        }
        _ => {
            println!("Match failed: {:?}", event.kind);
        }
    }
}
