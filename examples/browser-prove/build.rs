// Copyright 2026 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{env, fs, path::PathBuf, process::Command};

fn cpp_toolchain() -> PathBuf {
    let rzup = rzup::Rzup::new().unwrap();
    let Some((_version, path)) = rzup
        .get_default_version(&rzup::Component::CppToolchain)
        .unwrap()
    else {
        panic!("RISC Zero C++ toolchain not found. Try running `rzup install cpp`");
    };
    path
}

fn rust_toolchain() -> PathBuf {
    let rzup = rzup::Rzup::new().unwrap();
    let Some((_version, path)) = rzup
        .get_default_version(&rzup::Component::RustToolchain)
        .unwrap()
    else {
        panic!("RISC Zero Rust toolchain not found. Try running `rzup install rust`");
    };
    path
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let examples_dir = manifest_dir.parent().unwrap();
    let guest_dir = examples_dir.join("c-guest").join("guest");

    println!(
        "cargo:rerun-if-changed={}",
        guest_dir.join("main.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        guest_dir.join("riscv32im-risc0-zkvm-elf.ld").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        examples_dir.join("c-guest").join("platform").display()
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("c-guest");
    let guest_elf = out_dir.join("main");
    println!("cargo:rustc-env=C_GUEST_USER_ELF={}", guest_elf.display());

    if env::var("RISC0_SKIP_BUILD").is_ok() {
        return;
    }

    fs::create_dir_all(&out_dir).unwrap();

    let rust_toolchain = rust_toolchain();
    let cargo = rust_toolchain.join("bin/cargo");
    let rustc = rust_toolchain.join("bin/rustc");
    let platform_target_dir = out_dir.join("platform");
    let examples_manifest = examples_dir.join("Cargo.toml");
    let status = Command::new(cargo)
        .env("RUSTC", &rustc)
        .args([
            "rustc",
            "--manifest-path",
            examples_manifest.to_str().unwrap(),
            "-p",
            "zkvm-platform",
            "--target",
            "riscv32im-risc0-zkvm-elf",
            "--lib",
            "--crate-type",
            "staticlib",
            "--release",
        ])
        .arg("--target-dir")
        .arg(&platform_target_dir)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "failed to build c-guest zkvm-platform staticlib"
    );

    let gcc = cpp_toolchain().join("bin/riscv32-unknown-elf-gcc");
    let status = Command::new(gcc)
        .arg("-nostartfiles")
        .arg(guest_dir.join("main.c"))
        .arg("-o")
        .arg(&guest_elf)
        .arg(format!(
            "-L{}",
            platform_target_dir
                .join("riscv32im-risc0-zkvm-elf")
                .join("release")
                .display()
        ))
        .arg("-lzkvm_platform")
        .arg("-T")
        .arg(guest_dir.join("riscv32im-risc0-zkvm-elf.ld"))
        .status()
        .unwrap();
    assert!(status.success(), "failed to build c-guest ELF");
}
