# libssg
> static site generation library

Build your own executable static generator that includes your building logic instead of using configuration files and command line arguments. Inspired by [Hakyll](https://jaspervdj.be/hakyll/)

```rust
use libssg;

fn main() {
    let mut state = libssg::State::new();
    state
        .then(libssg::r#match(
            "posts/*",
            libssg::Route::SetExtension("html"),
            Box::new(|state, body| state.templates().render("default.html", body).unwrap()),
        ))
        .then(libssg::r#match(
            "index.md",
            libssg::Route::SetExtension("html"),
            Box::new(|state, body| state.templates().render("default.html", body).unwrap()),
        ))
        .finish();
}
```

Output is saved at `./_site/`.
