// No key lives here. This file holds the ONE macro that decides whether a
// compile-time key is allowed into the binary at all, and in a release build
// the answer is no.
//
// `make appstore` and `make desktop-app` source `.env` before building, so a
// bare `option_env!("OPENAI_API_KEY")` bakes the developer's own key into the
// binary handed to testers and to the App Store. Two consequences, both
// unacceptable: every tester's transcriptions and embeddings bill to that key,
// and anyone can pull it back out of the app.
//
// The `#[cfg]` is what makes this real rather than cosmetic. A runtime `if`
// would still leave the literal in the binary, because both branches compile;
// `#[cfg(debug_assertions)]` deletes the `option_env!` expansion outright, so a
// release build has no key to find.
//
// A release build therefore resolves keys from SQLite settings or the runtime
// environment only: the user enters their own.

#[macro_export]
macro_rules! baked_key {
    ($name:literal) => {{
        #[cfg(debug_assertions)]
        {
            option_env!($name).map(String::from)
        }
        #[cfg(not(debug_assertions))]
        {
            None::<String>
        }
    }};
}
