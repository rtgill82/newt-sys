extern crate lazy_static;
extern crate pkg_config;
extern crate regex;

use lazy_static::lazy_static;
use pkg_config::Library;
use regex::Regex;

use std::{env, fs};
use std::path::Path;
use std::process::{Command,Stdio};

const NEWT_VERSION:   &str = "0.52.25";
const POPT_VERSION:   &str = "1.19";
const SLANG_VERSION:  &str = "2.3.3";

const OLD_CFLAGS_ENV: &str = "_OLD_CFLAGS";

lazy_static! {
    static ref MAKE: &'static str = find_gnu_make();
}

struct BuildConfig<'a> {
    build_prefix: &'a str,
    archive_name: &'a str,
    src_dir: &'a str,
    install_prefix: &'a str,
    pkg_config_path: &'a str
}

fn check_make(make: &str) -> bool {
    let cmd = Command::new(make)
        .stdin(Stdio::null())
        .args(&["-f", "-", "--version"])
        .output();

    match cmd {
        Ok(output) => {
            let re = Regex::new(r"\AGNU Make").unwrap();
            let s = String::from_utf8_lossy(output.stdout.as_slice());
            re.is_match(&s)
        },
        Err(_e) => false
    }
}

fn find_gnu_make() -> &'static str {
    for make in ["make", "gmake"].iter() {
        if check_make(make) {
            return make;
        }
    }
    panic!("GNU Make is required for building this package.");
}

fn append_pkg_config_path(path: &str) {
    if let Ok(pkg_config_path) = env::var("PKG_CONFIG_PATH") {
        let new_path = format!("{}:{}", pkg_config_path, path);
        env::set_var("PKG_CONFIG_PATH", new_path);
    } else {
        env::set_var("PKG_CONFIG_PATH", path);
    }
}

fn build_newt(version: &str, cfg: &BuildConfig) -> Library {
    let archive = &format!("{}.tar.gz", cfg.archive_name);

    Command::new("tar").args(&["xzf", archive])
        .args(&["-C", cfg.build_prefix])
        .status().expect("error running tar");

    env::set_current_dir(&Path::new(cfg.src_dir))
        .expect("unable to change directory");
    Command::new("./configure")
        .args(&["--prefix", cfg.install_prefix])
        .arg("--disable-nls")
        .arg("--without-python")
        .arg("--without-tcl")
        .status().expect("error running configure");

    Command::new(make())
        .arg("install")
        .status().expect("error running make");

    append_pkg_config_path(cfg.pkg_config_path);
    pkg_config::Config::new()
        .atleast_version(version)
        .statik(true)
        .probe("libnewt")
        .expect("error running pkg-config")
}

fn build_popt(version: &str, cfg: &BuildConfig) -> Library {
    let archive = &format!("{}.tar.gz", cfg.archive_name);
    Command::new("tar").args(&["xzf", archive])
        .args(&["-C", cfg.build_prefix])
        .status().expect("error running tar");

    env::set_current_dir(&Path::new(cfg.src_dir))
        .expect("unable to change directory");
    Command::new("./configure")
        .args(&["--prefix", cfg.install_prefix])
        .arg("--disable-nls")
        .arg("--disable-rpath")
        .status().expect("error running configure");

    Command::new(make())
        .arg("install")
        .status().expect("error running make");

    append_pkg_config_path(cfg.pkg_config_path);
    pkg_config::Config::new()
        .atleast_version(version)
        .arg("--cflags")
        .statik(true)
        .probe("popt")
        .expect("error running pkg-config")
}

fn build_slang(version: &str, cfg: &BuildConfig) -> Library {
    let archive = &format!("{}.tar.bz2", cfg.archive_name);

    cflags_set_fpic();
    Command::new("tar").args(&["xjf", archive])
        .args(&["-C", cfg.build_prefix])
        .status().expect("error running tar");

    env::set_current_dir(&Path::new(cfg.src_dir))
        .expect("unable to change directory");
    Command::new("./configure")
        .args(&["--prefix", cfg.install_prefix])
        .status().expect("error running configure");

    Command::new(make())
        .arg("install-static")
        .status().expect("error running make");

    cflags_restore();
    append_pkg_config_path(cfg.pkg_config_path);
    pkg_config::Config::new()
        .atleast_version(version)
        .arg("--cflags")
        .statik(true)
        .probe("slang")
        .expect("error running pkg-config")
}

