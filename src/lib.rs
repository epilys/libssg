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

#![deny(
    //missing_docs,
    rustdoc::broken_intra_doc_links,
    /* groups */
    clippy::correctness,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    clippy::cargo,
    clippy::nursery,
    /* restriction */
    clippy::dbg_macro,
    clippy::rc_buffer,
    clippy::as_underscore,
    clippy::assertions_on_result_states,
    /* pedantic */
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::ptr_as_ptr,
    clippy::bool_to_int_with_if,
    clippy::borrow_as_ptr,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::cast_lossless,
    clippy::cast_ptr_alignment,
    clippy::naive_bytecount
)]
#![allow(clippy::multiple_crate_versions, clippy::missing_const_for_fn)]

//! ## How to use
//! `libssg` is meant to be used as a tool for a custom site generator binary.
//! Common tasks in static site generation are provided as tools for you to
//! combine them as you see fit in your own site.
//!
//! ### Your binary's structure
//! In the main function of your binary, you will create a [`State`](State), add
//! a bunch of [`Rule`s](Rule) to be performed sequentially and then call
//! [`State::finish`](State::finish) to perform the necessary rendering. Files
//! that didn't change *should* be cached instead of being regenerated. By
//! executing the binary, the generated site should be up to date with the
//! source content.
//!
//! An example binary and project structure:
//!
//!```no_run
//! use libssg::*;
//! /*
//!  * $ tree
//!  * .
//!  * ├── Cargo.toml etc
//!  * ├── src
//!  * │   └── main.rs
//!  * ├── css
//!  * │   └── *.css
//!  * ├── images
//!  * │   └── *.png
//!  * ├── index.md
//!  * ├── posts
//!  * │   └── *.md
//!  * ├── _site
//!  * │   ├── css
//!  * │   │   └── *.css
//!  * │   ├── images
//!  * │   │   └── *.png
//!  * │   ├── index.html
//!  * │   ├── posts
//!  * │   │   └── *.html
//!  * └── templates
//!  *     ├── default.hbs
//!  *     └── post.hbs
//!  */
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut state = State::new()?;
//!     state
//!         .then(match_pattern(
//!             "^posts/*",
//!             Route::SetExtension("html"),
//!             Renderer::Pipeline(vec![
//!                 Renderer::LoadAndApplyTemplate("templates/post.hbs"),
//!                 Renderer::LoadAndApplyTemplate("templates/default.hbs"),
//!             ]),
//!             pandoc(),
//!         ))
//!         .then(match_pattern(
//!             "index.md",
//!             Route::SetExtension("html"),
//!             Renderer::LoadAndApplyTemplate("templates/default.hbs"),
//!             pandoc(),
//!         ))
//!         .then(copy("^images/*", Route::Id))
//!         .then(copy("^css/*", Route::Id))
//!         .finish()?;
//!     Ok(())
//! }
//! ```
//!
//!`cargo run` and the output is saved at `./_site/`.
//!
//! ## Runtime configuration
//! `libssg` uses some environment variables for configuration but you can also
//! customise this in your binary. By default the following variables are read:
//! - `FORCE` if set forces rendering of all resources even if they are cached.
//! - `VERBOSITY` gets values from `0` up to `5` to change output verbosity.
//!
//!
//! ## Snapshots
//! Rendered content can be saved in named snapshots. This allows you reusing
//! rendered content in later steps, for example generating an RSS feed with
//! generated post content.
use std::{
    borrow::Cow,
    env, fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};

pub use chrono;
use indexmap::IndexMap;
pub use minijinja;
pub use serde_json::{self, Map, Value};
pub use uuid::Uuid;

pub mod route;
pub use route::*;

pub mod match_patterns;
pub use match_patterns::*;

pub mod rules;
pub use rules::*;

pub mod compilers;
pub use compilers::*;

pub mod renderers;
pub use renderers::*;

pub mod error;
pub use error::*;

pub mod filters;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

/// The state of site render.
#[derive(Debug)]
pub struct State {
    snapshots: IndexMap<String, Vec<Uuid>>,
    artifacts: IndexMap<Uuid, BuildArtifact>,
    build_actions: IndexMap<PathBuf, BuildAction>,
    templates: minijinja::Environment<'static>,
    templates_dir: PathBuf,
    output_dir: PathBuf,
    output_dirname: String,
    current_dir: PathBuf,

    err: Option<Error>,
    force_generate: bool,
    verbosity: u8,
    url_root: PathBuf,
}

