# libssg [![License]][gpl3]&nbsp;[![No Maintenance Intended]][no-maintenance]

[gpl3]: https://github.com/epilys/libssg/blob/master/LICENSE.md
[License]: https://img.shields.io/github/license/epilys/libssg?color=white
[No Maintenance Intended]: https://img.shields.io/badge/No%20Maintenance%20Intended-%F0%9F%97%99-red
[no-maintenance]: https://unmaintained.tech/

> static site generation library

Build your own executable static generator that includes your building logic instead of using configuration files and command line arguments. Inspired by [Hakyll](https://jaspervdj.be/hakyll/).

- You will need to have `pandoc` installed to use Markdown.
- Uses the [handlebars template engine](https://docs.rs/handlebars/3.0.1/handlebars/index.html)

```rust
use libssg::*;
/*
 * $ tree
 * .
 * ├── Cargo.toml etc
 * ├── src
 * │   └── main.rs
 * ├── css
 * │   └── *.css
 * ├── images
 * │   └── *.png
 * ├── index.md
 * ├── posts
 * │   └── *.md
 * ├── _site
 * │   ├── css
 * │   │   └── *.css
 * │   ├── images
 * │   │   └── *.png
 * │   ├── index.html
 * │   ├── posts
 * │   │   └── *.html
 * │   └── rss.xml
 * └── templates
 *     ├── default.hbs
 *     └── post.hbs
*/


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::new()?;
    state
        .then(match_pattern(
            "^posts/*",
            Route::SetExtension("html"),
               Renderer::Pipeline(vec![
                   Renderer::LoadAndApplyTemplate("templates/post.hbs"),
                   Renderer::LoadAndApplyTemplate("templates/default.hbs"),
               ]),
            compiler_seq(
                pandoc(),
                Box::new(|state, path| {
                    let path = path
                        .strip_prefix(&state.output_dir().parent().unwrap())
                        .unwrap_or(&path)
                        .to_path_buf();
                    if state.verbosity() > 3 {
                        println!("adding {} to RSS snapshot", path.display());
                    }
                    let uuid = uuid_from_path(&path);
                    state.add_to_snapshot("main-rss-feed".into(), uuid);
                    Ok(Default::default())
                }),
            ),
        ))
        .then(match_pattern(
            "index.md",
            Route::SetExtension("html"),
            Renderer::LoadAndApplyTemplate("templates/default.hbs"),
            pandoc(),
        ))
        .then(copy("^images/*", Route::Id))
        .then(copy("^css/*", Route::Id))
        .then(build_rss_feed(
            "rss.xml".into(),
            rss_feed(
                "main-rss-feed".into(),
                RssItem {
                    title: "example page".into(),
                    description: "example using libssg".into(),
                    link: "http://example.local".into(),
                    last_build_date: String::new(),
                    pub_date: "Thu, 01 Jan 1970 00:00:00 +0000".to_string(),
                    ttl: 1800,
                },
            ),
        ))
        .finish()?;
    Ok(())
}
```

`cargo run` and the output is saved at `./_site/`.

Set `$FORCE`, `$VERBOSITY` (`0..5`) to change behaviour.
