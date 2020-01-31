use npm_rs::Build;

fn main() {
    Build::new()
        .project_directory("examples/node-project")
        .target_directory("examples/node-project")
        .run_script("build");
}