impl State {
    /// Create new state.
    pub fn new(working_dir: Option<&Path>) -> Result<Self> {
        let working_dir = working_dir
            .map(Path::to_path_buf)
            .map(Ok)
            .unwrap_or_else(std::env::current_dir)?;
        std::env::set_current_dir(&working_dir).with_context(|| {
            format!(
                "Could not set current working dir to {}",
                working_dir.display()
            )
        })?;
        let templates_dir = PathBuf::from("./templates").canonicalize()?;
        let mut templates = minijinja::Environment::new();
        templates.add_filter("sort_by_key", filters::sort_by_key);
        templates.set_source(minijinja::Source::from_path("./templates"));

        let output_dirname = env::var("OUTPUT_DIR")
            .ok()
            .unwrap_or_else(|| "./_site/".into());
        match fs::create_dir(Path::new(&output_dirname)) {
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            err => err?,
        }
        let output_dir = PathBuf::from(&output_dirname).canonicalize()?;
        let current_dir = env::current_dir()?;
        Ok(Self {
            templates,
            output_dir,
            output_dirname,
            current_dir,
            templates_dir,
            artifacts: Default::default(),
            build_actions: Default::default(),

            err: None,
            snapshots: Default::default(),
            force_generate: env::var("FORCE").is_ok(),
            verbosity: env::var("VERBOSITY")
                .ok()
                .as_ref()
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(1),
            url_root: PathBuf::new(),
        })
    }

    pub fn url_root(mut self, url_root: Cow<'static, str>) -> Self {
        self.url_root = PathBuf::from(url_root.as_ref());
        self
    }

    /// Sets `force_generate` option.
    pub fn set_force_generate(&mut self, force_generate: bool) -> &mut Self {
        self.force_generate = force_generate;
        self
    }

    /// Sets `verbosity` option.
    pub fn set_verbosity(&mut self, verbosity: u8) -> &mut Self {
        self.verbosity = verbosity;
        self
    }

    /// Returns `verbosity` option.
    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    /// Returns current state of build artifacts.
    pub fn artifacts(&self) -> &IndexMap<Uuid, BuildArtifact> {
        &self.artifacts
    }

    /// Returns current state of snapshots.
    pub fn snapshots(&self) -> &IndexMap<String, Vec<Uuid>> {
        &self.snapshots
    }

    /// Initialize a snapshot.
    pub fn add_snapshot(&mut self, key: &str) {
        self.snapshots.entry(key.to_string()).or_default();
    }

    /// Adds an artifact to a snapshot.
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
        self.output_dir.push(dest);
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

    /// Adds a build action of copying a resource to a destination, unchanged.
    pub fn copy_page(&mut self, resource: PathBuf, dest: PathBuf) -> Uuid {
        let uuid = uuid_from_path(&resource);
        let modified_date: Option<chrono::DateTime<chrono::Utc>> = fs::metadata(&resource)
            .ok()
            .and_then(|mdata| mdata.modified().ok())
            .map(|st| st.into());
        let updated_date: chrono::DateTime<chrono::Utc> = {
            let output = Command::new("git")
                .args(["log", "-1", "--date=iso-strict", "--format=\"%ad\"", "--"])
                .arg(&resource)
                .output()
                .with_context(|| format!("Could not execute git log for file {resource:?}"))
                .unwrap();
            let s = String::from_utf8_lossy(&output.stdout);
            chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(s.trim().trim_matches('"'))
                .with_context(|| format!("Could not parse git date {}", s.trim().trim_matches('"')))
                .unwrap()
                .into()
        };
        if self.check_mtime(&dest, &resource) {
            if self.verbosity > 0 {
                println!(
                    "Will copy {} to {}/{}",
                    resource.display(),
                    self.output_dirname,
                    dest.display()
                );
            }
            self.build_actions.insert(
                dest.clone(),
                BuildAction {
                    src: uuid,
                    to: Renderer::None,
                },
            );
            self.artifacts.insert(
                uuid,
                BuildArtifact {
                    uuid,
                    path: dest.clone(),
                    resource,
                    metadata: Default::default(),
                    contents: String::new(),
                    modified_date,
                    updated_date,
                },
            );
        } else {
            self.artifacts.insert(
                uuid,
                BuildArtifact {
                    uuid,
                    path: dest.clone(),
                    resource: dest,
                    metadata: Default::default(),
                    contents: String::new(),
                    modified_date,
                    updated_date,
                },
            );
        }
        uuid
    }

