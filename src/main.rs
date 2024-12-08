mod watch;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    println!("About to start watching...");

    // let mut file_map = HashMap::new();
    let directories: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    // Start watching the folder in a separate task
    let directories_clone = Arc::clone(&directories);
    tokio::spawn(async move {
        watch::watch::watch(directories_clone).await;
    });
    // Example of accessing the directories list
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let dirs = directories.lock().await;
        println!("Main thread directories: {:?}", *dirs);
    }
}
