/*
For 'unwrap' /sa https://doc.rust-lang.org/rust-by-example/error/option_unwrap.html
For 'iter' and 'collect' /sa  https://doc.rust-lang.org/std/path/struct.PathBuf.html#examples
For '?' /sa https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator
For HashMap /sa https://doc.rust-lang.org/std/collections/struct.HashMap.html
*/

use std::{collections::HashMap, fs, path::PathBuf};
use structopt::StructOpt;

const CMLT_FILE_NAME: &str = "CMakeLists.txt";
const CMLT: &str = include_str!("../res/CMakeLists.txt.in");

// Options
#[derive(Debug, StructOpt)]
#[structopt(name = "cpp-proj-gen", about = "C++ project generator.")]
pub struct Opt {
    // Project name
    #[structopt(short, long, help = "e.g. company name")]
    name_space: Option<String>,

    // Target name
    #[structopt(short, long, default_value = "my-target")]
    target_name: String,

    // CMake version
    #[structopt(short, long, default_value = "3.15.0")]
    cmake_version: String,

    // Output directory
    #[structopt(short, long, parse(from_os_str))]
    output_dir: Option<PathBuf>,
}

type PathBufVec = Vec<PathBuf>;
type CmakeVarsMap = HashMap<String, String>;

// CppProjGen
#[derive(Debug)]
pub struct CppProjGen {
    directories: PathBufVec,
    cmake_lists_file: PathBuf,
    cmake_vars: CmakeVarsMap,
    opt: Opt,
    out_dir: PathBuf,
}