#[inline]
fn make() -> &'static str {
    &MAKE
}

fn export_env_libs(libs: &[Box<Library>]) {
    let mut include_paths = String::new();
    let mut link_paths = String::new();

    for lib in libs {
        for ipath in lib.include_paths.iter() {
            if let Some(path) = ipath.to_str() {
                include_paths.push_str(&format!("-I{} ", path));
            }
        }

        for lpath in lib.link_paths.iter() {
            if let Some(path) = lpath.to_str() {
                link_paths.push_str(&format!("-L{} ", path));
            }
        }
    }

    if include_paths.len() > 0 {
        env::set_var("CPPFLAGS", include_paths)
    }

    if link_paths.len() > 0 {
        env::set_var("LDFLAGS", link_paths)
    }
}

fn unset_env_libs() {
    env::remove_var("CPPFLAGS");
    env::remove_var("LDFLAGS");
}

fn cflags_set_fpic() {
    let mut cflags = match env::var("CFLAGS") {
        Ok(val) => val,
        Err(_)  => String::new()
    };

    if !cflags.contains("-fPIC") {
        env::set_var(OLD_CFLAGS_ENV, &cflags);
        cflags.push_str(" -fPIC");
        env::set_var("CFLAGS", &cflags);
    }
}

fn cflags_restore() {
    if let Ok(old_cflags) = env::var(OLD_CFLAGS_ENV) {
        env::set_var("CFLAGS", &old_cflags);
        env::remove_var(OLD_CFLAGS_ENV);
    }
}

fn build(package: &str, version: &str, out_dir: &str,
         libs: Option<&[Box<Library>]>) -> Library {
    let crate_path = env::var("CARGO_MANIFEST_DIR").unwrap();
    let version_name = &format!("{}-{}", package, version);
    let build_prefix = &format!("{}/build", out_dir);
    let install_prefix = &format!("{}/install/{}", out_dir, version_name);

    let build_cfg = BuildConfig {
        build_prefix: &build_prefix,
        archive_name: &format!("{}/vendor/{}", crate_path, version_name),
        src_dir: &format!("{}/{}", build_prefix, version_name),
        install_prefix: &install_prefix,
        pkg_config_path: &format!("{}/lib/pkgconfig", install_prefix)
    };

    if let Some(libs) = libs { export_env_libs(&libs) }
    let old_dir = env::current_dir()
        .expect("unable to read current directory");
    fs::create_dir_all(&Path::new(build_prefix))
        .expect("unable to create build directory");
    env::set_current_dir(&Path::new(build_prefix))
        .expect("unable to change directory");
    let library = match package {
        "newt" => build_newt(version, &build_cfg),
        "popt" => build_popt(version, &build_cfg),
        "slang" => build_slang(version, &build_cfg),
        _ => panic!("Unexpected package requested to be built: {}", package)
    };
    env::set_current_dir(&old_dir)
        .expect("unable to change directory");
    unset_env_libs();
    return library;
}

fn build_libs() -> Library {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut libraries: Vec<Box<Library>> = Vec::new();

    let library = Box::new(build("popt", POPT_VERSION, &out_dir, None));
    libraries.push(library);

    let library = Box::new(build("slang", SLANG_VERSION, &out_dir, None));
    libraries.push(library);

    build("newt", NEWT_VERSION, &out_dir, Some(&libraries))
}

fn build_c(lib: &Library) {
    let mut build = cc::Build::new();
    build.file("src/colorset_custom.c");
    for path in lib.include_paths.iter() {
        build.include(path);
    }
    build.compile("libnewt-rs");
}

fn main() {
    let statik = cfg!(feature = "static") ||
                 env::var("NEWT_STATIC").is_ok();

    let result = pkg_config::Config::new()
        .atleast_version(NEWT_VERSION)
        .probe("libnewt");

    let lib: Library;
    if statik || result.is_err() {
        find_gnu_make();
        lib = build_libs();
    } else {
        lib = result.unwrap();
    }
    build_c(&lib);
}
