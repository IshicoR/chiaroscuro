use std::{env, error::Error, io};

use chiaroscuro_irsdk::IbtFile;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args_os().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo run -p chiaroscuro-irsdk --example ibt_summary -- <file.ibt>",
        )
    })?;
    let mut source = IbtFile::open(&path)?;
    let metadata = *source.metadata();
    let track = source
        .session_info()
        .parse()
        .ok()
        .and_then(|session| session.weekend_info)
        .and_then(|weekend| weekend.track_display_name.or(weekend.track_name))
        .unwrap_or_else(|| "unknown".to_owned());

    println!("file: {}", path.to_string_lossy());
    println!("track: {track}");
    println!("variables: {}", source.variables().len());
    println!("records: {}", metadata.record_count);
    println!("tick rate: {} Hz", metadata.tick_rate);
    println!("laps: {}", metadata.lap_count);
    println!("duration: {:.3} s", metadata.duration_seconds());

    if !source.is_empty() {
        let first = source.read_value(0, "SessionTime")?;
        let last = source.read_value(source.len() - 1, "SessionTime")?;
        let first_snapshot = source.read_snapshot(0)?;
        let last_snapshot = source.read_snapshot(source.len() - 1)?;
        println!("SessionTime: {first:?} .. {last:?}");
        println!(
            "speed: {:.1} .. {:.1} km/h",
            first_snapshot.sample.speed_kmh, last_snapshot.sample.speed_kmh
        );
    }

    Ok(())
}
