## Pitfalls

- **`gen` is a reserved keyword in Rust edition 2024** — `rand::thread_rng().gen()` fails to parse. Use `rand::random()` instead.
- **Topic validation requires `facts/*.md` to exist** — `alzai log` rejects unknown topics by checking `facts/<topic>.md` existence. To create a new topic, touch the file first (an empty file is enough).
