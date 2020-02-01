//! A library for build scripts to run npm scripts
//!
//! This library is intended to be used as a `build-dependencies` entry in
//! `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! npm-rs = {git = "https://github.com/peacememories/npm-rs"}
//! ```
//!
//! The purpose of this crate is to allow easy execution of npm build scripts.
//! Configuration is available through the [`Build`] struct.
//!
//!
//! # Examples
//!
//! Use the [`Build`] struct to run `npm run build` in the target directory.
//!
//! ```no_run
//! // build.rs
//!
//! use npm_rs::Build;
//! use std::env;
//! use std::path::PathBuf;
//!
//! fn main() {
//!     Build::new()
//!         .project_directory(env::var("CARGO_MANIFEST_DIR").unwrap())
//!         .target_directory(PathBuf::from(env::var("OUT_DIR").unwrap()).join("npm_dir"))
//!         .copy_all()
//!         .run_script("build");
//! }
//! ```
//!
//! [`Build`]: struct.Build.html

use fs_extra::{copy_items, dir::CopyOptions, remove_items};
use std::env;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

#[derive(PartialEq)]
enum CopyItems {
    Nothing,
    All,
    Some(Vec<PathBuf>),
}

enum NodeEnv {
    Production,
    Development,
    Custom(String),
}

impl NodeEnv {
    fn to_env_var(&self) -> &str {
        match self {
            Self::Production => "production",
            Self::Development => "development",
            Self::Custom(str) => str.as_ref(),
        }
    }
}

/// A builder for an npm runner configuration
///
/// A `Build` is the main type of the `npm-rs` crate and is used to control all
/// configuration options.
pub struct Build {
    project_directory: PathBuf,
    copy: CopyItems,
    target_directory: PathBuf,
    installed: bool,
    node_env: NodeEnv,
}

impl Default for Build {
    fn default() -> Self {
        Self::new()
    }
}

fn is_release() -> bool {
    !cfg!(debug_assertions)
}

fn node_env() -> NodeEnv {
    match env::var("NODE_ENV") {
        Ok(str) if str == "production" => NodeEnv::Production,
        Ok(str) if str == "development" => NodeEnv::Development,
        Ok(str) => NodeEnv::Custom(str),
        Err(_) => {
            if is_release() {
                NodeEnv::Production
            } else {
                NodeEnv::Development
            }
        }
    }
}

fn get_folder_contents(dir: &PathBuf) -> Vec<PathBuf> {
    dir.read_dir()
        .unwrap()
        .map(Result::unwrap)
        .filter_map(|read_dir| {
            let name = read_dir.file_name();
            if name != "node_modules" {
                Some(PathBuf::from(name))
            } else {
                None
            }
        })
        .collect()
}

fn copy_to_target(config: &CopyItems, from: &PathBuf, to: &PathBuf) {
    let item_list = match config {
        CopyItems::Nothing => panic!("Target directory selected but no items to copy there"),
        CopyItems::All => get_folder_contents(from),
        CopyItems::Some(items) => items.clone(),
    };
    for item in &item_list {
        if item.is_absolute() {
            panic!("Items to be copied cannot be absolute paths");
        }
    }
    remove_items(&item_list.iter().map(|p| to.join(p)).collect()).unwrap();
    copy_items(
        &item_list.iter().map(|p| from.join(p)).collect(),
        to,
        &CopyOptions::new(),
    )
    .unwrap();
}

impl Build {
    /// Construct a new instance of a blank set of configuration.
    ///
    /// This builder is finished with one or multiple calls to
    /// the [`run_script`] function.
    ///
    /// [`run_script`]: struct.Build.html#method.run_script
    pub fn new() -> Self {
        Build {
            project_directory: "".into(),
            copy: CopyItems::Nothing,
            target_directory: "".into(),
            installed: false,
            node_env: node_env(),
        }
    }

    /// Set the `NODE_ENV` environment variable. If this function is not
    /// called the `NODE_ENV` defaults to
    /// * `production` if building with `--release`
    /// * `development` otherwise
    pub fn node_env(&mut self, value: &str) -> &mut Self {
        self.node_env = match value {
            "production" => NodeEnv::Production,
            "development" => NodeEnv::Development,
            custom => NodeEnv::Custom(custom.into()),
        };
        self
    }

