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

use handlebars;
use regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use handlebars::Handlebars;

mod route;
pub use route::*;

mod match_patterns;
pub use match_patterns::*;

mod rules;
pub use rules::*;

mod helpers;
pub use helpers::*;

mod compilers;
pub use compilers::*;

mod renderers;
pub use renderers::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

#[derive(Debug)]
enum PageGeneration {
    Create(String),
    Copy(PathBuf),
}

#[derive(Debug)]
pub struct State {
    pages: HashMap<PathBuf, PageGeneration>,
    templates: Handlebars<'static>,
    output_dir: PathBuf,
    force_generate: bool,
    verbosity: u8,
}

impl State {
    pub fn new() -> Self {
        let mut templates = Handlebars::new();
        templates
            .register_templates_directory("", "./templates")
            .expect("Could not find templates/ dir");
        templates.register_helper("include", Box::new(include_helper));
        match fs::create_dir(&Path::new("./_site/")) {
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            err => err.unwrap(),
        }
        let output_dir = PathBuf::from("./_site/").canonicalize().unwrap();
        State {
            pages: Default::default(),
            templates,
            output_dir,
            force_generate: env::var("FORCE").is_ok(),
            verbosity: env::var("VERBOSITY")
                .ok()
                .as_ref()
                .and_then(|v| u8::from_str_radix(v, 10).ok())
                .unwrap_or(1),
        }
    }

    pub fn set_force_generate(&mut self, force_generate: bool) -> &mut Self {
        self.force_generate = force_generate;
        self
    }

    pub fn set_verbosity(&mut self, verbosity: u8) -> &mut Self {
        self.verbosity = verbosity;
        self
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    /// Check if `dest`'s mtime is older than `resource`'s.
    pub fn check_mtime(&mut self, dest: &Path, resource: &Path) -> bool {
        let resource = env::current_dir().unwrap().join(resource);
        if self.force_generate {
            return true;
        }
        let fs_depth = self.output_dir.components().count();
        self.output_dir.push(&dest);
        if self.verbosity > 1 {
            print!(
                "Checking resource {} against destination path {}... ",
                resource.display(),
                self.output_dir.display()
            );
        }
        let mut ret = true;
        if self.output_dir.exists() {
            if let Ok(out_mtime) = fs::metadata(&self.output_dir).and_then(|mdata| mdata.modified())
            {
                if let Ok(src_mtime) = fs::metadata(&resource).and_then(|mdata| mdata.modified()) {
                    if src_mtime <= out_mtime {
                        ret = false;
                    }
                }
            }
        }
        /* Cleanup */
        for _ in fs_depth..self.output_dir.components().count() {
            self.output_dir.pop();
        }
        if self.verbosity > 1 {
            println!("returning {}", ret);
        }
        ret
    }

    pub fn copy_page(&mut self, resource: PathBuf, dest: PathBuf) {
        if self.check_mtime(&dest, &resource) {
            if self.verbosity > 0 {
                println!(
                    "Will copy {} to _site/{}",
                    resource.display(),
                    dest.display()
                );
            }
            self.pages.insert(dest, PageGeneration::Copy(resource));
        }
    }

    pub fn add_page(&mut self, path: PathBuf, contents: String) -> &mut Self {
        if self.verbosity > 0 {
            println!("Will create _site/{}", path.display());
        }
        self.pages.insert(path, PageGeneration::Create(contents));
        self
    }

    pub fn then(&mut self, rule: Rule) -> &mut Self {
        rule(self);
        self
    }

    pub fn templates_render(&self, template_path: &'static str, context: &Value) -> String {
        let template = Path::new(template_path).strip_prefix("templates/").unwrap();
        self.templates
            .render(&template.display().to_string(), context)
            .unwrap()
    }

    pub fn finish(&mut self) {
        if self.pages.is_empty() {
            println!(r#"Nothing to be generated. This might happen if:
- You haven't added any rules.
- You either haven't made any changes to your source files or they weren't detected (might be a bug). Rerun with $FORCE environmental variable set to ignore mtimes and force generation. Set $VERBOSITY to greater than 1 to get more messages."#);
            return;
        }
        let fs_depth = self.output_dir.components().count();
        if self.verbosity > 0 {
            println!("Output directory is {}", self.output_dir.display());
        }
        for (mut path, generation) in self.pages.drain() {
            if path.is_absolute() {
                path = path
                    .strip_prefix(&self.output_dir.parent().unwrap())
                    .unwrap()
                    .to_path_buf();
            }

            self.output_dir.push(&path);
            match fs::create_dir_all(self.output_dir.parent().unwrap()) {
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                err => err.unwrap(),
            }
            match generation {
                PageGeneration::Create(contents) => {
                    use std::io::prelude::*;

                    if self.verbosity > 0 {
                        println!("{}: creating {}", path.display(), self.output_dir.display());
                    }
                    let mut file = fs::File::create(&self.output_dir).unwrap();
                    file.write_all(contents.as_bytes()).unwrap();
                }
                PageGeneration::Copy(src_path) => {
                    if self.verbosity > 0 {
                        println!(
                            "{}: copying to {}",
                            src_path.display(),
                            self.output_dir.display()
                        );
                    }
                    assert!(&src_path != &self.output_dir);

                    fs::copy(src_path, &self.output_dir).unwrap();
                }
            }
            for _ in fs_depth..self.output_dir.components().count() {
                self.output_dir.pop();
            }
        }
    }
}
