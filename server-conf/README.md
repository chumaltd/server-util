# server-conf

Reference implementation for loading server config from file and environmental variables.


Conditional config between Development and Test
------------

You can override config file with env `RUST_CONF_ENV`.  
If you set env as bellows, `cargo run` will load `config/development.toml`.

```rust
use std::env::set_var;

fn main() {
    if cfg!(debug_assertions) {
        set_var("RUST_CONF_ENV", "development");
    }
    
    // .. main code
}
```

Default `RUST_CONF_ENV` is set to `test`, and `config/test.toml` can be used for testing.  
`cargo test` doesn't invoke `main()`, so you can conditionally load development.toml / test.toml just with `cargo run` / `cargo test` respectively.
