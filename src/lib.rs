//! Simple library for obtaining Advent of Code inputs.
//!
//! This requires an AOC session token - this is the value of the `session` cookie when logged
//! in to the Advent of Code website. This must be from the same account as you are using to submit
//! solutions as each account has different input which produces a different result.
//!
//! Requests are cached and throttled to one every 3 minutes in accordance with the Advent of Code
//! automation guidelines. If for whatever reason the default file caching implementation isn't
//! suitable for your use case, create your own implementation of the `PersistentCacheProvider`
//! trait.

pub mod cache;
pub mod example_parse;

use crate::cache::{FileCacheProvider, PersistentCacheProvider};
use crate::example_parse::Example;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static MIN_TIME_BETWEEN_REQUESTS: Duration = Duration::from_secs(180);

pub struct AocClient<C: PersistentCacheProvider> {
    session: String,
    client: reqwest::blocking::Client,
    throttle_timestamp: SystemTime,
    mem_cache: HashMap<(i32, i32), String>,
    example_cache: HashMap<(i32, i32, i32), String>,
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
            "libaoc/{0} (automated; +https://github.com/arthomnix/libaoc; +{3}-{2}@{1}.dev) reqwest/0.12",
            env!("CARGO_PKG_VERSION"),
            "arthomnix", "contact", "libaoc",
        );

        reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .build()
            .unwrap()
    }

    fn throttle(&mut self) -> bool {
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
            self.throttle_timestamp = SystemTime::now();
            true
        } else if throttle_duration.is_err() {
            eprintln!("libaoc: warning: received SystemTimeError while processing throttle, sleeping for 1 second and retrying...");
            sleep(Duration::from_secs(1));
            false
        } else {
            true
        }
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
            example_cache: HashMap::new(),
        }
    }

    /// Get the input text for the Advent of Code puzzle for the given day and year, bypassing the cache.
    /// Only use this if you believe the cached input is corrupted.
    pub fn get_input_without_cache(&mut self, year: i32, day: i32) -> reqwest::Result<String> {
        if !self.throttle() {
            return self.get_input_without_cache(year, day);
        }

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
            .or_else(|| {
                self.persistent_cache.load((year, day)).map(|o| {
                    self.mem_cache.insert((year, day), o.clone());
                    Ok(o)
                })
            })
            .unwrap_or_else(|| self.get_input_without_cache(year, day))
    }

    /// Get the example input and (possibly unreliable) answer(s) for the given day and year, bypassing the cache.
    /// Only use this if you believe the cache is corrupted, or you have completed part 1 and want to get the example answer for part 2 (which is hidden before part 1 is complete)>.
    pub fn get_example_without_cache(
        &mut self,
        year: i32,
        day: i32,
        part: i32,
    ) -> reqwest::Result<Option<Example>> {
        if !self.throttle() {
            return self.get_example_without_cache(year, day, part);
        }

        let html = self
            .client
            .get(format!("https://adventofcode.com/{year}/day/{day}"))
            .header("Cookie", format!("session={}", self.session))
            .send()
            .and_then(|r| r.text());

        if let Ok(html) = &html {
            self.example_cache.insert((year, day, part), html.clone());
        }

        html.map(|html| Example::parse_example(html))
    }

    /// Get the example input and (possibly unreliable) answer(s) for the given day and year, bypassing the persistent cache but using the in-memory cache.
    /// Only use this if you believe the consistent cache is corrupted.
    pub fn get_example_without_persistent_cache(
        &mut self,
        year: i32,
        day: i32,
        part: i32,
    ) -> reqwest::Result<Option<Example>> {
        self.example_cache
            .get(&(year, day, part))
            .map(|s| Ok(Example::parse_example(s.clone())))
            .unwrap_or_else(|| self.get_example_without_cache(year, day, part))
    }

    /// Get the example input and (possibly unreliable) answer(s) for the given day and year.
    ///
    /// The `part` parameter is only used to cache the data for part 1 and part 2 separately (since
    /// the answer for part 2 will only be available once your account has completed part 1). All
    /// example data present in the HTML is returned regardless of the value of the parameter.
    pub fn get_example(
        &mut self,
        year: i32,
        day: i32,
        part: i32,
    ) -> reqwest::Result<Option<Example>> {
        self.example_cache
            .get(&(year, day, part))
            .map(|s| Ok(Example::parse_example(s.clone())))
            .or_else(|| {
                self.persistent_cache
                    .load_example((year, day, part))
                    .map(|o| {
                        self.example_cache.insert((year, day, part), o.clone());
                        Ok(Example::parse_example(o))
                    })
            })
            .unwrap_or_else(|| self.get_example_without_cache(year, day, part))
    }
}

impl<C: PersistentCacheProvider> Drop for AocClient<C> {
    fn drop(&mut self) {
        self.persistent_cache.save_all(
            &self.mem_cache,
            &self.example_cache,
            self.throttle_timestamp,
        );
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
        let example = client.get_example(2022, 25, 1);
        assert!(example.is_ok());
        let example = example.unwrap();
        assert!(example.is_some());
        println!("{}", res.unwrap());
        println!("{:?}", example.unwrap());
    }
}
