# libaoc

A simple Rust library for retrieving Advent of Code input data. Used by my Advent of Code 2023 solutions.

## Guidelines compliance

This library complies with the [Advent of Code automation guidelines](https://old.reddit.com/r/adventofcode/wiki/faqs/automation/):

* Input data is cached locally
  * An in-memory cache is always used; a persistent file cache is enabled by default (users can also implement their own persistent cache by implementing the `PersistentCacheProvider` trait)
  * If you suspect the cache is corrupted, request a new copy with `AocClient::get_input_without_cache`
  * The in-memory cache is saved to disk when the `AocClient` is dropped
* Requests are throttled to 1 every 3 minutes
  * If a user requests input that would result in a request being made sooner than 3 minutes after the previous request, the function blocks until 3 minutes has elapsed since the previous request
  * When the `AocClient` is dropped, the timestamp of the last request is saved to disk and is loaded again when a new `AocClient` is created
* The user agent is set to `libaoc/<version> (automated; +https://github.com/arthomnix/libaoc; +<contact email>) reqwest/0.11`
  * The contact email has been redacted (and slightly obfuscated in the source) to stop it getting picked up by spambots; it is always the same