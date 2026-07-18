use std::path::PathBuf;

use chiaro_telemetry::{LoadedIbt, load_ibt};

pub async fn select_file() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Open iRacing telemetry")
        .add_filter("iRacing telemetry", &["ibt"])
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}

pub async fn load(path: PathBuf) -> Result<LoadedIbt, String> {
    smol::unblock(move || load_ibt(&path)).await
}
