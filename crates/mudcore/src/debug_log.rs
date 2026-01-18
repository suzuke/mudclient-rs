use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use std::path::PathBuf;

lazy_static::lazy_static! {
    static ref DEBUG_LOGGER: Mutex<Option<DebugLogger>> = Mutex::new(None);
}

pub struct DebugLogger {
    file: File,
}

impl DebugLogger {
    pub fn init(path: PathBuf) {
        if let Ok(file) = File::create(path) {
            let mut logger = DEBUG_LOGGER.lock().unwrap();
            *logger = Some(DebugLogger { file });
        }
    }

    pub fn log(msg: &str) {
        let mut logger = DEBUG_LOGGER.lock().unwrap();
        if let Some(l) = logger.as_mut() {
            let _ = writeln!(l.file, "[{}] {}", chrono_lite_timestamp(), msg);
        }
    }

    pub fn log_bytes(prefix: &str, bytes: &[u8]) {
        let hex = bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        Self::log(&format!("{}: [{}]", prefix, hex));
    }
}

fn chrono_lite_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.to_string()
}
