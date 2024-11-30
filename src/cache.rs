use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs};

pub trait PersistentCacheProvider {
    fn save(&mut self, key: (i32, i32), text: String);

    fn save_example(&mut self, key: (i32, i32, i32), html: String);

    fn save_throttle_timestamp(&mut self, timestamp: SystemTime);

    fn load(&self, key: (i32, i32)) -> Option<String>;

    fn load_example(&self, key: (i32, i32, i32)) -> Option<String>;

    fn load_throttle_timestamp(&self) -> Option<SystemTime>;

    fn save_all(
        &mut self,
        real: &HashMap<(i32, i32), String>,
        examples: &HashMap<(i32, i32, i32), String>,
        throttle_timestamp: SystemTime,
    ) {
        self.save_throttle_timestamp(throttle_timestamp);
        for (key, text) in real {
            self.save(*key, text.clone());
        }
        for (key, val) in examples {
            self.save_example(*key, val.clone());
        }
    }
}

pub struct FileCacheProvider {
    cache_dir: PathBuf,
}

impl FileCacheProvider {
    pub fn new() -> Self {
        Self {
            cache_dir: env::var("LIBAOC_CACHE_DIRECTORY")
                .ok()
                .map(|d| PathBuf::from(d))
                .or(dirs::cache_dir())
                .expect("Could not find a cache directory for inputs!\nSpecify a directory in the LIBAOC_CACHE_DIRECTORY environment variable."),
        }
    }

    pub fn new_with_dir<P: Into<PathBuf>>(dir: P) -> Self {
        Self {
            cache_dir: dir.into(),
        }
    }
}

impl PersistentCacheProvider for FileCacheProvider {
    fn save(&mut self, key: (i32, i32), text: String) {
        let (year, day) = key;
        let dir = self.cache_dir.join(format!("libaoc/{year}"));
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("libaoc: warning: failed to create directory for caching: {e}");
            return;
        }
        let file = self.cache_dir.join(format!("libaoc/{year}/{day}.txt"));
        if let Err(e) = fs::write(file, text) {
            eprintln!("libaoc: warning: failed to save cache file: {e}");
        }
    }

    fn save_example(&mut self, key: (i32, i32, i32), html: String) {
        let (year, day, part) = key;
        let dir = self.cache_dir.join(format!("libaoc/examples/{year}"));
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("libaoc: warning: failed to create directory for caching: {e}");
        }
        let file = self
            .cache_dir
            .join(format!("libaoc/examples/{year}/{day}_{part}.html"));
        if let Err(e) = fs::write(file, html) {
            eprintln!("libaoc: warning: failed to save cache file: {e}");
        }
    }

    fn save_throttle_timestamp(&mut self, timestamp: SystemTime) {
        let dir = self.cache_dir.join("libaoc");
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("libaoc: warning: failed to create directory for caching: {e}");
        }
        let file = self.cache_dir.join("libaoc/throttle_timestamp");
        if let Err(e) = fs::write(
            file,
            timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap_or(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Your system time is earlier than the UNIX epoch!"),
                )
                .as_secs_f64()
                .to_string(),
        ) {
            eprintln!("libaoc: warning: failed to save throttle timestamp: {e}");
        }
    }

    fn load(&self, key: (i32, i32)) -> Option<String> {
        let (year, day) = key;
        let file = self.cache_dir.join(format!("libaoc/{year}/{day}.txt"));
        if file.exists() {
            fs::read_to_string(file).ok()
        } else {
            None
        }
    }

    fn load_example(&self, key: (i32, i32, i32)) -> Option<String> {
        let (year, day, part) = key;
        let file = self
            .cache_dir
            .join(format!("libaoc/examples/{year}/{day}_{part}.html"));
        if file.exists() {
            fs::read_to_string(file).ok()
        } else {
            None
        }
    }

    fn load_throttle_timestamp(&self) -> Option<SystemTime> {
        let file = self.cache_dir.join("libaoc/throttle_timestamp");
        if file.exists() {
            match fs::read_to_string(file).ok() {
                Some(s) => Some(UNIX_EPOCH + Duration::from_secs_f64(f64::from_str(&s).ok()?)),
                None => None,
            }
        } else {
            None
        }
    }
}
