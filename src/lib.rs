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

mod cache;

use crate::cache::{FileCacheProvider, PersistentCacheProvider};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static MIN_TIME_BETWEEN_REQUESTS: Duration = Duration::from_secs(180);

pub struct AocClient<C: PersistentCacheProvider> {
    session: String,
    client: reqwest::blocking::Client,
    throttle_timestamp: SystemTime,
    mem_cache: HashMap<(i32, i32), String>,
    persistent_cache: C,
}

impl AocClient<FileCacheProvider> {
    /// Create an `AocClient` using the session token stored in the environment variable `AOC_SESSION`.
    pub fn new_from_env() -> Self {
        Self::new(
            std::env::var("AOC_SESSION")
                .expect("AOC_SESSION environment variable not found!")
                .to_string(),
        )
    }

    /// Create an `AocClient` using the given session token and the default cache directory.
    pub fn new(session: String) -> Self {
        Self::new_with_custom_cache(session, FileCacheProvider::new())
    }
}

impl<C: PersistentCacheProvider> AocClient<C> {
    fn make_client() -> reqwest::blocking::Client {
        let user_agent = format!(
            "libaoc/{0} (automated; +https://github.com/arthomnix/libaoc; +{3}-{2}@{1}.dev) reqwest/0.11",
            env!("CARGO_PKG_VERSION"),
            "arthomnix", "contact", "libaoc",
        );

        reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .build()
            .unwrap()
    }

    /// Create an `AocClient` using the given session token and cache directory.
    pub fn new_with_custom_cache(session: String, cache_provider: C) -> Self {
        let throttle_timestamp = cache_provider
            .load_throttle_timestamp()
            .unwrap_or(UNIX_EPOCH);

        AocClient {
            session,
            persistent_cache: cache_provider,
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
    pub fn get_input_without_persistent_cache(
        &mut self,
        year: i32,
        day: i32,
    ) -> reqwest::Result<String> {
        self.mem_cache
            .get(&(year, day))
            .map(|s| Ok(s.clone()))
            .unwrap_or_else(|| self.get_input_without_cache(year, day))
    }

    /// Get the input text for the Advent of Code puzzle for the given day and year.
    pub fn get_input(&mut self, year: i32, day: i32) -> reqwest::Result<String> {
        self.mem_cache
            .get(&(year, day))
            .map(|s| Ok(s.clone()))
            .or(self.persistent_cache.load((year, day)).map(|o| Ok(o)))
            .unwrap_or_else(|| self.get_input_without_cache(year, day))
    }
}

impl<C: PersistentCacheProvider> Drop for AocClient<C> {
    fn drop(&mut self) {
        self.persistent_cache
            .save_all(&self.mem_cache, self.throttle_timestamp);
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
