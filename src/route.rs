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

//!Mapping rendered files to relative URLs.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub struct RoutePrefix(Cow<'static, str>);

/// Explains how to map the relative file system path to a relative URL.
pub enum Route {
    /// Keep file system path and url identical
    Id,
    /// Disregard file system path and always use this constant value.
    Const(String),
    /// Replace extension in file system path with this value.
    SetExtension(&'static str),
    Custom(Box<dyn Fn(&Path) -> PathBuf>),
}
