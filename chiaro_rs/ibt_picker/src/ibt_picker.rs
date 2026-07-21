use chiaro_telemetry::{LoadedIbt, RecordingSource, load_ibt_source};

pub async fn select_file() -> Option<RecordingSource> {
    rfd::AsyncFileDialog::new()
        .set_title("Open iRacing telemetry")
        .add_filter("iRacing telemetry", &["ibt"])
        .pick_file()
        .await
        .map(|file| RecordingSource::local_file(file.path()))
}

pub async fn load(source: RecordingSource) -> Result<LoadedIbt, String> {
    smol::unblock(move || load_ibt_source(&source)).await
}
