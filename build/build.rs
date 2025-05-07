//
// Copyright (C) 2025 Robert Gill <rtgill82@gmail.com>
//
// This file is a part of newt-sys.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License version 2.1 as published by the Free Software Foundation.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
//

extern crate lazy_static;
extern crate pkg_config;
extern crate regex;

use lazy_static::lazy_static;
use pkg_config::Library;
use regex::Regex;

use std::ffi::OsStr;
use std::path::{Path,PathBuf};
use std::process::{Command,Stdio};
use std::{env, fs};

mod build_config;
use build_config::BuildConfig;

const NEWT_VERSION:   &str = "0.52.25";
const POPT_VERSION:   &str = "1.19";
const SLANG_VERSION:  &str = "2.3.3";

const OLD_CFLAGS_ENV: &str = "_OLD_CFLAGS";

lazy_static! {
    static ref TOP: String = env::var("CARGO_MANIFEST_DIR").unwrap();
    static ref MAKE: &'static str = find_gnu_make();

    static ref CONFIG_GUESS: String = format!("{}/gnuconfig/config.guess", crate_path());
    static ref CONFIG_SUB: String = format!("{}/gnuconfig/config.sub", crate_path());
}

#[inline]
fn crate_path() -> &'static str {
    &TOP
}

#[inline]
fn make() -> &'static str {
    &MAKE
}