impl CppProjGen {
    pub fn new(opt: Opt) -> Self {
        let vars: HashMap<String, String> = [
            (
                String::from("@CMAKE_MINIMUM_VERSION@"),
                String::from(&opt.cmake_version),
            ),
            (
                String::from("@CMAKE_TARGET_NAME@"),
                String::from(&opt.target_name),
            ),
            (
                String::from("@CMAKE_PROJECT_NAME@"),
                build_cmake_project_name(&opt, "-"),
            ),
            (
                String::from("@INCLUDE_DOMAIN_DIR@"),
                build_cmake_project_name(&opt, "/"),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        Self {
            directories: Vec::new(),
            cmake_lists_file: PathBuf::from(CMLT_FILE_NAME),
            cmake_vars: vars,
            out_dir: build_out_dir(&opt),
            opt: opt,
        }
    }

    pub fn add_include_dir(mut self, dir: PathBuf) -> Self {
        self.cmake_vars.insert(
            String::from("@INCLUDE_DIR@"),
            String::from(dir.to_str().unwrap()),
        );

        let local_include_dir: PathBuf = build_cmake_local_include_dir(&self.opt, dir);

        self.add_toplevel_dir(local_include_dir)
    }

    pub fn add_source_dir(mut self, dir: PathBuf) -> Self {
        self.cmake_vars.insert(
            String::from("@SOURCE_DIR@"),
            String::from(dir.to_str().unwrap()),
        );

        self.add_toplevel_dir(dir)
    }

    pub fn add_toplevel_dir(mut self, dir: PathBuf) -> Self {
        self.directories.push(dir);

        self
    }

    pub fn gen(&self, progress: Option<fn(String)>) -> std::io::Result<()> {
        let contents = replace_cmake_vars(CMLT, &self.cmake_vars);
        let paths = self.build_paths();
        create_all_paths(paths, contents, progress)?;

        Ok(())
    }

    pub fn build_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        for dir in &self.directories {
            paths.push(make_absolute_path(&self.out_dir, dir));
        }

        paths.push(make_absolute_path(&self.out_dir, &self.cmake_lists_file));

        paths
    }
}

fn build_cmake_local_include_dir(opt: &Opt, dir: PathBuf) -> PathBuf {
    let result_dir: PathBuf = if opt.name_space.is_none() {
        // e.g. include/target-name
        [dir, PathBuf::from(&opt.target_name)].iter().collect()
    } else {
        // e.g. include/name-space/target-name
        [
            dir,
            PathBuf::from(opt.name_space.as_ref().unwrap()),
            PathBuf::from(&opt.target_name),
        ]
        .iter()
        .collect()
    };

    result_dir
}

fn build_out_dir(opt: &Opt) -> PathBuf {
    let parent = match &opt.output_dir {
        Some(p) => p.clone(),
        None => PathBuf::from(std::env::current_dir().unwrap().clone()),
    };

    let out_dir: PathBuf = [parent, PathBuf::from(&opt.target_name)].iter().collect();

    out_dir
}

fn make_absolute_path(out_dir: &PathBuf, dir: &PathBuf) -> PathBuf {
    [out_dir, dir].iter().collect()
}

fn replace_cmake_vars(cmake_contents: &str, cmake_vars: &HashMap<String, String>) -> String {
    let mut result = String::from(cmake_contents);

    for (var, value) in cmake_vars {
        result = result.replace(var, value);
    }

    result
}

fn build_cmake_project_name(opt: &Opt, delimiter: &str) -> String {
    let project_name = if opt.name_space.is_none() {
        String::from(&opt.target_name)
    } else {
        let tmp = format!(
            "{}{}{}",
            opt.name_space.as_ref().unwrap(),
            delimiter,
            &opt.target_name
        );
        tmp
    };

    project_name
}

fn create_all_paths(
    paths: Vec<PathBuf>,
    contents: String,
    progress: Option<fn(String)>,
) -> std::io::Result<()> {
    for path in paths {
        if progress.is_some() {
            progress.unwrap()(path.to_str().unwrap().to_string());
        }
        // TODO: How to distinguish between file and dir?
        if path.ends_with(CMLT_FILE_NAME) {
            fs::write(path, &contents)?;
        } else {
            fs::create_dir_all(path)?;
        }
    }

    Ok(())
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_opt() -> Opt {
        let opt = Opt {
            name_space: Some(String::from("nmspc")),
            target_name: String::from("tgtnm"),
            cmake_version: String::from("1.23.4"),
            output_dir: Some(PathBuf::from("test_out_dir")),
        };

        opt
    }

    #[test]
    fn test_path_vec_len() {
        let opt = create_test_opt();

        let cpp_proj_gen = CppProjGen::new(opt)
            .add_include_dir(PathBuf::from("include"))
            .add_toplevel_dir(PathBuf::from("test"))
            .add_source_dir(PathBuf::from("source"));

        let paths = cpp_proj_gen.build_paths();
        assert_eq!(paths.len(), 4);
    }

    #[test]
    fn test_path_vec_items() {
        let opt = create_test_opt();

        let cpp_proj_gen = CppProjGen::new(opt)
            .add_include_dir(PathBuf::from("include"))
            .add_toplevel_dir(PathBuf::from("test"))
            .add_source_dir(PathBuf::from("source"));

        let paths = cpp_proj_gen.build_paths();

        assert_eq!(
            paths.contains(&PathBuf::from("test_out_dir/tgtnm/include/nmspc/tgtnm")),
            true
        );

        assert_eq!(
            paths.contains(&PathBuf::from("test_out_dir/tgtnm/test")),
            true
        );

        assert_eq!(
            paths.contains(&PathBuf::from("test_out_dir/tgtnm/source")),
            true
        );

        assert_eq!(
            paths.contains(&PathBuf::from("test_out_dir/tgtnm/CMakeLists.txt")),
            true
        );

        println!("{:#?}", paths);
    }

    #[test]
    fn test_cmake_vars() {
        let opt = create_test_opt();

        let cpp_proj_gen = CppProjGen::new(opt)
            .add_include_dir(PathBuf::from("include"))
            .add_toplevel_dir(PathBuf::from("test"))
            .add_source_dir(PathBuf::from("source"));

        println!("{:#?}", cpp_proj_gen.cmake_vars);

        let result = replace_cmake_vars(CMLT, &cpp_proj_gen.cmake_vars);
        println!("{}", result);
    }

    #[test]
    fn test_include_dir_without_namespace() {
        let opt = Opt {
            name_space: None,
            target_name: String::from("tgtnm"),
            cmake_version: String::from("1.23.4"),
            output_dir: Some(PathBuf::from("test_out_dir")),
        };

        let cpp_proj_gen = CppProjGen::new(opt)
            .add_include_dir(PathBuf::from("include"))
            .add_toplevel_dir(PathBuf::from("test"))
            .add_source_dir(PathBuf::from("source"));

        let paths = cpp_proj_gen.build_paths();
        // println!("{:#?}", paths);

        assert_eq!(
            paths.contains(&PathBuf::from("test_out_dir/tgtnm/include/tgtnm")),
            true
        );

        // let result = replace_cmake_vars(&cpp_proj_gen.cmake_vars);
        // println!("{}", result);
    }
}
