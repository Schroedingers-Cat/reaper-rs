# reaper-rs

Makes REAPER SDK accessible from Rust.

Consists of 3 layers:

1. Low-level API
    - Copied from bindings generated by `bindgen` from `reaper_plugin_functions.h`
    - Makes access of function pointers safe
    - Not a pleasure to work with because it still doesn't use Rust types
2. Medium-level API
    - Uses low-level API
    - Exposes the original REAPER SDK functions almost 1:1, so as closely as possible **but** contains 
      some improvements, like being able to deal with Rust strings instead of C-strings
    - Not very opinionated, just some obvious common-sense adjustments probably every Rust programmer
      would do in order to get the API closer to Rust (if you don't think so and want to suggest a
      different form of adjustment for the low-level API, please raise an issue) 
3. High-level API
    - Uses medium-level API
    - In some ways opinionated because it uses tools like rxRust to deal with events
    - In other ways just consequent because it reflects 1:1 the typical hierarchy of a REAPER project
      (Project → Track → Item)   
    - A pleasure to work with (in my opinion)
    - Integration tests use this
    
## Use

### REAPER plug-in

#### Scenario 1

- [ ] Provide an extension of this macro which allows to load just some functions  

```rust
use reaper_rs::{high_level_reaper_plugin};
use reaper_rs::high_level::Reaper;
use std::error::Error;
use c_str_macro::c_str;

#[reaper_plugin(email = "info@example.com")]
fn main() -> Result<(), Box<dyn Error>> {
    let reaper = Reaper::get();
    reaper.show_console_msg(c_str!("Hello world"));
    Ok(())
}
```

- Fastest way to get going
- Already has set up a `high_level::Reaper` with a sensible default configuration
    - All available REAPER functions loaded
    - File logger to home directory
    - ...
- Also installs panic hook (which you can still overwrite by calling `std::panic::set_hook()`)

#### Scenario 2

```rust
use reaper_rs::{reaper_plugin};
use reaper_rs::low_level::ReaperPluginContext;
use reaper_rs::high_level::Reaper;
use std::error::Error;
use c_str_macro::c_str;

#[low_level_reaper_plugin]
fn main(context: ReaperPluginContext) -> Result<(), Box<dyn Error>> {
    Reaper::with_all_functions_loaded(context)
        .setup();
    // TODO
    Ok(())
}
```

#### Scenario 3

- [ ] Add an example for loading just some functions

```rust
use reaper_rs::{reaper_plugin};
use reaper_rs::high_level::Reaper;
use reaper_rs::low_level::ReaperPluginContext;
use std::error::Error;
use c_str_macro::c_str;

#[low_level_reaper_plugin]
fn main(context: ReaperPluginContext) -> Result<(), Box<dyn Error>> {
    let low = low_level::Reaper::with_all_functions_loaded(context.function_provider);
    let medium = medium_level::Reaper::new(low);
    Reaper::with_custom_medium(medium)
        .setup();
    // TODO
    Ok(())
}
```

    
## Develop

### Build

- `bindgen` should be executed on Linux (including Windows WSL)