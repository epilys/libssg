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

///`Compiler`s are functions or closures that transform resource files (think stylesheets, text in markdown, etc) to
/// something. A compiler that uses `pandoc` is provided, it expects a pandoc markdown file with
/// optional metadata in the preamble like so:
///
/// ```text
///  ---
/// title: example title
/// author: epilys
/// date: June 15, 2019
/// ---
///
/// Lorem ipsum.
/// ```
///
/// `Compiler`s' only obligation is transforming the contents of the given file `path` into a
/// `String`.
pub type Compiler = Box<dyn Fn(&mut State, &Path) -> Value>;

pub use pandoc::pandoc;
mod pandoc {
    use super::*;
    use serde::{self, Deserialize};
    use serde_json;
    use serde_json::{Map, Value};
    use std::collections::HashMap;
    pub fn pandoc() -> Compiler {
        Box::new(|state: &mut State, path: &Path| {
            let metadata = Command::new("pandoc")
                .args(&["-t", "json"])
                .arg(&path)
                .output()
                .expect("failed to execute pandoc");
            let pandoc_json: PandocJsonOutput =
                serde_json::from_str(&String::from_utf8_lossy(&metadata.stdout))
                    .unwrap_or_default();
            let mut metadata_map: Map<String, Value> = parse_metadata(pandoc_json);
            if state.verbosity() > 2 {
                println!(
                    "Parsed metadata for {}: {:#?}",
                    path.display(),
                    &metadata_map
                );
            }
            let output = Command::new("pandoc")
                .arg(&path)
                .output()
                .expect("failed to execute pandoc");
            metadata_map.insert(
                "body".to_string(),
                Value::String(String::from_utf8_lossy(&output.stdout).to_string()),
            );
            Value::Object(metadata_map)
        })
    }

    fn parse_metadata(output: PandocJsonOutput) -> Map<String, Value> {
        let meta = output.meta;

        meta.into_iter()
            .map(|(key, metaval)| (key, metaval.into()))
            .collect::<_>()
    }

    #[derive(Deserialize, Debug, Default)]
    struct PandocJsonOutput {
        blocks: Value,
        #[serde(rename = "pandoc-api-version")]
        pandoc_api_version: Value,
        meta: HashMap<String, PandocMetaValue>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "t", content = "c")]
    enum PandocMetaValue {
        MetaMap(HashMap<String, PandocMetaValue>),
        MetaList(Vec<PandocMetaValue>),
        MetaBool(bool),
        MetaString(String),
        MetaInlines(Vec<PandocMetaInline>),
        MetaBlocks(Value),
    }

    impl Into<Value> for PandocMetaValue {
        fn into(self) -> Value {
            use PandocMetaValue::*;
            match self {
                MetaMap(map) => Value::Object(
                    map.into_iter()
                        .map(|(key, metaval)| (key.clone(), metaval.into()))
                        .collect(),
                ),
                MetaList(list) => Value::Array(
                    list.into_iter()
                        .map(|metaval| metaval.into())
                        .collect::<Vec<Value>>(),
                ),
                MetaBool(v) => Value::Bool(v),
                MetaString(v) => Value::String(v),
                MetaInlines(inlines_list) => Value::String(inlines_list.into_iter().fold(
                    String::new(),
                    |mut acc, inline| {
                        let inline: String = inline.into();
                        acc.extend(inline.chars());
                        acc
                    },
                )),
                MetaBlocks(_) => Value::String(String::new()),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "t", content = "c")]
    enum PandocMetaInline {
        Str(String),
        Emph(Vec<PandocMetaInline>),
        Strong(Vec<PandocMetaInline>),
        Strikeout(Vec<PandocMetaInline>),
        Superscript(Vec<PandocMetaInline>),
        Subscript(Vec<PandocMetaInline>),
        SmallCaps(Vec<PandocMetaInline>),
        Quoted(Value),
        Cite(Value),
        Code(Value),
        Space,
        SoftBreak,
        LineBreak,
        Math(Value),
        RawPandocMetaInline(Value),
        Link(Value),
        Image(Value),
        Note(Value),
        Span(Value),
    }

    impl Into<String> for PandocMetaInline {
        fn into(self) -> String {
            use PandocMetaInline::*;
            match self {
                Str(inner) => inner,
                Emph(list) => list.into_iter().fold(String::new(), |mut acc, el| {
                    let el: String = el.into();
                    acc.extend(el.chars());
                    acc
                }),
                Strong(list) => list.into_iter().fold(String::new(), |mut acc, el| {
                    let el: String = el.into();
                    acc.extend(el.chars());
                    acc
                }),
                Space => String::from(" "),
                SoftBreak => String::from("\n"),
                LineBreak => String::from("\n"),
                Strikeout(list) => list.into_iter().fold(String::new(), |mut acc, el| {
                    let el: String = el.into();
                    acc.extend(el.chars());
                    acc
                }),
                Superscript(list) => list.into_iter().fold(String::new(), |mut acc, el| {
                    let el: String = el.into();
                    acc.extend(el.chars());
                    acc
                }),
                Subscript(list) => list.into_iter().fold(String::new(), |mut acc, el| {
                    let el: String = el.into();
                    acc.extend(el.chars());
                    acc
                }),
                SmallCaps(list) => list.into_iter().fold(String::new(), |mut acc, el| {
                    let el: String = el.into();
                    acc.extend(el.chars());
                    acc
                }),
                Quoted(_) => String::new(),
                Cite(_) => String::new(),
                Code(_) => String::new(),
                Math(_) => String::new(),
                RawPandocMetaInline(_) => String::new(),
                Link(_) => String::new(),
                Image(_) => String::new(),
                Note(_) => String::new(),
                Span(_) => String::new(),
            }
        }
    }
}