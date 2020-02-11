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
use handlebars::Handlebars;
use regex;
pub use serde_json::Value;
use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
pub use uuid::Uuid;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
pub struct State {
    snapshots: HashMap<String, Vec<Uuid>>,
    artifacts: HashMap<Uuid, BuildArtifact>,
    build_actions: HashMap<PathBuf, BuildAction>,
    templates: Handlebars<'static>,
    output_dir: PathBuf,
    current_dir: PathBuf,

    err: Option<Box<dyn std::error::Error>>,
    force_generate: bool,
    verbosity: u8,
}

impl State {
    pub fn new() -> Result<Self> {
        let mut templates = Handlebars::new();
        templates
            .register_templates_directory("", "./templates")
            .map_err(|_| "Could not find templates/ dir")?;
        templates.register_helper("include", Box::new(include_helper));
        match fs::create_dir(&Path::new("./_site/")) {
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            err => err?,
        }
        let output_dir = PathBuf::from("./_site/").canonicalize()?;
        let current_dir = env::current_dir()?;
        Ok(State {
            templates,
            output_dir,
            current_dir,
            artifacts: Default::default(),
            build_actions: Default::default(),

            err: None,
            snapshots: Default::default(),
            force_generate: env::var("FORCE").is_ok(),
            verbosity: env::var("VERBOSITY")
                .ok()
                .as_ref()
                .and_then(|v| u8::from_str_radix(v, 10).ok())
                .unwrap_or(1),
        })
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

    pub fn artifacts(&self) -> &HashMap<Uuid, BuildArtifact> {
        &self.artifacts
    }

    pub fn snapshots(&self) -> &HashMap<String, Vec<Uuid>> {
        &self.snapshots
    }

    pub fn add_to_snapshot(&mut self, key: String, artifact: Uuid) {
        self.snapshots.entry(key).or_default().push(artifact)
    }

    /// Check if `dest`'s mtime is older than `resource`'s.
    pub fn check_mtime(&mut self, dest: &Path, resource: &Path) -> bool {
        let resource = self.current_dir.as_path().join(resource);
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

    pub fn copy_page(&mut self, resource: PathBuf, dest: PathBuf) -> Uuid {
        let uuid = uuid_from_path(&resource);
        if self.check_mtime(&dest, &resource) {
            if self.verbosity > 0 {
                println!(
                    "Will copy {} to _site/{}",
                    resource.display(),
                    dest.display()
                );
            }
            self.build_actions.insert(
                dest.clone(),
                BuildAction {
                    src: uuid.clone(),
                    to: Renderer::None,
                },
            );
            self.artifacts.insert(
                uuid.clone(),
                BuildArtifact {
                    uuid: uuid.clone(),
                    path: dest.clone(),
                    resource,
                    metadata: Default::default(),
                    contents: String::new(),
                },
            );
        } else {
            self.artifacts.insert(
                uuid.clone(),
                BuildArtifact {
                    uuid: uuid.clone(),
                    path: dest.clone(),
                    resource: dest,
                    metadata: Default::default(),
                    contents: String::new(),
                },
            );
        }
        uuid
    }

    pub fn add_page(
        &mut self,
        dest: PathBuf,
        resource: PathBuf,
        compiler: &Compiler,
        renderer: Renderer,
    ) -> Result<Uuid> {
        let resource = resource
            .strip_prefix(&self.output_dir().parent().unwrap())
            .unwrap_or(&resource)
            .to_path_buf();
        let uuid = uuid_from_path(&resource);
        let metadata = compiler(self, &resource)?;
        if self.check_mtime(&dest, &resource) || renderer.check_mtime(self, &dest) {
            if self.verbosity > 0 {
                print!(
                    "Will create {} from resource {} with artifact uuid {}",
                    dest.display(),
                    resource.display(),
                    uuid,
                );
                if self.verbosity > 3 {
                    print!(" and metadata {:#?}", &metadata,);
                }
                println!("");
            }
            self.artifacts.insert(
                uuid.clone(),
                BuildArtifact {
                    uuid: uuid.clone(),
                    path: dest.clone(),
                    resource,
                    metadata,
                    contents: String::new(),
                },
            );
            self.build_actions.insert(
                dest.clone(),
                BuildAction {
                    src: uuid.clone(),
                    to: renderer,
                },
            );
        } else {
            if self.verbosity > 0 {
                println!("Using cached _site/{}", dest.display());
            }
            self.artifacts.insert(
                uuid.clone(),
                BuildArtifact {
                    uuid: uuid.clone(),
                    path: dest.clone(),
                    resource,
                    metadata,
                    contents: String::new(),
                },
            );
        }
        Ok(uuid)
    }

    pub fn then(&mut self, rule: Rule) -> &mut Self {
        if self.err.is_none() {
            if let Err(err) = rule(self) {
                self.err = Some(err);
            }
        }
        self
    }

    pub fn templates_render(&self, template_path: &'static str, context: &Value) -> Result<String> {
        let template = Path::new(template_path).strip_prefix("templates/").unwrap();
        self.templates
            .render(&template.display().to_string(), context)
            .map_err(|err| {
                format!(
                    "Encountered error when trying to render with template `{}`: {}",
                    err, template_path
                )
                .into()
            })
    }

    pub fn finish(&mut self) -> Result<()> {
        if let Some(err) = self.err.take() {
            Err(err)?;
        }

        if self.build_actions.is_empty() {
            println!(r#"Nothing to be generated. This might happen if:
- You haven't added any rules.
- You either haven't made any changes to your source files or they weren't detected (might be a bug). Rerun with $FORCE environmental variable set to ignore mtimes and force generation. Set $VERBOSITY to greater than 1 to get more messages."#);
            return Ok(());
        }
        let fs_depth = self.output_dir.components().count();
        if self.verbosity > 0 {
            println!("Output directory is {}", self.output_dir.display());
        }
        let actions = self.build_actions.drain().collect::<Vec<(_, _)>>();
        for (mut path, action) in actions {
            let artifact = &self.artifacts[&action.src];
            let mut metadata = artifact.metadata.clone();
            let contents = match action.to {
                Renderer::None => None,
                renderer => Some(renderer.render(self, &mut metadata)?),
            };
            if path.is_absolute() {
                path = path.strip_prefix(&self.current_dir)?.to_path_buf();
            }

            self.output_dir.push(&path);
            match fs::create_dir_all(self.output_dir.parent().unwrap()) {
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                err => err?,
            }
            if let Some(contents) = contents {
                use std::io::prelude::*;

                if self.verbosity > 0 {
                    print!("{}: creating {}", path.display(), self.output_dir.display());
                    if self.verbosity > 3 {
                        print!(" and metadata {:#?}", &metadata,);
                    }
                    println!("");
                }
                let mut file = fs::File::create(&self.output_dir)?;
                file.write_all(contents.as_bytes())?;
            } else {
                let src_path = &self.artifacts[&action.src].resource;
                if self.verbosity > 0 {
                    println!(
                        "{}: copying to {}",
                        src_path.display(),
                        self.output_dir.display()
                    );
                }
                assert!(src_path != &self.output_dir);

                fs::copy(src_path, &self.output_dir)?;
            }
            for _ in fs_depth..self.output_dir.components().count() {
                self.output_dir.pop();
            }
        }
        Ok(())
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }
}

pub struct BuildArtifact {
    uuid: Uuid,
    path: PathBuf,
    resource: PathBuf,
    metadata: Value,
    contents: String,
}

impl std::fmt::Debug for BuildArtifact {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            fmt,
            "BuildArtifact {{ uuid: {}, resource: {}, metadata: {:?}, contents: \"{:.15}...\" }}",
            self.uuid,
            self.resource.display(),
            &self.metadata,
            &self.contents
        )
    }
}

#[derive(Debug)]
pub struct BuildAction {
    src: Uuid,
    to: Renderer,
}

pub fn uuid_from_path(path: &Path) -> Uuid {
    Uuid::new_v3(&Uuid::NAMESPACE_OID, path.as_os_str().as_bytes())
}
