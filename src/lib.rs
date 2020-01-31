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

pub struct Build {
    project_directory: PathBuf,
    copy: CopyItems,
    target_directory: PathBuf,
    installed: bool,
    node_env: NodeEnv,
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
    pub fn new() -> Self {
        let dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
        Build {
            project_directory: dir.clone(),
            copy: CopyItems::Nothing,
            target_directory: dir,
            installed: false,
            node_env: node_env(),
        }
    }

    pub fn node_env(&mut self, value: &str) -> &mut Self {
        self.node_env = match value {
            "production" => NodeEnv::Production,
            "development" => NodeEnv::Development,
            custom => NodeEnv::Custom(custom.into()),
        };
        self
    }

    pub fn target_directory<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.target_directory = directory.as_ref().into();
        self.installed = false;
        self
    }

    pub fn project_directory<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.project_directory = directory.as_ref().into();
        self.installed = false;
        self
    }

    pub fn copy_items<L: IntoIterator<Item = P>, P: AsRef<Path>>(&mut self, items: L) -> &mut Self {
        self.copy = CopyItems::Some(items.into_iter().map(|p| p.as_ref().into()).collect());
        if self.target_directory == self.project_directory {
            self.target_directory = PathBuf::from(env::var("OUT_DIR").unwrap()).join("npm");
        }
        self
    }

    pub fn copy_all(&mut self) -> &mut Self {
        self.copy = CopyItems::All;
        self
    }

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
