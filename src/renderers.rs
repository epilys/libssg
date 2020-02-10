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

use super::State;
use serde_json::Value;
use std::path::Path;

/// A template rending pipeline.
pub enum Renderer {
    LoadAndApplyTemplate(&'static str),
    Pipeline(Vec<Renderer>),
}

impl Renderer {
    /// check if we should overwrite `dest_path` by checking if the template's mtime is newer
    /// than the destination.
    pub fn check_mtime(&self, state: &mut State, dest_path: &Path) -> bool {
        match self {
            Renderer::LoadAndApplyTemplate(ref path) => {
                state.check_mtime(dest_path, &Path::new(path))
            }
            Renderer::Pipeline(ref list) => list
                .iter()
                .fold(false, |acc, el| acc || el.check_mtime(state, dest_path)),
        }
    }

    pub fn render(&self, state: &mut State, context: &mut Value) -> String {
        match self {
            Renderer::LoadAndApplyTemplate(ref path) => state.templates_render(path, context),
            Renderer::Pipeline(ref list) => {
                let mut iter = list.iter().peekable();
                while let Some(stage) = iter.next() {
                    let new_body = stage.render(state, context);
                    if iter.peek().is_none() {
                        return new_body;
                    } else if let Value::Object(ref mut map) = context {
                        map.insert("body".to_string(), Value::String(new_body));
                    }
                }
                String::new()
            }
        }
    }
}
