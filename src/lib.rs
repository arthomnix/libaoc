//! Simple library for obtaining Advent of Code inputs.
//!
//! This requires an AOC session token - this is the value of the `session` cookie when logged
//! in to the Advent of Code website. This must be from the same account as you are using to submit
//! solutions as each account has different input which produces a different result.
//!
//! Requests are cached and throttled to one every 3 minutes in accordance with the Advent of Code
//! automation guidelines. If for whatever reason the default file caching implementation isn't
//! suitable for your use case, disable the `file_cache` feature and implement your own caching
//! system.

#[cfg(feature = "file_cache")]
use dirs::cache_dir;
#[cfg(feature = "file_cache")]
use std::fs::{create_dir_all, read_to_string, write};
#[cfg(feature = "file_cache")]
use std::path::PathBuf;
#[cfg(feature = "file_cache")]
use std::str::FromStr;

use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static MIN_TIME_BETWEEN_REQUESTS: Duration = Duration::from_secs(180);

pub struct AocClient {
    session: String,
    client: reqwest::blocking::Client,

    #[cfg(feature = "file_cache")]
    cache_dir: PathBuf,

    throttle_timestamp: SystemTime,
    mem_cache: HashMap<(i32, i32), String>,
}

impl AocClient {
    #[cfg(feature = "file_cache")]
    fn get_cache_file(&self, year: i32, day: i32) -> PathBuf {
        let dir = self.cache_dir.join(format!("libaoc/{year}"));
        create_dir_all(dir.clone()).expect("Could not create directory for cache!");
        dir.join(format!("{day}.txt"))
    }

    fn make_client() -> reqwest::blocking::Client {
        let user_agent = format!(
            "libaoc/{0} (automated; +https://github.com/arthomnix/libaoc; +{3}-{2}@{1}.dev{4}) reqwest/0.11",
            env!("CARGO_PKG_VERSION"),
            "arthomnix", "contact", "libaoc",
            if cfg!(not(feature = "file_cache")) { "; builtin caching disabled by user" } else { "" }
        );

        reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .build()
            .unwrap()
    }

    /// Create an `AocClient` using the session token stored in the environment variable `AOC_SESSION`.
    pub fn new_from_env() -> Self {
        Self::new(std::env::var("AOC_SESSION").expect("AOC_SESSION environment variable not found!").to_string())
    }

    /// Create an `AocClient` using the given session token and the default cache directory.
    pub fn new(session: String) -> Self {
        #[cfg(not(feature = "file_cache"))]
        {
            eprintln!("libaoc: warning: persistent cache disabled - make sure to implement your own cache, or reenable the file_cache feature!");
            return AocClient {
                session,
                client: Self::make_client(),
                throttle_timestamp: UNIX_EPOCH,
                mem_cache: HashMap::new(),
            };
        }

        #[cfg(feature = "file_cache")]
        {
            let cache_dir = std::env::var("LIBAOC_CACHE_DIRECTORY").ok()
                .map(|d| PathBuf::from(d))
                .or(cache_dir());
            if let Some(cache_dir) = cache_dir {
                return Self::new_with_custom_cache(session, cache_dir);
            } else {
                panic!("Could not find a cache directory for inputs!\nSpecify a directory in the AOC_CACHE_DIRECTORY environment variable, or disable the cache feature **and implement your own caching**.");
            }
        }
    }

    /// Create an `AocClient` using the given session token and cache directory.
    #[cfg(feature = "file_cache")]
    pub fn new_with_custom_cache<P: Into<PathBuf>>(session: String, cache_dir: P) -> Self {
        let cache_dir = cache_dir.into();
        let mut throttle_timestamp = UNIX_EPOCH;
        if cache_dir.join("libaoc/throttle_timestamp").exists() {
            if let Ok(text) = read_to_string(cache_dir.join("libaoc/throttle_timestamp")) {
                if let Ok(timestamp) = f64::from_str(&text) {
                    throttle_timestamp += Duration::from_secs_f64(timestamp);
                }
            }
        }

        AocClient {
            session,
            cache_dir: cache_dir.into(),
            client: Self::make_client(),
            throttle_timestamp,
            mem_cache: HashMap::new(),
        }
    }

    /// Get the input text for the Advent of Code puzzle for the given day and year, bypassing the cache.
    /// Only use this if you believe the cached input is corrupted.
    pub fn get_input_without_cache(&mut self, year: i32, day: i32) -> reqwest::Result<String> {
        let throttle_duration = SystemTime::now().duration_since(self.throttle_timestamp);
        if throttle_duration
            .as_ref()
            .is_ok_and(|d| *d < MIN_TIME_BETWEEN_REQUESTS)
        {
            let sleep_duration = MIN_TIME_BETWEEN_REQUESTS - throttle_duration.unwrap();
            eprintln!(
                "libaoc: request throttled - sleeping for {}s",
                sleep_duration.as_secs_f64()
            );
            sleep(sleep_duration);
        } else if throttle_duration.is_err() {
            eprintln!("libaoc: warning: received SystemTimeError while processing throttle, sleeping for 1 second and retrying...");
            sleep(Duration::from_secs(1));
            return self.get_input_without_cache(year, day);
        }

        self.throttle_timestamp = SystemTime::now();

        let text = self
            .client
            .get(format!("https://adventofcode.com/{year}/day/{day}/input"))
            .header("Cookie", format!("session={}", self.session))
            .send()
            .and_then(|r| r.text());

        if let Ok(text) = &text {
            self.mem_cache.insert((year, day), text.clone());
        }

        text
    }

    /// Get the input text for the Advent of Code puzzle for the given day and year, bypassing the file cache but using any value in the in-memory cache.
    /// Only use this if you believe the file cache is corrupted.
    ///
    /// Equivalent to `get_input` if the `file_cache` feature is disabled.
    pub fn get_input_without_file_cache(&mut self, year: i32, day: i32) -> reqwest::Result<String> {
        let cached = self.mem_cache.get(&(year, day));
        if let Some(cached) = cached {
            Ok(cached.clone())
        } else {
            self.get_input_without_cache(year, day)
        }
    }

    /// Get the input text for the Advent of Code puzzle for the given day and year.
    pub fn get_input(&mut self, year: i32, day: i32) -> reqwest::Result<String> {
        #[cfg(not(feature = "file_cache"))]
        return self.get_input_without_file_cache(year, day);

        #[cfg(feature = "file_cache")]
        {
            let file = self.get_cache_file(year, day);
            if file.exists() {
                if let Ok(text) = read_to_string(file) {
                    return Ok(text);
                }
            }

            self.get_input_without_file_cache(year, day)
        }
    }
}

#[cfg(feature = "file_cache")]
impl Drop for AocClient {
    fn drop(&mut self) {
        write(
            self.cache_dir.join("libaoc/throttle_timestamp"),
            self.throttle_timestamp
                .duration_since(UNIX_EPOCH)
                .unwrap_or(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Your system time is earlier than the UNIX epoch!"),
                )
                .as_secs_f64()
                .to_string(),
        )
        .unwrap_or_else(|_| eprintln!("libaoc: warning: failed to save throttle timestamp"));

        for ((year, day), input) in &self.mem_cache {
            write(self.get_cache_file(*year, *day), input).unwrap_or_else(|_| {
                eprintln!("libaoc: warning: failed to cache input for year {year} day {day}")
            });
        }
    }
}

#[cfg(test)]
mod test {
    use crate::AocClient;

    #[test]
    fn test_aoc() {
        let mut client = AocClient::new_from_env();
        let res = client.get_input(2022, 25);
        assert!(res.is_ok());
        println!("{}", res.unwrap());
    }
}