    /// Adds a build action with a custom
    /// [`Compiler`](crate::compilers::Compiler).
    pub fn add_page(
        &mut self,
        dest: PathBuf,
        resource: PathBuf,
        compiler: &Compiler,
        renderer: Renderer,
    ) -> Result<Uuid> {
        let resource = resource
            .strip_prefix(self.output_dir().parent().unwrap())
            .unwrap_or(&resource)
            .to_path_buf();
        let uuid = uuid_from_path(&resource);
        let metadata = compiler(self, &resource)?;
        let modified_date: Option<chrono::DateTime<chrono::Utc>> = fs::metadata(&resource)
            .and_then(|mdata| mdata.modified())
            .map(chrono::DateTime::from)
            .ok();
        //git log -1 --date=iso-strict --format="%ad" --
        let updated_date: chrono::DateTime<chrono::Utc> = {
            let output = Command::new("git")
                .args(["log", "-1", "--date=iso-strict", "--format=\"%ad\"", "--"])
                .arg(&resource)
                .output()
                .with_context(|| format!("Could not execute git log for file {resource:?}"))?;
            let s = String::from_utf8_lossy(&output.stdout);
            chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(s.trim().trim_matches('"'))
                .with_context(|| {
                    format!("Could not parse git date {}", s.trim().trim_matches('"'))
                })?
                .into()
        };
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
                println!();
            }
            self.artifacts.insert(
                uuid,
                BuildArtifact {
                    uuid,
                    path: dest.clone(),
                    resource,
                    metadata,
                    contents: String::new(),
                    modified_date,
                    updated_date,
                },
            );
            self.build_actions.insert(
                dest.clone(),
                BuildAction {
                    src: uuid,
                    to: renderer,
                },
            );
        } else {
            if self.verbosity > 0 {
                println!("Using cached {}/{}", self.output_dirname, dest.display());
            }
            self.artifacts.insert(
                uuid,
                BuildArtifact {
                    uuid,
                    path: dest.clone(),
                    resource,
                    metadata,
                    contents: String::new(),
                    modified_date,
                    updated_date,
                },
            );
        }
        Ok(uuid)
    }

    /// Add a new [`Rule`](Rule).
    pub fn then(&mut self, rule: Rule) -> &mut Self {
        if self.err.is_none() {
            if let Err(err) = rule(self) {
                self.err = Some(err);
            }
        }
        self
    }

    /// Render a context with a specific template and return it.
    pub fn templates_render(
        &self,
        template_path: &'static str,
        context: &Map<String, Value>,
    ) -> Result<String> {
        self.templates
            .get_template(template_path)?
            .render(&minijinja::value::Value::from_serializable(&context))
            .map_err(|err| {
                format!(
                    "Encountered error when trying to render with template `{}`: {}",
                    template_path, err
                )
                .into()
            })
    }

    /// Perform all build actions.
    pub fn finish(&mut self) -> Result<()> {
        if let Some(err) = self.err.take() {
            Err(err)?;
        }

        if self.build_actions.is_empty() {
            println!(
                r#"Nothing to be generated. This might happen if:
- You haven't added any rules.
- You either haven't made any changes to your source files or they weren't detected (might be a bug). Rerun with $FORCE environmental variable set to ignore mtimes and force generation. Set $VERBOSITY to greater than 1 to get more messages."#
            );
            return Ok(());
        }
        self.artifacts
            .sort_by(|_ak, av, _bk, bv| av.updated_date.cmp(&bv.updated_date));
        self.build_actions.sort_by(|_ak, av, _bk, bv| {
            self.artifacts[&av.src]
                .updated_date
                .cmp(&self.artifacts[&bv.src].updated_date)
        });
        let fs_depth = self.output_dir.components().count();
        if self.verbosity > 0 {
            println!("Output directory is {}", self.output_dir.display());
        }
        let actions = self.build_actions.drain(..).collect::<Vec<(_, _)>>();
        for (mut path, action) in actions {
            let artifact = &self.artifacts[&action.src];
            let mut metadata = artifact.metadata.clone();
            metadata.insert(
                "ROOT_PREFIX".to_string(),
                serde_json::json! { self.url_root.display().to_string() },
            );
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
                    println!();
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

    /// Return `output_dir`.
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Return `current_dir`.
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }
}

/// An artifact generated during the build process.
pub struct BuildArtifact {
    pub uuid: Uuid,
    pub path: PathBuf,
    pub resource: PathBuf,
    pub metadata: Map<String, Value>,
    pub contents: String,
    pub modified_date: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_date: chrono::DateTime<chrono::Utc>,
}

impl std::fmt::Debug for BuildArtifact {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct(stringify!(BuildArtifact))
            .field("uuid", &self.uuid)
            .field("resource", &self.resource.display())
            .field("metadata", &self.metadata)
            .field("modified_date", &self.modified_date)
            .field("updated_date", &self.updated_date)
            .field("contents", &format!("{:.15}..", self.contents))
            .finish()
    }
}

/// Build actions to be performed in the finish stage.
#[derive(Debug)]
pub struct BuildAction {
    pub src: Uuid,
    pub to: Renderer,
}

/// Create an [`Uuid`](uuid::Uuid) from a [Path] using
/// [`Uuid::NAMESPACE_OID`](uuid::Uuid::NAMESPACE_OID).
pub fn uuid_from_path(path: &Path) -> Uuid {
    Uuid::new_v3(&Uuid::NAMESPACE_OID, path.as_os_str().as_bytes())
}
