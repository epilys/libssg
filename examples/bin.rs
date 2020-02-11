/*
 * libssg
 *
 * Copyright 2020 Manos Pitsidianakis
 *
 * This file is part of libssg.
 *
 * libssg is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * libssg is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with libssg. If not, see <http://www.gnu.org/licenses/>.
 */

use libssg;

/* This expects the following directory tree:
 *  ├── bin.rs
 *  ├── css
 *  │   └── main.css
 *  ├── index.md
 *  ├── posts
 *  │   └── *.md
 *  └── templates
 *      ├── default.html
 *      └── index.html
 *
 *
 *  Run executable in this directory.
 *  Customize behaviour via environmental variables, eg:
 *
 *   FORCE= VERBOSITY=3 cargo run --example bin
 */

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = libssg::State::new()?;
    state
        .then(libssg::match_pattern(
            "^posts/*",
            libssg::Route::SetExtension("html"),
            libssg::Renderer::LoadAndApplyTemplate("templates/default.html"),
            libssg::compiler_seq(
                libssg::pandoc(),
                Box::new(|state, path| {
                    let path = path
                        .strip_prefix(&state.output_dir().parent().unwrap())
                        .unwrap_or(&path)
                        .to_path_buf();
                    if state.verbosity() > 3 {
                        println!("adding {} to RSS snapshot", path.display());
                    }
                    let uuid = libssg::uuid_from_path(&path);
                    state.add_to_snapshot("main-rss-feed".into(), uuid);
                    Ok(Default::default())
                }),
            ),
        ))
        .then(libssg::match_pattern(
            "^index.md",
            libssg::Route::SetExtension("html"),
            libssg::Renderer::Pipeline(vec![
                libssg::Renderer::LoadAndApplyTemplate("templates/index.html"),
                libssg::Renderer::LoadAndApplyTemplate("templates/default.html"),
            ]),
            libssg::pandoc(),
        ))
        .then(libssg::copy("^css/*", libssg::Route::Id))
        .then(libssg::build_rss_feed(
            "rss.xml".into(),
            libssg::rss_feed(
                "main-rss-feed".into(),
                libssg::RssItem {
                    title: "example page".into(),
                    description: "example using libssg".into(),
                    link: "http://localhost".into(),
                    last_build_date: String::new(),
                    pub_date: "Thu, 01 Jan 1970 00:00:00 +0000".to_string(),
                    ttl: 1800,
                },
            ),
        ))
        .finish()?;
    Ok(())
}
