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

use super::*;
use std::env;

/// `Rule`s are generation steps, that is, separate steps in the generation process. They can
/// alter `State` however they like.
pub type Rule = Box<dyn FnOnce(&mut State) -> Result<()>>;

/// Find matches from current directory and potentially descendants for `pattern`. For each
/// match, create a route, render and compile.
pub fn match_pattern<P: Into<MatchPattern>>(
    pattern: P,
    route: Route,
    renderer: Renderer,
    compiler: Compiler,
) -> Rule {
    let patterns = pattern.into();
    Box::new(move |state: &mut State| {
        for pattern in patterns {
            for entry in pattern.list() {
                let resource = entry.path();
                let extension = if let Some(e) = resource.extension() {
                    e
                } else {
                    continue;
                };
                if extension == "markdown" || extension == "md" {
                    let mut dest_path = resource
                        .strip_prefix(env::current_dir().unwrap())?
                        .to_path_buf();
                    let dest_path = match route {
                        Route::Id => dest_path,
                        Route::Const(ref s) => PathBuf::from(s),
                        Route::SetExtension(extension) => {
                            dest_path.set_extension(extension);
                            dest_path
                        }
                        Route::Custom(ref cl) => cl(&dest_path),
                    };
                    state.add_page(
                        dest_path.clone(),
                        resource.clone(),
                        &compiler,
                        renderer.clone(),
                    )?;
                }
            }
        }
        Ok(())
    })
}

pub fn create(path: PathBuf, compiler: Compiler) -> Rule {
    Box::new(move |state: &mut State| {
        state.add_page(path.clone(), path.clone(), &compiler, Renderer::None)?;
        Ok(())
    })
}

/// Copy everything that matches to `pattern` to destinations according to `route`
pub fn copy<P: Into<MatchPattern>>(pattern: P, route: Route) -> Rule {
    let patterns = pattern.into();
    Box::new(move |state: &mut State| {
        for pattern in patterns {
            for entry in pattern.list() {
                let rel_path = entry
                    .path()
                    .strip_prefix(&state.current_dir())?
                    .to_path_buf();
                state.copy_page(
                    rel_path.clone(),
                    match route {
                        Route::Id => rel_path,
                        Route::Const(ref s) => PathBuf::from(s),
                        Route::SetExtension(extension) => {
                            let mut path = rel_path;
                            path.set_extension(extension);
                            path
                        }
                        Route::Custom(ref cl) => cl(&rel_path),
                    },
                );
            }
        }
        Ok(())
    })
}

pub fn build_rss_feed(path: PathBuf, compiler: Compiler) -> Rule {
    Box::new(move |state: &mut State| {
        state.add_page(
            path.clone(),
            path.clone(),
            &compiler,
            Renderer::Custom(Box::new(|metadata| {
                Ok(if let Value::Object(ref map) = metadata {
                    map.get("body").and_then(|b| b.as_str()).ok_or_else(|| format!("Internal error while building rss feed: metadata does not contain `body`: {:#?}", &map))?.to_string()
                } else {
                    String::new()
                })
            })),
        )?;
        Ok(())
    })
}
