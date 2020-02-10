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

fn main() {
    let mut state = libssg::State::new();
    state
        .then(libssg::match_pattern(
            "posts/*",
            libssg::Route::SetExtension("html"),
            libssg::Renderer::LoadAndApplyTemplate("templates/default.html"),
            libssg::pandoc(),
        ))
        .then(libssg::match_pattern(
            "index.md",
            libssg::Route::SetExtension("html"),
            libssg::Renderer::Pipeline(vec![
                libssg::Renderer::LoadAndApplyTemplate("templates/index.html"),
                libssg::Renderer::LoadAndApplyTemplate("templates/default.html"),
            ]),
            libssg::pandoc(),
        ))
        .then(libssg::copy("css/*", libssg::Route::Id))
        .finish();
}
