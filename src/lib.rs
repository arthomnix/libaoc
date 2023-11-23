//! Simple library for obtaining Advent of Code inputs.
//!
//! This requires an AOC session token - this is the value of the `session` cookie when logged
//! in to the Advent of Code website. This must be from the same account as you are using to submit
//! solutions as each account has different input which produces a different result.

pub struct AocClient {
    session: String,
    client: reqwest::blocking::Client,
}

impl AocClient {
    /// Create an `AocClient` using the session token stored in the environment variable `AOC_SESSION`.
    pub fn new_from_env() -> Self {
        Self::new(env!("AOC_SESSION").to_string())
    }

    /// Create an `AocClient` using the given session token.
    pub fn new(session: String) -> Self {
        AocClient {
            session,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Get the input text for the Advent of Code puzzle for the given day and year.
    pub fn get_input(&self, year: i32, day: i32) -> reqwest::Result<String> {
        self.client
            .get(format!("https://adventofcode.com/{year}/day/{day}/input"))
            .header("Cookie", format!("session={}", self.session))
            .send()
            .and_then(|r| r.text())
    }
}

#[cfg(test)]
mod test {
    use crate::AocClient;

    #[test]
    fn test_aoc() {
        let client = AocClient::new(env!("AOC_SESSION").to_string());
        let res = client.get_input(2022, 25);
        assert!(res.is_ok());
        println!("{}", res.unwrap());
    }
}