    /// Set the target directory. Defaults to the current directory, which
    /// for a build script is the directory the `build.rs` resides in.
    /// This is the directory the npm project gets copied to.
    ///
    /// If you set this to something other than the [`project_directory`],
    /// you must call either [`copy_items`] or [`copy_all`] to tell the
    /// `Build` how to move the project files.
    ///
    /// [`project_directory`]: struct.Build.html#method.project_directory
    /// [`copy_items`]: struct.Build.html#method.copy_items
    /// [`copy_all`]: struct.Build.html#method.copy_all
    pub fn target_directory<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.target_directory = directory.as_ref().into();
        self.installed = false;
        self
    }

    /// Sets the project directory, which is where the source of the npm
    /// project is located. Defaults to the current directory, which for
    /// a build script is the directory the `build.rs` resides in.
    pub fn project_directory<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.project_directory = directory.as_ref().into();
        self.installed = false;
        self
    }

    /// Tells the `Build` to copy the selected items from [`project_directory`]
    /// to [`target_directory`]
    ///
    /// Has no effect if [`project_directory`] and [`target_directory`] are
    /// the same.
    ///
    /// [`target_directory`]: struct.Build.html#method.target_directory
    /// [`project_directory`]: struct.Build.html#method.project_directory
    pub fn copy_items<L: IntoIterator<Item = P>, P: AsRef<Path>>(&mut self, items: L) -> &mut Self {
        self.copy = CopyItems::Some(items.into_iter().map(|p| p.as_ref().into()).collect());
        self
    }

    /// Tells the `Build` to copy all files from [`project_directory`]
    /// to [`target_directory`]
    ///
    /// This does not copy the `node_modules` directory.
    ///
    /// Has no effect if [`project_directory`] and [`target_directory`] are
    /// the same.
    ///
    /// [`target_directory`]: struct.Build.html#method.target_directory
    /// [`project_directory`]: struct.Build.html#method.project_directory
    pub fn copy_all(&mut self) -> &mut Self {
        self.copy = CopyItems::All;
        self
    }

    /// Run an npm script with the given `script_name`.
    ///
    /// Before running the script this function copies files from
    /// [`project_directory`] to [`target_directory`] if necessary and
    /// installs node packages.
    ///
    /// It uses `npm ci` if building with `--release`.
    ///
    /// # Panics
    ///
    /// * Panics if [`target_directory`] is different from [`project_directory`]
    /// but neither [`copy_items`] nor [`copy_all`] was called.
    /// * Panics if npm cannot be found on this machine.
    /// * Panics if either `npm install`/`npm ci` returns with an error.
    /// * Panics if the executed npm script returns with an error.
    ///
    /// [`target_directory`]: struct.Build.html#method.target_directory
    /// [`project_directory`]: struct.Build.html#method.project_directory
    pub fn run_script(&mut self, script_name: &str) -> &mut Self {
        let npm = which("npm").expect("Could not find npm installation");

        if !self.installed {
            create_dir_all(&self.target_directory).expect("Could not create target directory");
            if self.project_directory != self.target_directory {
                copy_to_target(&self.copy, &self.project_directory, &self.target_directory);
            }

            let cmd = if is_release() { "ci" } else { "install" };

            let npm_status = Command::new(&npm)
                .env("NODE_ENV", self.node_env.to_env_var())
                .arg(cmd)
                .current_dir(&self.target_directory)
                .status()
                .expect("Could not run npm install/ci");
            if !npm_status.success() {
                panic!("Npm install/ci failed with a non 0 exit code");
            }
        }

        let npm_status = Command::new(&npm)
            .env("NODE_ENV", self.node_env.to_env_var())
            .args(&["run", script_name])
            .current_dir(&self.target_directory)
            .status()
            .expect("Could not start npm");

        if npm_status.success() {
            self
        } else {
            panic!("Npm finished with a non 0 exit code");
        }
    }
}
