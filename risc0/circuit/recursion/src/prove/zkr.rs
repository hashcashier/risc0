// Copyright 2025 RISC Zero, Inc.
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

use std::io::{Cursor, Read as _};

use anyhow::{bail, Context as _, Result};

use super::Program;

const ZKR_ZIP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/recursion_zkr.zip"));

// Browser sessions load the same one or two programs (e.g. `join.zkr`,
// 23.9 MiB decoded) once per recursion proof; DEFLATE plus the
// per-element field conversion dominates the per-proof CPU setup cost.
// Cache the decoded `Program` per (name, po2) and clone on hit (a
// `Vec` memcpy, ~ms). wasm32-only: native provers keep the untouched
// path so long-lived multi-po2 services don't pin every program.
#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
thread_local! {
    static ZKR_CACHE: std::cell::RefCell<std::collections::HashMap<(String, usize), Program>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

#[cfg(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown"))]
pub fn get_zkr(name: &str, po2: usize) -> Result<Program> {
    let cached = ZKR_CACHE.with(|cache| cache.borrow().get(&(name.to_string(), po2)).cloned());
    let _timer = risc0_zkp::hal::webgpu::WebGpuStageTimer::new(format!(
        "recursion_program_load name={name} po2={po2} cached={}",
        cached.is_some()
    ));
    if let Some(program) = cached {
        return Ok(program);
    }
    let program = load_zkr(name, po2)?;
    ZKR_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert((name.to_string(), po2), program.clone())
    });
    Ok(program)
}

#[cfg(not(all(feature = "webgpu", target_arch = "wasm32", target_os = "unknown")))]
pub fn get_zkr(name: &str, po2: usize) -> Result<Program> {
    load_zkr(name, po2)
}

fn load_zkr(name: &str, po2: usize) -> Result<Program> {
    let mut zip = zip::ZipArchive::new(Cursor::new(ZKR_ZIP))?;
    let encoded = extract_zkr(&mut zip, name)?;
    Ok(Program::from_encoded(&encoded, po2))
}

/// Iterate over all provided zkr programs.
///
/// ```rust
/// let listing = risc0_circuit_recursion::prove::zkr::get_all_zkrs().unwrap();
/// println!("{}", listing.into_iter().map(|(name, _)| name).collect::<Vec<_>>().join("\n"));
/// ```
pub fn get_all_zkrs() -> Result<Vec<(String, Vec<u32>)>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(ZKR_ZIP))?;
    let files: Vec<_> = zip.file_names().map(|name| name.to_string()).collect();
    files
        .into_iter()
        .map(|name| {
            let encoded = extract_zkr(&mut zip, &name)?;
            Ok((name, encoded))
        })
        .collect()
}

fn extract_zkr(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Vec<u32>> {
    let mut f = zip
        .by_name(name)
        .with_context(|| format!("Failed to read {name}"))?;
    let uncompressed_size = f.size() as usize;

    if uncompressed_size % std::mem::size_of::<u32>() != 0 {
        bail!(".zkr is incorrect size");
    }

    let mut u32s = vec![0u32; uncompressed_size / std::mem::size_of::<u32>()];
    f.read_exact(bytemuck::cast_slice_mut(&mut u32s[..]))?;

    Ok(u32s)
}