fn check_make(make: &str) -> bool {
    let cmd = Command::new(make)
        .stdin(Stdio::null())
        .args(["-f", "-", "--version"])
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

fn update_gnuconfig_files(cfg: &BuildConfig) {
    let autoconf_aux_path = cfg.autoconf_aux_path().unwrap().display();

    let dest = format!("{}/config.guess", autoconf_aux_path);
    fs::copy(&*CONFIG_GUESS, dest).unwrap();

    let dest = format!("{}/config.sub", autoconf_aux_path);
    fs::copy(&*CONFIG_SUB, dest).unwrap();
}

fn append_pkg_config_path(path: &Path) {
    if let Ok(pkg_config_path) = env::var("PKG_CONFIG_PATH") {
        let new_path = format!("{}:{}", pkg_config_path, path.display());
        env::set_var("PKG_CONFIG_PATH", new_path);
    } else {
        env::set_var("PKG_CONFIG_PATH", path);
    }
}

fn build_newt(version: &str, cfg: &BuildConfig) -> Library {
    Command::new("tar").args([OsStr::new("xzf"), cfg.archive_path().as_ref()])
        .args([OsStr::new("-C"), cfg.build_prefix().as_ref()])
        .status().expect("error running tar");

    env::set_current_dir(cfg.src_path())
        .expect("unable to change directory");
    Command::new("./configure")
        .args([OsStr::new("--prefix"), cfg.install_prefix().as_ref()])
        .args(["--host", cfg.target()])
        .arg("--disable-nls")
        .arg("--without-python")
        .arg("--without-tcl")
        .status().expect("error running configure");

    Command::new(make())
        .arg("install")
        .status().expect("error running make");

    append_pkg_config_path(cfg.pkg_config_path());
    pkg_config::Config::new()
        .atleast_version(version)
        .statik(true)
        .probe("libnewt")
        .expect("error running pkg-config")
}

fn build_popt(version: &str, cfg: &BuildConfig) -> Library {
    Command::new("tar").args([OsStr::new("xzf"), cfg.archive_path().as_ref()])
        .args([OsStr::new("-C"), cfg.build_prefix().as_ref()])
        .status().expect("error running tar");

    update_gnuconfig_files(cfg);
    env::set_current_dir(cfg.src_path())
        .expect("unable to change directory");
    Command::new("./configure")
        .args([OsStr::new("--prefix"), cfg.install_prefix().as_ref()])
        .args(["--host", cfg.target()])
        .arg("--disable-nls")
        .arg("--disable-rpath")
        .status().expect("error running configure");

    Command::new(make())
        .arg("install")
        .status().expect("error running make");

    append_pkg_config_path(cfg.pkg_config_path());
    pkg_config::Config::new()
        .atleast_version(version)
        .arg("--cflags")
        .statik(true)
        .probe("popt")
        .expect("error running pkg-config")
}

fn build_slang(version: &str, cfg: &BuildConfig) -> Library {
    cflags_set_fpic();
    Command::new("tar").args([OsStr::new("xjf"), cfg.archive_path().as_ref()])
        .args([OsStr::new("-C"), cfg.build_prefix().as_ref()])
        .status().expect("error running tar");

    update_gnuconfig_files(cfg);
    env::set_current_dir(cfg.src_path())
        .expect("unable to change directory");
    Command::new("./configure")
        .args([OsStr::new("--prefix"), cfg.install_prefix().as_ref()])
        .args(["--host", cfg.target()])
        .status().expect("error running configure");

    Command::new(make())
        .arg("install-static")
        .status().expect("error running make");

    cflags_restore();
    append_pkg_config_path(cfg.pkg_config_path());
    pkg_config::Config::new()
        .atleast_version(version)
        .arg("--cflags")
        .statik(true)
        .probe("slang")
        .expect("error running pkg-config")
}

fn compiler() -> Option<PathBuf> {
    let mut cc_cfg = cc::Build::new();
    cc_cfg.cargo_metadata(false)
          .warnings(false);

    let _compiler = cc_cfg.get_compiler();
    let compiler = _compiler.path();
    let mut cmd = Command::new("sh");
    cmd.stdout(Stdio::null())
        .arg("-c")
        .arg(&format!("command -v \"{}\"", compiler.display()));

    match cmd.status() {
        Ok(status) => {
            if status.success() {
                Some(compiler.to_owned())
            } else {
                None
            }
        },
        Err(_) => None
    }
}

fn set_cc() {
    if let Some(compiler) = compiler() {
        if let Err(_) = env::var("CC") {
            env::set_var("CC", compiler.as_os_str());
        }
    }
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

    if !include_paths.is_empty() {
        env::set_var("CPPFLAGS", include_paths)
    }

    if !link_paths.is_empty() {
        env::set_var("LDFLAGS", link_paths)
    }
}

fn unset_env_libs() {
    env::remove_var("CPPFLAGS");
    env::remove_var("LDFLAGS");
}

fn cflags_set_fpic() {
    let mut cflags = env::var("CFLAGS").unwrap_or_default();
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

fn build(package: &str, version: &str, libs: Option<&[Box<Library>]>) -> Library {
    let mut build_cfg = BuildConfig::new(package, version);
    match package {
        "popt"  => build_cfg.set_autoconf_aux_path("build-aux"),
        "slang" => build_cfg.set_autoconf_aux_path("autoconf"),
        _       => { }
    }

    if let Some(libs) = libs { export_env_libs(libs) }
    let old_dir = env::current_dir()
        .expect("unable to read current directory");
    fs::create_dir_all(build_cfg.build_prefix())
        .expect("unable to create build directory");
    env::set_current_dir(build_cfg.build_prefix())
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
    library
}

fn build_libs() -> Library {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut libraries: Vec<Box<Library>> = Vec::new();

    env::set_var("PKG_CONFIG_SYSROOT_DIR", &out_dir);
    let library = Box::new(build("popt", POPT_VERSION, None));
    libraries.push(library);

    let library = Box::new(build("slang", SLANG_VERSION, None));
    libraries.push(library);

    build("newt", NEWT_VERSION, Some(&libraries))
}

fn build_c(lib: &Library) {
    let mut build = cc::Build::new();
    build.file("src/colorset_custom.c");

    if let Ok(cc) = env::var("CC") {
        build.compiler(cc);
    }

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

    set_cc();
    let lib: Library = if statik || result.is_err() {
        find_gnu_make();
        build_libs()
    } else {
        result.unwrap()
    };
    build_c(&lib);

    if statik {
        println!("cargo:rustc-link-lib=static=newt");
    }
}
