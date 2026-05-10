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

#[cfg(all(test, target_arch = "wasm32", target_os = "unknown"))]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use risc0_zkp::{
        core::{digest::Digest, hash::poseidon2::Poseidon2HashSuite},
        field::{
            baby_bear::{BabyBearElem, BabyBearExtElem},
            Elem as _,
        },
        hal::{
            webgpu::{WebGpuBuffer, WebGpuHal},
            Buffer as _, Hal as _,
        },
    };
    use risc0_zkvm::{
        serde::{from_slice, to_vec},
        webgpu_prover, Assumption, Executor, ExecutorEnv, ExitCode, ProveInfo, Prover, ProverOpts,
        Receipt, WebGpuProver, ALLOWED_CONTROL_ROOT,
    };
    use wasm_bindgen_test::{console_log, wasm_bindgen_test, wasm_bindgen_test_configure};

    wasm_bindgen_test_configure!(run_in_worker);

    async fn init_prover() -> Rc<WebGpuProver> {
        console_error_panic_hook::set_once();

        let prover = webgpu_prover().await.unwrap();
        assert_eq!(prover.get_name(), "webgpu");
        prover
    }

    async fn assert_gpu_buffer_matches_cpu<T>(hal: &WebGpuHal, name: &str, buffer: &WebGpuBuffer<T>)
    where
        T: Clone + bytemuck::NoUninit,
    {
        assert_eq!(buffer.byte_offset(), 0, "{name}: expected full buffer");
        let byte_len = (buffer.size() * std::mem::size_of::<T>()) as u64;
        let gpu_bytes = hal
            .read_buffer(buffer.raw_buffer().expect("non-empty GPU buffer"), byte_len)
            .await
            .unwrap_or_else(|err| panic!("{name}: GPU readback failed: {err}"));
        let cpu = buffer.to_vec();
        let cpu_bytes: &[u8] = bytemuck::cast_slice(cpu.as_slice());
        assert_eq!(gpu_bytes.as_slice(), cpu_bytes, "{name}: GPU/CPU mismatch");
    }

    fn elem(seed: usize) -> BabyBearElem {
        BabyBearElem::new((seed as u32).wrapping_mul(0x1f12bb5).wrapping_add(0x12345))
    }

    fn ext_elem(seed: usize) -> BabyBearExtElem {
        BabyBearExtElem::new(elem(seed), elem(seed + 1), elem(seed + 2), elem(seed + 3))
    }

    fn digest(seed: u32) -> Digest {
        Digest::from([
            seed.wrapping_mul(17).wrapping_add(1),
            seed.wrapping_mul(17).wrapping_add(2),
            seed.wrapping_mul(17).wrapping_add(3),
            seed.wrapping_mul(17).wrapping_add(4),
            seed.wrapping_mul(17).wrapping_add(5),
            seed.wrapping_mul(17).wrapping_add(6),
            seed.wrapping_mul(17).wrapping_add(7),
            seed.wrapping_mul(17).wrapping_add(8),
        ])
    }

    fn log_webgpu_diagnostics(prover: &WebGpuProver, name: &str) {
        let diagnostics = prover.diagnostics();
        assert!(
            diagnostics.gpu_dispatches > 0,
            "{name}: WebGPU proof path did not dispatch any GPU work"
        );
        assert_eq!(
            diagnostics.cpu_only_ops, 0,
            "{name}: WebGPU proof path used CPU-only HAL operations"
        );
        console_log!(
            "browser-prove:webgpu {name}: gpu_dispatches={} cpu_mirrors={} cpu_fallbacks={} cpu_only_ops={} uploads={} upload_bytes={} device_copies={} device_copy_bytes={} readbacks={} readback_bytes={} buffers={} buffer_bytes={}",
            diagnostics.gpu_dispatches,
            diagnostics.cpu_mirrors,
            diagnostics.cpu_fallbacks,
            diagnostics.cpu_only_ops,
            diagnostics.host_to_gpu_uploads,
            diagnostics.host_to_gpu_bytes,
            diagnostics.device_copies,
            diagnostics.device_copy_bytes,
            diagnostics.readbacks,
            diagnostics.readback_bytes,
            diagnostics.buffers_allocated,
            diagnostics.bytes_allocated,
        );
        for op in diagnostics.ops {
            console_log!(
                "browser-prove:webgpu-op {name}: op={} gpu_dispatches={} cpu_mirrors={} cpu_fallbacks={} cpu_only_ops={}",
                op.name,
                op.gpu_dispatches,
                op.cpu_mirrors,
                op.cpu_fallbacks,
                op.cpu_only_ops,
            );
        }
    }

    fn prove_succinct_info(
        prover: &WebGpuProver,
        name: &str,
        env: ExecutorEnv,
        elf: &[u8],
        image_id: [u32; 8],
        opts: &ProverOpts,
    ) -> ProveInfo {
        console_log!("browser-prove:start {name}");
        prover.reset_diagnostics();
        let prove_info = prover
            .prove_with_opts(env, elf, opts)
            .unwrap_or_else(|err| panic!("{name}: prove failed: {err}"));

        prove_info
            .receipt
            .inner
            .succinct()
            .unwrap_or_else(|_| panic!("{name}: receipt is not succinct"));
        prove_info
            .receipt
            .verify(image_id)
            .unwrap_or_else(|err| panic!("{name}: receipt verification failed: {err}"));
        console_log!(
            "browser-prove:done {name}: segments={} user_cycles={} total_cycles={}",
            prove_info.stats.segments,
            prove_info.stats.user_cycles,
            prove_info.stats.total_cycles
        );
        log_webgpu_diagnostics(prover, name);
        prove_info
    }

    fn prove_succinct(
        prover: &WebGpuProver,
        name: &str,
        env: ExecutorEnv,
        elf: &[u8],
        image_id: [u32; 8],
    ) -> Receipt {
        prove_succinct_info(prover, name, env, elf, image_id, &ProverOpts::succinct()).receipt
    }

    fn prove_succinct_integrity(
        prover: &WebGpuProver,
        name: &str,
        env: ExecutorEnv,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> Receipt {
        console_log!("browser-prove:start {name}");
        prover.reset_diagnostics();
        let receipt = prover
            .prove_with_opts(env, elf, opts)
            .unwrap_or_else(|err| panic!("{name}: prove failed: {err}"))
            .receipt;

        receipt
            .inner
            .succinct()
            .unwrap_or_else(|_| panic!("{name}: receipt is not succinct"));
        receipt
            .verify_integrity_with_context(&Default::default())
            .unwrap_or_else(|err| panic!("{name}: receipt integrity verification failed: {err}"));
        console_log!("browser-prove:done {name}");
        log_webgpu_diagnostics(prover, name);
        receipt
    }

    fn prove_multi(prover: &WebGpuProver, name: &str, spec: impl serde::Serialize) -> Receipt {
        use risc0_zkvm_methods::{MULTI_TEST_ELF, MULTI_TEST_ID};

        let env = ExecutorEnv::builder()
            .write(&spec)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(prover, name, env, MULTI_TEST_ELF, MULTI_TEST_ID)
    }

    #[wasm_bindgen_test(async)]
    async fn webgpu_hal_ntt_gpu_results_match_cpu() {
        console_error_panic_hook::set_once();

        let hal = WebGpuHal::new(Poseidon2HashSuite::new_suite())
            .await
            .unwrap();
        let count = 3;
        let in_size = 16;
        let expand_bits = 2;
        let out_size = in_size << expand_bits;
        let input = (0..count * in_size).map(elem).collect::<Vec<_>>();
        let input = hal.copy_from_elem("webgpu_hal_ntt_input", &input);
        let output = hal.alloc_elem("webgpu_hal_ntt_output", count * out_size);
        hal.batch_expand_into_evaluate_ntt(&output, &input, count, expand_bits);
        assert_gpu_buffer_matches_cpu(&hal, "batch_expand_into_evaluate_ntt", &output).await;

        let io = (0..count * out_size)
            .map(|idx| elem(idx + 1000))
            .collect::<Vec<_>>();
        let io = hal.copy_from_elem("webgpu_hal_intt_io", &io);
        hal.batch_interpolate_ntt(&io, count);
        assert_gpu_buffer_matches_cpu(&hal, "batch_interpolate_ntt", &io).await;
    }

    #[wasm_bindgen_test(async)]
    async fn webgpu_hal_core_gpu_results_match_cpu() {
        console_error_panic_hook::set_once();

        let hal = WebGpuHal::new(Poseidon2HashSuite::new_suite())
            .await
            .unwrap();

        let bit_reverse = hal.copy_from_elem(
            "webgpu_hal_bit_reverse",
            &(0..96).map(elem).collect::<Vec<_>>(),
        );
        hal.batch_bit_reverse(&bit_reverse, 3);
        assert_gpu_buffer_matches_cpu(&hal, "batch_bit_reverse", &bit_reverse).await;

        let zk_shift = hal.copy_from_elem(
            "webgpu_hal_zk_shift",
            &(0..128).map(|idx| elem(idx + 100)).collect::<Vec<_>>(),
        );
        hal.zk_shift(&zk_shift, 2);
        assert_gpu_buffer_matches_cpu(&hal, "zk_shift", &zk_shift).await;

        let coeffs = hal.copy_from_elem(
            "webgpu_hal_eval_coeffs",
            &(0..48).map(|idx| elem(idx + 200)).collect::<Vec<_>>(),
        );
        let which = hal.copy_from_u32("webgpu_hal_eval_which", &[0, 1, 2, 1, 0]);
        let xs = hal.copy_from_extelem(
            "webgpu_hal_eval_xs",
            &(0..5).map(|idx| ext_elem(idx + 300)).collect::<Vec<_>>(),
        );
        let eval_out = hal.alloc_extelem("webgpu_hal_eval_out", 5);
        hal.batch_evaluate_any(&coeffs, 3, &which, &xs, &eval_out);
        assert_gpu_buffer_matches_cpu(&hal, "batch_evaluate_any", &eval_out).await;

        let fri_input = hal.copy_from_elem(
            "webgpu_hal_fri_input",
            &(0..192).map(|idx| elem(idx + 400)).collect::<Vec<_>>(),
        );
        let fri_output = hal.alloc_elem("webgpu_hal_fri_output", 12);
        hal.fri_fold(&fri_output, &fri_input, &ext_elem(500));
        assert_gpu_buffer_matches_cpu(&hal, "fri_fold", &fri_output).await;

        let mix_output = hal.alloc_extelem_zeroed("webgpu_hal_mix_output", 24);
        let mix_input = hal.copy_from_elem(
            "webgpu_hal_mix_input",
            &(0..40).map(|idx| elem(idx + 600)).collect::<Vec<_>>(),
        );
        let combos = hal.copy_from_u32("webgpu_hal_mix_combos", &[0, 2, 1, 2, 0]);
        hal.mix_poly_coeffs(
            &mix_output,
            &ext_elem(700),
            &ext_elem(800),
            &mix_input,
            &combos,
            5,
            8,
        );
        assert_gpu_buffer_matches_cpu(&hal, "mix_poly_coeffs", &mix_output).await;

        let hash_matrix = hal.copy_from_elem(
            "webgpu_hal_hash_matrix",
            &(0..35).map(|idx| elem(idx + 900)).collect::<Vec<_>>(),
        );
        let hash_rows = hal.alloc_digest("webgpu_hal_hash_rows", 5);
        hal.hash_rows(&hash_rows, &hash_matrix);
        assert_gpu_buffer_matches_cpu(&hal, "hash_rows", &hash_rows).await;

        let hash_fold = hal.copy_from_digest(
            "webgpu_hal_hash_fold",
            &(0..16).map(digest).collect::<Vec<_>>(),
        );
        hal.hash_fold(&hash_fold, 8, 4);
        assert_gpu_buffer_matches_cpu(&hal, "hash_fold", &hash_fold).await;

        let add_output = hal.alloc_elem("webgpu_hal_add_output", 16);
        let add_input1 = hal.copy_from_elem(
            "webgpu_hal_add_input1",
            &(0..16).map(|idx| elem(idx + 1000)).collect::<Vec<_>>(),
        );
        let add_input2 = hal.copy_from_elem(
            "webgpu_hal_add_input2",
            &(0..16).map(|idx| elem(idx + 1100)).collect::<Vec<_>>(),
        );
        hal.eltwise_add_elem(&add_output, &add_input1, &add_input2);
        assert_gpu_buffer_matches_cpu(&hal, "eltwise_add_elem", &add_output).await;

        let sum_output = hal.alloc_elem("webgpu_hal_sum_output", 12);
        let sum_input = hal.copy_from_extelem(
            "webgpu_hal_sum_input",
            &(0..15).map(|idx| ext_elem(idx + 1200)).collect::<Vec<_>>(),
        );
        hal.eltwise_sum_extelem(&sum_output, &sum_input);
        assert_gpu_buffer_matches_cpu(&hal, "eltwise_sum_extelem", &sum_output).await;

        let copy_input = hal.copy_from_elem(
            "webgpu_hal_copy_input",
            &(0..16).map(|idx| elem(idx + 1300)).collect::<Vec<_>>(),
        );
        let copy_output = hal.alloc_elem("webgpu_hal_copy_output", 16);
        hal.eltwise_copy_elem(&copy_output, &copy_input);
        assert_gpu_buffer_matches_cpu(&hal, "eltwise_copy_elem", &copy_output).await;

        let copy_slice_into = hal.copy_from_elem(
            "webgpu_hal_copy_slice_into",
            &(0..30).map(|idx| elem(idx + 1400)).collect::<Vec<_>>(),
        );
        let copy_slice_from = (0..40).map(|idx| elem(idx + 1500)).collect::<Vec<_>>();
        hal.eltwise_copy_elem_slice(&copy_slice_into, &copy_slice_from, 3, 4, 5, 8, 7, 6);
        assert_gpu_buffer_matches_cpu(&hal, "eltwise_copy_elem_slice", &copy_slice_into).await;

        let zeroize = hal.copy_from_elem(
            "webgpu_hal_zeroize",
            &[
                elem(1600),
                BabyBearElem::INVALID,
                elem(1601),
                BabyBearElem::INVALID,
            ],
        );
        hal.eltwise_zeroize_elem(&zeroize);
        assert_gpu_buffer_matches_cpu(&hal, "eltwise_zeroize_elem", &zeroize).await;

        let gather_src = hal.copy_from_elem(
            "webgpu_hal_gather_src",
            &(0..80).map(|idx| elem(idx + 1700)).collect::<Vec<_>>(),
        );
        let gather_dst = hal.alloc_elem("webgpu_hal_gather_dst", 10);
        hal.gather_sample(&gather_dst, &gather_src, 3, 10, 8);
        assert_gpu_buffer_matches_cpu(&hal, "gather_sample", &gather_dst).await;

        let scatter_into = hal.copy_from_elem(
            "webgpu_hal_scatter_into",
            &(0..12).map(|idx| elem(idx + 1800)).collect::<Vec<_>>(),
        );
        let scatter_values = (0..5).map(|idx| elem(idx + 1900)).collect::<Vec<_>>();
        hal.scatter(&scatter_into, &[0, 2, 5], &[4, 1, 7, 3, 9], &scatter_values);
        assert_gpu_buffer_matches_cpu(&hal, "scatter", &scatter_into).await;

        let prefix = hal.copy_from_extelem(
            "webgpu_hal_prefix",
            &(0..8).map(|idx| ext_elem(idx + 2000)).collect::<Vec<_>>(),
        );
        hal.prefix_products(&prefix);
        assert_gpu_buffer_matches_cpu(&hal, "prefix_products", &prefix).await;
    }

    #[wasm_bindgen_test(async)]
    async fn internal_cfg_succinct_receipt_verifies() {
        use risc0_zkvm_methods::{CFG_ELF, CFG_ID};

        let prover = init_prover().await;
        let env = ExecutorEnv::builder().build().unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/cfg",
            env,
            CFG_ELF,
            CFG_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_prover_api_and_execution_modes_verify() {
        use risc0_zkvm_methods::{multi_test::MultiTestSpec, MULTI_TEST_ELF, MULTI_TEST_ID};

        const MEM_POS: u32 = 0x0020_0600;

        let prover = init_prover().await;

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::DoNothing)
            .unwrap()
            .build()
            .unwrap();
        let composite = prover
            .prove_with_opts(env, MULTI_TEST_ELF, &ProverOpts::composite())
            .expect("multi_test/do_nothing: composite prove failed")
            .receipt;
        composite
            .inner
            .composite()
            .expect("multi_test/do_nothing: receipt is not composite");
        composite
            .verify(MULTI_TEST_ID)
            .expect("multi_test/do_nothing: composite receipt verification failed");

        let compressed = prover
            .compress(&ProverOpts::succinct(), &composite)
            .expect("multi_test/do_nothing: composite compression failed");
        compressed
            .inner
            .succinct()
            .expect("multi_test/do_nothing: compressed receipt is not succinct");
        compressed
            .verify(MULTI_TEST_ID)
            .expect("multi_test/do_nothing: compressed receipt verification failed");

        let bytes = b"browser echo parity".to_vec();
        let receipt = prove_multi(
            prover.as_ref(),
            "multi_test/echo",
            MultiTestSpec::Echo {
                bytes: bytes.clone(),
            },
        );
        assert_eq!(receipt.journal.bytes, bytes);

        prove_multi(
            prover.as_ref(),
            "multi_test/sha_cycle_count",
            MultiTestSpec::ShaCycleCount,
        );

        let mut output = Vec::new();
        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::ReadWriteMem {
                values: vec![(MEM_POS, 0x1234_5678), (MEM_POS, 0)],
            })
            .unwrap()
            .stdout(&mut output)
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "multi_test/read_write_mem",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
        assert!(receipt.journal.bytes.is_empty());
        assert_eq!(from_slice::<u32, u8>(&output).unwrap(), 0x1234_5678);

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::PauseResume(7))
            .unwrap()
            .build()
            .unwrap();
        let session = prover
            .execute(env, MULTI_TEST_ELF)
            .expect("multi_test/pause_resume: execute failed");
        assert_eq!(session.exit_code, ExitCode::Paused(7));

        // `RunUnconstrained { unconstrained: true }` uses SYS_FORK, which is
        // disabled in the native syscall table in this checkout and is covered
        // by an ignored native test. It is classified as a native-disabled
        // fixture rather than active browser proving parity.
    }

    #[wasm_bindgen_test(async)]
    async fn native_zkvm_method_guests_succinct_receipts_verify() {
        use risc0_zkvm::sha::Digest;
        use risc0_zkvm_methods::{
            bench::BenchmarkSpec, BENCH_ELF, BENCH_ID, BLST_ELF, BLST_ID, FIB_ELF, FIB_ID,
            HEAP_ELF, HEAP_ID, HELLO_COMMIT_ELF, HELLO_COMMIT_ID, RAND2_ELF, RAND2_ID,
            SLICE_IO_ELF, SLICE_IO_ID, STANDARD_LIB_ELF, STANDARD_LIB_ID, TEST_FEATURE_ELF,
            TEST_FEATURE_ID, VERIFY_ELF, VERIFY_ID, ZKVM_527_ELF, ZKVM_527_ID,
        };

        let prover = init_prover().await;

        let exec_env = ExecutorEnv::builder()
            .write(&10u32)
            .unwrap()
            .build()
            .unwrap();
        let session = prover.execute(exec_env, FIB_ELF).unwrap();
        assert_eq!(session.exit_code, ExitCode::Halted(0));
        assert_eq!(session.journal.decode::<u64>().unwrap(), 55);
        assert!(session.receipt_claim.is_some());

        let env = ExecutorEnv::builder().build().unwrap();
        let hello_receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/hello_commit",
            env,
            HELLO_COMMIT_ELF,
            HELLO_COMMIT_ID,
        );
        assert_eq!(hello_receipt.journal.bytes, b"hello world");

        let env = ExecutorEnv::builder()
            .write(&10u32)
            .unwrap()
            .build()
            .unwrap();
        let fib_receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/fib",
            env,
            FIB_ELF,
            FIB_ID,
        );
        assert_eq!(fib_receipt.journal.decode::<u64>().unwrap(), 55);

        let slice = b"browser-native-slice-io";
        let env = ExecutorEnv::builder()
            .write_slice(&[slice.len() as u32])
            .write_slice(slice)
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/slice_io",
            env,
            SLICE_IO_ELF,
            SLICE_IO_ID,
        );
        assert_eq!(receipt.journal.bytes, slice);

        let env = ExecutorEnv::builder()
            .write(&3u32)
            .unwrap()
            .env_var("ALL_FORKS", "testing")
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/heap",
            env,
            HEAP_ELF,
            HEAP_ID,
        );
        assert_eq!(receipt.journal.decode::<u32>().unwrap(), 0);

        let env = ExecutorEnv::builder().build().unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/zkvm-527",
            env,
            ZKVM_527_ELF,
            ZKVM_527_ID,
        );

        let env = ExecutorEnv::builder().build().unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/rand2",
            env,
            RAND2_ELF,
            RAND2_ID,
        );

        let env = ExecutorEnv::builder()
            .env_var("TEST_MODE", "ENV_VARS")
            .env_var("ENV_VAR1", "val1")
            .env_var("ENV_VAR2", "")
            .stdin("ENV_VAR1\nENV_VAR2\nENV_VAR3".as_bytes())
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/standard_lib/env",
            env,
            STANDARD_LIB_ELF,
            STANDARD_LIB_ID,
        );
        assert_eq!(
            std::str::from_utf8(&receipt.journal.bytes).unwrap(),
            "ENV_VAR1=val1\nENV_VAR2=\n!ENV_VAR3\n"
        );

        let args = vec![
            "grep".to_string(),
            "-c".to_string(),
            "foo bar".to_string(),
            "-".to_string(),
        ];
        let env = ExecutorEnv::builder()
            .env_var("TEST_MODE", "ARGS")
            .args(&args)
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/standard_lib/args",
            env,
            STANDARD_LIB_ELF,
            STANDARD_LIB_ID,
        );
        assert_eq!(receipt.journal.decode::<Vec<String>>().unwrap(), args);

        let input = b"1234567";
        let env = ExecutorEnv::builder()
            .env_var("TEST_MODE", "BUF_READ")
            .write(&9usize)
            .unwrap()
            .write_slice(input.as_slice())
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/standard_lib/buf_read",
            env,
            STANDARD_LIB_ELF,
            STANDARD_LIB_ID,
        );
        assert_eq!(receipt.journal.bytes, input);

        let env = ExecutorEnv::builder().build().unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/blst",
            env,
            BLST_ELF,
            BLST_ID,
        );
        assert_eq!(
            receipt.journal.decode::<String>().unwrap(),
            "blst is such a blast"
        );

        let env = ExecutorEnv::builder()
            .write(&BenchmarkSpec::SimpleLoop { iters: 16 })
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/bench/simple_loop",
            env,
            BENCH_ELF,
            BENCH_ID,
        );

        let env = ExecutorEnv::builder().build().unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/test_feature",
            env,
            TEST_FEATURE_ELF,
            TEST_FEATURE_ID,
        );

        let verify_input = (
            hello_receipt,
            Digest::from(HELLO_COMMIT_ID),
            false, /* dev_mode */
        );
        let env = ExecutorEnv::builder()
            .write(&verify_input)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/verify",
            env,
            VERIFY_ELF,
            VERIFY_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_blst_succinct_receipt_verifies() {
        use risc0_zkvm_methods::{BLST_ELF, BLST_ID};

        let prover = init_prover().await;
        let env = ExecutorEnv::builder().build().unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/blst",
            env,
            BLST_ELF,
            BLST_ID,
        );
        assert_eq!(
            receipt.journal.decode::<String>().unwrap(),
            "blst is such a blast"
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_bench_succinct_receipt_verifies() {
        use risc0_zkvm_methods::{bench::BenchmarkSpec, BENCH_ELF, BENCH_ID};

        let prover = init_prover().await;
        let env = ExecutorEnv::builder()
            .write(&BenchmarkSpec::SimpleLoop { iters: 16 })
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/bench/simple_loop",
            env,
            BENCH_ELF,
            BENCH_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_test_feature_succinct_receipt_verifies() {
        use risc0_zkvm_methods::{TEST_FEATURE_ELF, TEST_FEATURE_ID};

        let prover = init_prover().await;
        let env = ExecutorEnv::builder().build().unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/test_feature",
            env,
            TEST_FEATURE_ELF,
            TEST_FEATURE_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_guest_verify_succinct_receipt_verifies() {
        use risc0_zkvm::sha::Digest;
        use risc0_zkvm_methods::{HELLO_COMMIT_ELF, HELLO_COMMIT_ID, VERIFY_ELF, VERIFY_ID};

        let prover = init_prover().await;
        let hello_receipt = prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/verify/hello_commit",
            ExecutorEnv::builder().build().unwrap(),
            HELLO_COMMIT_ELF,
            HELLO_COMMIT_ID,
        );
        let verify_input = (
            hello_receipt,
            Digest::from(HELLO_COMMIT_ID),
            false, /* dev_mode */
        );
        let env = ExecutorEnv::builder()
            .write(&verify_input)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "risc0-zkvm-methods/verify",
            env,
            VERIFY_ELF,
            VERIFY_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_syscall_and_io_succinct_receipts_verify() {
        use bytes::Bytes;
        use risc0_zkvm::sha::{Digest, Digestible};
        use risc0_zkvm_methods::{
            multi_test::{MultiTestSpec, SYS_MULTI_TEST, SYS_MULTI_TEST_WORDS},
            MULTI_TEST_ELF, MULTI_TEST_ID,
        };

        const FD: u32 = 123;

        let prover = init_prover().await;

        let expected: Vec<Bytes> = vec![
            Bytes::from_static(b""),
            Bytes::from_static(b"H"),
            Bytes::from_static(b"He"),
            Bytes::from_static(b"Hel"),
            Bytes::from_static(b"Hell"),
            Bytes::from_static(b"Hello"),
        ];
        let actual: RefCell<Vec<Bytes>> = RefCell::new(Vec::new());
        {
            let env = ExecutorEnv::builder()
                .write(&MultiTestSpec::Syscall {
                    count: expected.len() as u32 - 1,
                })
                .unwrap()
                .io_callback(SYS_MULTI_TEST, |buf| {
                    let mut actual = actual.borrow_mut();
                    let response = expected[actual.len() + 1].clone();
                    actual.push(buf);
                    Ok(response)
                })
                .build()
                .unwrap();
            prove_succinct(
                prover.as_ref(),
                "multi_test/syscall",
                env,
                MULTI_TEST_ELF,
                MULTI_TEST_ID,
            );
        }
        assert_eq!(*actual.borrow(), expected[..expected.len() - 1]);

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::SyscallWords)
            .unwrap()
            .io_callback(SYS_MULTI_TEST_WORDS, Ok)
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "multi_test/syscall_words",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let digest = Digest::from([1, 2, 3, 4, 5, 6, 7, 8]);
        let env = ExecutorEnv::builder()
            .input_digest(digest)
            .write(&MultiTestSpec::SysInput(digest))
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct_integrity(
            prover.as_ref(),
            "multi_test/sys_input",
            env,
            MULTI_TEST_ELF,
            &ProverOpts::succinct(),
        );
        let opened_claim = receipt.claim().unwrap();
        let claim = opened_claim.as_value().unwrap();
        assert_eq!(claim.exit_code, ExitCode::Halted(0));
        assert_eq!(claim.pre.digest(), Digest::from(MULTI_TEST_ID));
        assert_eq!(claim.input.digest(), digest);

        let initial = b"abcdefghijkl".to_vec();
        let readbuf = b"ABCDEFG".to_vec();
        let spec = MultiTestSpec::SysRead {
            fd: FD,
            buf: initial,
            pos_and_len: vec![(2, 6), (8, 4)],
        };
        let env = ExecutorEnv::builder()
            .read_fd(FD, &readbuf[..])
            .write(&spec)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "multi_test/sys_read",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
        let (actual, num_read): (Vec<u8>, Vec<usize>) = receipt.journal.decode().unwrap();
        assert_eq!(num_read, vec![6, 1]);
        assert_eq!(actual, b"abABCDEFG\0\0\0".to_vec());

        let mut stdout = Vec::new();
        {
            let env = ExecutorEnv::builder()
                .read_fd(FD, "Hello world!".as_bytes())
                .write(&MultiTestSpec::EchoStdout { nbytes: 5, fd: FD })
                .unwrap()
                .stdout(&mut stdout)
                .build()
                .unwrap();
            prove_succinct(
                prover.as_ref(),
                "multi_test/echo_stdout",
                env,
                MULTI_TEST_ELF,
                MULTI_TEST_ID,
            );
        }
        assert_eq!(stdout, b"Hello world!");

        let words: Vec<u32> = (0..32).collect();
        let env = ExecutorEnv::builder()
            .read_fd(FD, bytemuck::cast_slice(&words))
            .write(&MultiTestSpec::EchoWords {
                fd: FD,
                nwords: words.len() as u32,
            })
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "multi_test/echo_words",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
        let actual: &[u32] = bytemuck::cast_slice(&receipt.journal.bytes);
        assert_eq!(actual, words.as_slice());
    }

    fn prove_accelerator_pre_rsa(prover: &WebGpuProver) {
        use risc0_zkvm_methods::multi_test::MultiTestSpec;

        for (name, spec) in [
            ("multi_test/libm", MultiTestSpec::LibM),
            ("multi_test/poseidon2_basic", MultiTestSpec::Poseidon2Basic),
            ("multi_test/poseidon2_short", MultiTestSpec::Poseidon2Short),
            ("multi_test/poseidon2_long", MultiTestSpec::Poseidon2Long),
            (
                "multi_test/poseidon2_continue",
                MultiTestSpec::Poseidon2Continue,
            ),
            ("multi_test/sha_conforms", MultiTestSpec::ShaConforms),
        ] {
            prove_multi(prover, name, spec);
        }
    }

    fn prove_accelerator_post_rsa(prover: &WebGpuProver) {
        use risc0_zkvm::sha::Digest;
        use risc0_zkvm_methods::{multi_test::MultiTestSpec, MULTI_TEST_ELF, MULTI_TEST_ID};

        for (name, spec) in [
            ("multi_test/do_random", MultiTestSpec::DoRandom),
            ("multi_test/aligned_alloc", MultiTestSpec::AlignedAlloc),
            ("multi_test/alloc_zeroed", MultiTestSpec::AllocZeroed),
            ("multi_test/keccak_update", MultiTestSpec::KeccakUpdate),
            (
                "multi_test/sha_single_keccak",
                MultiTestSpec::ShaSingleKeccak,
            ),
            ("multi_test/sys_keccak", MultiTestSpec::SysKeccak),
        ] {
            prove_multi(prover, name, spec);
        }

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::ShaDigest {
                data: b"abc".to_vec(),
            })
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover,
            "multi_test/sha_digest",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
        let digest = Digest::try_from(receipt.journal.bytes).unwrap();
        assert_eq!(
            hex::encode(digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::ShaDigestIter {
                data: vec![0u8; 32],
                num_iter: 16,
            })
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover,
            "multi_test/sha_digest_iter",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::BigInt {
                count: 3,
                x: [1, 2, 3, 4, 5, 6, 7, 8],
                y: [9, 10, 11, 12, 13, 14, 15, 16],
                modulus: [17, 18, 19, 20, 21, 22, 23, 24],
            })
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover,
            "multi_test/bigint",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        const BIGINT_LEGAL_ADDR: u32 = 0x3000_0000;
        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::BigIntRaw {
                result: BIGINT_LEGAL_ADDR,
                x: BIGINT_LEGAL_ADDR,
                y: BIGINT_LEGAL_ADDR,
                modulus: BIGINT_LEGAL_ADDR,
            })
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover,
            "multi_test/bigint_raw",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::KeccakUpdate2)
            .unwrap()
            .keccak_max_po2(14)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover,
            "multi_test/keccak_update2",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        prove_keccak_union(prover);
    }

    fn prove_keccak_union(prover: &WebGpuProver) {
        use risc0_zkvm_methods::{multi_test::MultiTestSpec, MULTI_TEST_ELF, MULTI_TEST_ID};

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::KeccakUnion(3))
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover,
            "multi_test/keccak_union",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_accelerator_pre_rsa_succinct_receipts_verify() {
        let prover = init_prover().await;
        prove_accelerator_pre_rsa(prover.as_ref());
    }

    #[wasm_bindgen_test(async)]
    async fn native_rsa_compat_succinct_receipt_verify() {
        use risc0_zkvm_methods::multi_test::MultiTestSpec;

        let prover = init_prover().await;
        prove_multi(
            prover.as_ref(),
            "multi_test/rsa_compat",
            MultiTestSpec::RsaCompat,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn native_accelerator_post_rsa_succinct_receipts_verify() {
        let prover = init_prover().await;
        prove_accelerator_post_rsa(prover.as_ref());
    }

    #[wasm_bindgen_test(async)]
    async fn native_keccak_union_succinct_receipt_verify() {
        let prover = init_prover().await;
        prove_keccak_union(prover.as_ref());
    }

    #[wasm_bindgen_test(async)]
    async fn native_accelerator_and_precompile_succinct_receipts_verify() {
        use risc0_zkvm_methods::multi_test::MultiTestSpec;

        let prover = init_prover().await;

        prove_accelerator_pre_rsa(prover.as_ref());
        prove_multi(
            prover.as_ref(),
            "multi_test/rsa_compat",
            MultiTestSpec::RsaCompat,
        );
        prove_accelerator_post_rsa(prover.as_ref());
    }

    #[wasm_bindgen_test(async)]
    async fn native_assumption_continuation_povw_and_guest_error_receipts_verify() {
        use risc0_binfmt::{PovwJobId, PovwLogId};
        use risc0_zkvm::sha::Digestible;
        use risc0_zkvm_methods::{
            multi_test::MultiTestSpec, HELLO_COMMIT_ELF, HELLO_COMMIT_ID, MULTI_TEST_ELF,
            MULTI_TEST_ID,
        };

        let prover = init_prover().await;

        let hello_receipt = prove_succinct(
            prover.as_ref(),
            "assumption/hello_commit",
            ExecutorEnv::builder().build().unwrap(),
            HELLO_COMMIT_ELF,
            HELLO_COMMIT_ID,
        );
        let hello_claim = hello_receipt.claim().unwrap().as_value().unwrap().clone();

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::SysVerify(vec![(
                HELLO_COMMIT_ID.into(),
                hello_receipt.journal.bytes.clone(),
            )]))
            .unwrap()
            .add_assumption(hello_receipt.clone())
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "multi_test/sys_verify",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::SysVerifyIntegrity {
                claim_words: to_vec(&hello_claim).unwrap(),
            })
            .unwrap()
            .add_assumption(hello_receipt.clone())
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "multi_test/sys_verify_integrity",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let assumption = Assumption {
            claim: hello_receipt.claim().unwrap().digest(),
            control_root: ALLOWED_CONTROL_ROOT,
        };
        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::SysVerifyAssumption {
                assumption_words: to_vec(&assumption).unwrap(),
            })
            .unwrap()
            .add_assumption(hello_receipt.clone())
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "multi_test/sys_verify_assumption",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let env = ExecutorEnv::builder()
            .segment_limit_po2(15)
            .write(&MultiTestSpec::BusyLoop { cycles: 1 << 16 })
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "multi_test/continuation",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
        assert_eq!(
            receipt.claim().unwrap().as_value().unwrap().exit_code,
            ExitCode::Halted(0)
        );

        let povw_job_id = PovwJobId {
            log: PovwLogId::from(0x202ce_u64),
            job: 42,
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(16)
            .povw(povw_job_id)
            .write(&MultiTestSpec::BusyLoop { cycles: 1 << 16 })
            .unwrap()
            .build()
            .unwrap();
        let prove_info = prove_succinct_info(
            prover.as_ref(),
            "multi_test/povw",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
            &ProverOpts::succinct(),
        );
        let work_receipt = prove_info
            .work_receipt
            .expect("multi_test/povw: missing work receipt");
        work_receipt
            .verify_integrity()
            .expect("multi_test/povw: work receipt integrity failed");
        let work_claim = work_receipt.claim().as_value().unwrap().clone();
        assert_eq!(
            work_claim.claim.digest(),
            prove_info.receipt.claim().unwrap().digest()
        );
        let work = work_claim.work.as_value().unwrap();
        assert!(work.value >= 1 << 16);
        assert_eq!(work.nonce_min.log, povw_job_id.log);
        assert_eq!(work.nonce_min.job, povw_job_id.job);
        assert_eq!(work.nonce_min.segment, 0);
        assert_eq!(work.nonce_max.log, povw_job_id.log);
        assert_eq!(work.nonce_max.job, povw_job_id.job);

        let env = ExecutorEnv::builder()
            .write(&MultiTestSpec::Halt(1))
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct_integrity(
            prover.as_ref(),
            "multi_test/halt_nonzero",
            env,
            MULTI_TEST_ELF,
            &ProverOpts::succinct().with_prove_guest_errors(true),
        );
        assert_eq!(
            receipt.claim().unwrap().as_value().unwrap().exit_code,
            ExitCode::Halted(1)
        );
    }

    #[wasm_bindgen_test(async)]
    async fn bigint2_precompile_guest_succinct_receipts_verify() {
        use num_bigint::BigUint;
        use risc0_bigint2_methods::{
            ECDSA_ELF, ECDSA_ID, EC_384_ELF, EC_384_ID, EC_ADD_256_ELF, EC_ADD_256_ID,
            EC_DOUBLE_256_ELF, EC_DOUBLE_256_ID, EC_MUL_256_ELF, EC_MUL_256_ID,
            EXTFIELD_DEG2_ADD_256_ELF, EXTFIELD_DEG2_ADD_256_ID, EXTFIELD_DEG2_ADD_384_ELF,
            EXTFIELD_DEG2_ADD_384_ID, EXTFIELD_DEG2_MUL_ELF, EXTFIELD_DEG2_MUL_ID,
            EXTFIELD_DEG2_SUB_256_ELF, EXTFIELD_DEG2_SUB_256_ID, EXTFIELD_DEG2_SUB_384_ELF,
            EXTFIELD_DEG2_SUB_384_ID, EXTFIELD_DEG4_MUL_ELF, EXTFIELD_DEG4_MUL_ID,
            EXTFIELD_XXONE_MUL_256_ELF, EXTFIELD_XXONE_MUL_256_ID, EXTFIELD_XXONE_MUL_384_ELF,
            EXTFIELD_XXONE_MUL_384_ID, MODADD_256_ELF, MODADD_256_ID, MODADD_384_ELF,
            MODADD_384_ID, MODINV_256_ELF, MODINV_256_ID, MODINV_384_ELF, MODINV_384_ID,
            MODMUL_256_ELF, MODMUL_256_ID, MODMUL_384_ELF, MODMUL_384_ID, MODSUB_256_ELF,
            MODSUB_256_ID, MODSUB_384_ELF, MODSUB_384_ID, RAW_TEST_ELF, RAW_TEST_ID, RSA_ELF,
            RSA_ID,
        };
        use risc0_zkvm::DeserializeOwned;

        fn bu(hex: &str) -> BigUint {
            BigUint::parse_bytes(hex.as_bytes(), 16).unwrap()
        }

        fn prove_decode<T: DeserializeOwned>(
            prover: &WebGpuProver,
            name: &str,
            env: ExecutorEnv,
            elf: &[u8],
            image_id: [u32; 8],
        ) -> T {
            prove_succinct(prover, name, env, elf, image_id)
                .journal
                .decode()
                .unwrap()
        }

        let prover = init_prover().await;

        for (name, elf, image_id, expected) in [
            (
                "bigint2/modadd_256",
                MODADD_256_ELF,
                MODADD_256_ID,
                bu("02"),
            ),
            (
                "bigint2/modadd_384",
                MODADD_384_ELF,
                MODADD_384_ID,
                bu("02"),
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("04"), bu("07"), bu("03")))
                .unwrap()
                .build()
                .unwrap();
            let result: BigUint = prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, expected);
        }

        for (name, elf, image_id, expected) in [
            (
                "bigint2/modsub_256",
                MODSUB_256_ELF,
                MODSUB_256_ID,
                bu("02"),
            ),
            (
                "bigint2/modsub_384",
                MODSUB_384_ELF,
                MODSUB_384_ID,
                bu("02"),
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("04"), bu("07"), bu("05")))
                .unwrap()
                .build()
                .unwrap();
            let result: BigUint = prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, expected);
        }

        for (name, elf, image_id, expected) in [
            (
                "bigint2/modmul_256",
                MODMUL_256_ELF,
                MODMUL_256_ID,
                bu("03"),
            ),
            (
                "bigint2/modmul_384",
                MODMUL_384_ELF,
                MODMUL_384_ID,
                bu("03"),
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("04"), bu("07"), bu("05")))
                .unwrap()
                .build()
                .unwrap();
            let result: BigUint = prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, expected);
        }

        for (name, elf, image_id, expected) in [
            (
                "bigint2/modinv_256",
                MODINV_256_ELF,
                MODINV_256_ID,
                bu("03"),
            ),
            (
                "bigint2/modinv_384",
                MODINV_384_ELF,
                MODINV_384_ID,
                bu("03"),
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("02"), bu("05")))
                .unwrap()
                .build()
                .unwrap();
            let result: BigUint = prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, expected);
        }

        for (name, elf, image_id, expected) in [
            (
                "bigint2/extfield_deg2_add_256",
                EXTFIELD_DEG2_ADD_256_ELF,
                EXTFIELD_DEG2_ADD_256_ID,
                (bu("00"), bu("03")),
            ),
            (
                "bigint2/extfield_deg2_add_384",
                EXTFIELD_DEG2_ADD_384_ELF,
                EXTFIELD_DEG2_ADD_384_ID,
                (bu("00"), bu("03")),
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("04"), bu("06"), bu("03"), bu("04"), bu("07")))
                .unwrap()
                .build()
                .unwrap();
            let result: (BigUint, BigUint) =
                prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, expected);
        }

        for (name, elf, image_id, expected) in [
            (
                "bigint2/extfield_deg2_sub_256",
                EXTFIELD_DEG2_SUB_256_ELF,
                EXTFIELD_DEG2_SUB_256_ID,
                (bu("06"), bu("04")),
            ),
            (
                "bigint2/extfield_deg2_sub_384",
                EXTFIELD_DEG2_SUB_384_ELF,
                EXTFIELD_DEG2_SUB_384_ID,
                (bu("06"), bu("04")),
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("02"), bu("06"), bu("03"), bu("02"), bu("07")))
                .unwrap()
                .build()
                .unwrap();
            let result: (BigUint, BigUint) =
                prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, expected);
        }

        let env = ExecutorEnv::builder()
            .write(&(
                bu("05"),
                bu("02"),
                bu("02"),
                bu("03"),
                bu("06"),
                bu("00"),
                bu("07"),
            ))
            .unwrap()
            .build()
            .unwrap();
        let result: (BigUint, BigUint) = prove_decode(
            prover.as_ref(),
            "bigint2/extfield_deg2_mul",
            env,
            EXTFIELD_DEG2_MUL_ELF,
            EXTFIELD_DEG2_MUL_ID,
        );
        assert_eq!(result, (bu("04"), bu("05")));

        for (name, elf, image_id) in [
            (
                "bigint2/extfield_xxone_mul_256",
                EXTFIELD_XXONE_MUL_256_ELF,
                EXTFIELD_XXONE_MUL_256_ID,
            ),
            (
                "bigint2/extfield_xxone_mul_384",
                EXTFIELD_XXONE_MUL_384_ELF,
                EXTFIELD_XXONE_MUL_384_ID,
            ),
        ] {
            let env = ExecutorEnv::builder()
                .write(&(bu("05"), bu("05"), bu("02"), bu("02"), bu("07"), bu("31")))
                .unwrap()
                .build()
                .unwrap();
            let result: (BigUint, BigUint) =
                prove_decode(prover.as_ref(), name, env, elf, image_id);
            assert_eq!(result, (bu("00"), bu("06")));
        }

        let env = ExecutorEnv::builder()
            .write(&(
                bu("04"),
                bu("05"),
                bu("02"),
                bu("04"),
                bu("03"),
                bu("06"),
                bu("06"),
                bu("02"),
                bu("06"),
                bu("00"),
                bu("00"),
                bu("00"),
                bu("07"),
            ))
            .unwrap()
            .build()
            .unwrap();
        let result: (BigUint, BigUint, BigUint, BigUint) = prove_decode(
            prover.as_ref(),
            "bigint2/extfield_deg4_mul",
            env,
            EXTFIELD_DEG4_MUL_ELF,
            EXTFIELD_DEG4_MUL_ID,
        );
        assert_eq!(result, (bu("01"), bu("04"), bu("03"), bu("06")));

        const BIGINT_LEGAL_ADDR: u32 = 0x3000_0000;
        let env = ExecutorEnv::builder()
            .write(&(BIGINT_LEGAL_ADDR, BIGINT_LEGAL_ADDR, BIGINT_LEGAL_ADDR))
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "bigint2/raw_test",
            env,
            RAW_TEST_ELF,
            RAW_TEST_ID,
        );

        let lhs: Option<[[u32; 8]; 2]> = Some([
            [
                0x16f81798, 0x59f2815b, 0x2dce28d9, 0x029bfcdb, 0xce870b07, 0x55a06295, 0xf9dcbbac,
                0x79be667e,
            ],
            [
                0xfb10d4b8, 0x9c47d08f, 0xa6855419, 0xfd17b448, 0x0e1108a8, 0x5da4fbfc, 0x26a3c465,
                0x483ada77,
            ],
        ]);
        let rhs: Option<[[u32; 8]; 2]> = Some([
            [
                0xac04dc3f, 0x9465e6a4, 0xf46d2dad, 0x5d5ac4b6, 0xad2c0db6, 0xa7c06f71, 0xe335abc9,
                0x0f66dc33,
            ],
            [
                0xd3f64d1c, 0x50650be0, 0x2a8577b0, 0xb701323c, 0x95565b00, 0x6dddd83d, 0x398fcd2c,
                0x83641fc5,
            ],
        ]);
        let expected = Some([
            [
                0x3db079e0, 0xd4ad0ff5, 0xdd0da7e2, 0x4faad0a4, 0x85894785, 0x280d6b36, 0xe8ab292d,
                0xa901b0db,
            ],
            [
                0x47298a9d, 0x01d0e60e, 0xa6b063b3, 0x716bc5e0, 0x61e7ae64, 0xaf6f04dc, 0x834f1a61,
                0x3f27e7e1,
            ],
        ]);
        let env = ExecutorEnv::builder()
            .write(&(lhs, rhs))
            .unwrap()
            .build()
            .unwrap();
        let result: Option<[[u32; 8]; 2]> = prove_decode(
            prover.as_ref(),
            "bigint2/ec_add_256",
            env,
            EC_ADD_256_ELF,
            EC_ADD_256_ID,
        );
        assert_eq!(result, expected);

        let point: Option<[[u32; 8]; 2]> = Some([
            [
                0x16F81798, 0x59F2815B, 0x2DCE28D9, 0x029BFCDB, 0xCE870B07, 0x55A06295, 0xF9DCBBAC,
                0x79BE667E,
            ],
            [
                0xFB10D4B8, 0x9C47D08F, 0xA6855419, 0xFD17B448, 0x0E1108A8, 0x5DA4FBFC, 0x26A3C465,
                0x483ADA77,
            ],
        ]);
        let env = ExecutorEnv::builder()
            .write(&point)
            .unwrap()
            .build()
            .unwrap();
        let result: Option<[[u32; 8]; 2]> = prove_decode(
            prover.as_ref(),
            "bigint2/ec_double_256",
            env,
            EC_DOUBLE_256_ELF,
            EC_DOUBLE_256_ID,
        );
        assert!(result.is_some());

        prove_succinct(
            prover.as_ref(),
            "bigint2/ec_mul_256",
            ExecutorEnv::builder().build().unwrap(),
            EC_MUL_256_ELF,
            EC_MUL_256_ID,
        );

        let point384: Option<[[u32; 12]; 2]> = Some([
            [
                0x72760ab7, 0x3a545e38, 0xbf55296c, 0x5502f25d, 0x82542a38, 0x59f741e0, 0x8ba79b98,
                0x6e1d3b62, 0xf320ad74, 0x8eb1c71e, 0xbe8b0537, 0xaa87ca22,
            ],
            [
                0x90ea0e5f, 0x7a431d7c, 0x1d7e819d, 0x0a60b1ce, 0xb5f0b8c0, 0xe9da3113, 0x289a147c,
                0xf8f41dbd, 0x9292dc29, 0x5d9e98bf, 0x96262c6f, 0x3617de4a,
            ],
        ]);
        let env = ExecutorEnv::builder()
            .write(&point384)
            .unwrap()
            .build()
            .unwrap();
        let result: Option<[[u32; 12]; 2]> = prove_decode(
            prover.as_ref(),
            "bigint2/ec_384",
            env,
            EC_384_ELF,
            EC_384_ID,
        );
        assert!(result.is_some());

        prove_succinct(
            prover.as_ref(),
            "bigint2/ecdsa",
            ExecutorEnv::builder().build().unwrap(),
            ECDSA_ELF,
            ECDSA_ID,
        );

        let env = ExecutorEnv::builder()
            .write(&(bu("01"), bu("05")))
            .unwrap()
            .build()
            .unwrap();
        let result: BigUint = prove_decode(prover.as_ref(), "bigint2/rsa", env, RSA_ELF, RSA_ID);
        assert_eq!(result, bu("01"));
    }

    #[wasm_bindgen_test(async)]
    async fn hello_world_succinct_receipt_verifies() {
        use hello_world_methods::{MULTIPLY_ELF, MULTIPLY_ID};

        let prover = init_prover().await;
        let env = ExecutorEnv::builder()
            .write(&17u64)
            .unwrap()
            .write(&23u64)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "hello-world",
            env,
            MULTIPLY_ELF,
            MULTIPLY_ID,
        );
        assert_eq!(receipt.journal.decode::<u64>().unwrap(), 391);
    }

    #[wasm_bindgen_test(async)]
    async fn json_succinct_receipt_verifies() {
        use json_core::Outputs;
        use json_methods::{SEARCH_JSON_ELF, SEARCH_JSON_ID};

        let prover = init_prover().await;
        let data = include_str!("../../json/res/example.json");
        let env = ExecutorEnv::builder()
            .write(&data)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "json",
            env,
            SEARCH_JSON_ELF,
            SEARCH_JSON_ID,
        );
        let outputs: Outputs = receipt.journal.decode().unwrap();
        assert_eq!(outputs.data, 47);
    }

    #[wasm_bindgen_test(async)]
    async fn chess_succinct_receipt_verifies() {
        use chess_core::Inputs;
        use chess_methods::{CHECKMATE_ELF, CHECKMATE_ID};

        const BOARD: &str = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4";
        const MOVE: &str = "Qxf7";

        let prover = init_prover().await;
        let inputs = Inputs {
            board: BOARD.to_string(),
            mv: MOVE.to_string(),
        };
        let env = ExecutorEnv::builder()
            .write(&inputs)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(prover.as_ref(), "chess", env, CHECKMATE_ELF, CHECKMATE_ID);
        assert_eq!(receipt.journal.decode::<String>().unwrap(), BOARD);
    }

    #[wasm_bindgen_test(async)]
    async fn composition_succinct_receipt_verifies() {
        use composition_example_methods::{EXPONENTIATE_ELF, EXPONENTIATE_ID};
        use hello_world_methods::{MULTIPLY_ELF, MULTIPLY_ID};

        let prover = init_prover().await;

        let multiply_env = ExecutorEnv::builder()
            .write(&17u64)
            .unwrap()
            .write(&23u64)
            .unwrap()
            .build()
            .unwrap();
        let multiply_receipt = prove_succinct(
            prover.as_ref(),
            "composition/multiply-assumption",
            multiply_env,
            MULTIPLY_ELF,
            MULTIPLY_ID,
        );
        let n: u64 = multiply_receipt.journal.decode().unwrap();

        let env = ExecutorEnv::builder()
            .add_assumption(multiply_receipt)
            .write(&(n, 9u64, 100u64))
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "composition",
            env,
            EXPONENTIATE_ELF,
            EXPONENTIATE_ID,
        );
        let (n_out, e, c): (u64, u64, u64) = receipt.journal.decode().unwrap();
        assert_eq!((n_out, e, c), (391, 9, 32));
    }

    #[wasm_bindgen_test(async)]
    async fn jwt_validator_succinct_receipt_verifies() {
        use jwt_core::{CustomClaims, Issuer};
        use jwt_methods::{VALIDATOR_ELF, VALIDATOR_ID};

        const SECRET_KEY: &str = r#"
    {
      "alg": "RS256",
      "d": "YuO1XZkYSwDRgauXQe6q1u8fET3S7x7g4N8uE49rdt7g3-O9q-Hwn_nQNiRr9o7Uslf7X8sL6txraQy7TdPUuSkaULpRNo2FoVLLoO2eACWwPtCG4n9wuvjnz7qCh9s3tfgOKxMA_riKkS8O7BxPH54rd7Ry1i6HN3TSYKYwxZxG4HFLhcewX6Q1KdGXdP7xVAsZ5lEpCQbhY5IKUzBZ5WIZpSTk10AadkVuwS622QT-9efk6PBWDyM48_udMdDo1HEcHsAdxrUMRdw_5uzVajQzZhNAmALXHCPT79P0qahzdYlUSHauT1XxU7z-KoCYVqt3z6epgYDcKmLzGkqIkSXUHxcVN-MTSGNET_dhio0tHG-jV3wB5jfsgayoIZCeTPF-F-nDwn8Cyz18uee_Y7U53NTtEXGqB9npZyu7SibTztwSeLs6zH965d1VTmUCxH8CWqizugfQY8ibNgVCd42naAuWbOmxYEjyelmHf_BS0Vb7NwpW9cuaODOjpjCz",
      "dp": "DOIAbWzet_-ZSED61WWvG9Byao9uQh3SSvtvAUa4WhWEq3lfGqt1wEDneOds1IrxNF7Y2rV_iHBVA2DWB9ctdMxau3DteGumbMzEObQIjDs7SP45plImxHzZbXgTIB-DWiujJmwDNJUIaB80q1sjeeBTJ9rfaU0ZNMFO26koOKQGoNDuuJTgejnRwGdIGhoOLcT_dus-7CNWY1pRBvTGhcEOygRE_icb8JzNoKo90fwZf0ACdxiFc6G_RUCXapap",
      "dq": "0yFAtOVm0-fPLg62RcALyhIXsyEOd25W0YFmWIzb6Bh5kbMruA-befX-ANnNGcktBGgY7QGN6myb-K8zRCOYfVt5zs0EFEFCHc6NO8UoSJCItOZFMdaLsG21MqOdtQRQi4F_TJ2yoqu1S81O-Y08wtFE0F8hVe7sGuJIoRtY5yF_Swwaw3ST-XMfghpbhvc71zVF7VyPlyrqU-NeKimBpuEfHuTKQSSudY9eLNdypyE71RC6q_xWxWzTSqu3pih5",
      "e": "AQAB",
      "key_ops": ["sign"],
      "kty": "RSA",
      "n": "zcQwXx3EevOSkfH0VSWqtfmWTL4c2oIzW6u83qKO1W7XjLgTqpryL5vNCaxbVTkpU-GZctit0n6kj570tfny_sy6pb2q9wlvFBmDVyD-nL5oNjP5s3qEfvy15Bl9vMGFf3zycqMaVg_7VRVwK5d8QzpnVC0AGT10QdHnyGCadfPJqazTuVRp1f3ecK7bg7596sgVb8d9Wpaz2XPykQPfphsEb40vcp1tPN95-eRCgA24PwfUaKYHQQFMEQY_atJWbffyJ91zsBRy8fEQdfuQVZIRVQgO7FTsmLmQAHxR1dl2jP8B6zonWmtqWoMHoZfa-kmTPB4wNHa8EaLvtQ1060qYFmQWWumfNFnG7HNq2gTHt1cN1HCwstRGIaU_ZHubM_FKH_gLfJPKNW0KWML9mQQzf4AVov0Yfvk89WxY8ilSRx6KodJuIKKqwVh_58PJPLmBqszEfkTjtyxPwP8X8xRXfSz-vTU6vESCk3O6TRknoJkC2BJZ_ONQ0U5dxLcx",
      "p": "-TQVt9yl_0S0uvUM37L3WSDPkOn_gy34zpAEllhgx1HQUg_pVbqEDwKzEIpBlZfbrcszMlmiJhKL6q4y0_a6e3O5QnfB1vrGTjhLcfcaUK6o-I7bxabrpZmvLIsTqSdAgUijXe8yhQFIoCjc1MPD7icRPc-V7P9IYE2ls9X6sgo4lUZjQAuQtOo8ndlZ3uqP2sMKRR3CS7tHiF1r_zq_NXcf98Sve-1rRnqT6GpGcJRcvVFu2wy8TyCPMAvWh903",
      "q": "02DUlUJrcTQ-mHMmg-V5qjxrtTKMmjqXpN0pgkXhM8_DWCrqKL9sXb1MKXQcbAZYr-lWmtBwzXeF4Qn66dRHpjlQLhSA947UxjuEtbhWx3wKGG460ZH026qcRr3QspcKZuiX2zISHb8suMl2lhDDSggCAjybs0l72pNHPIny9pucnwqc9ihrbeu68LlUpnQtS-Okt4j5ndVc1l1Vwv2PFt2PxrLmQkqdwRMla1F7r0vtgM7NIZz9XPszSrkxTILX",
      "qi": "3yweZ6b2adwqUrCvyvK5ub5XAjKOh1N7AoFqYQFpD_ho41ThyWErfjTztDlgqqTHo3wHyR49cq-L6aAuerNTPW7VAXTobC8vZSxIKazOU9p0xcDYSaGGH_IES62MAxJu1rdyAOrq_MLsqvBckVancmW6lVWQr27wDNTNwskkPpgDXwAygWSCBbM-oZOsWamge0SadQJOCd7Rr33aLfWFKaajl7FnQzX6Wh8Q0gLn2PRDnC7V1gEVWY3fWSzs4obj",
      "use": "sig",
      "kid": "6ab0e8e4bc121fc287e35d3e5e0efb8a"
    }
"#;

        let prover = init_prover().await;
        let claims = CustomClaims {
            subject: "Hello, world!".to_string(),
        };
        let issuer = SECRET_KEY.parse::<Issuer>().unwrap();
        let token = issuer.generate_token(&claims).unwrap();
        let env = ExecutorEnv::builder()
            .write(&token)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "jwt-validator",
            env,
            VALIDATOR_ELF,
            VALIDATOR_ID,
        );
        assert_eq!(receipt.journal.decode::<String>().unwrap(), claims.subject);
    }

    #[wasm_bindgen_test(async)]
    async fn bevy_succinct_receipt_verifies() {
        use bevy_core::Outputs;
        use bevy_methods::{BEVY_GUEST_ELF, BEVY_GUEST_ID};

        let prover = init_prover().await;
        let turns = 3u32;
        let env = ExecutorEnv::builder()
            .write(&turns)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(prover.as_ref(), "bevy", env, BEVY_GUEST_ELF, BEVY_GUEST_ID);
        let outputs: Outputs = receipt.journal.decode().unwrap();
        assert_eq!(outputs.position, turns as f32);
    }

    #[wasm_bindgen_test(async)]
    async fn digital_signature_succinct_receipt_verifies() {
        use digital_signature_core::SigningRequest;
        use digital_signature_methods::{SIGN_ELF, SIGN_ID};
        use risc0_zkvm::sha::{Impl, Sha256};

        let prover = init_prover().await;
        let request = SigningRequest {
            passphrase: *Impl::hash_bytes(b"passphr4ase"),
            msg: *Impl::hash_bytes(b"This message was signed by me"),
        };
        let env = ExecutorEnv::builder()
            .write(&request)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(prover.as_ref(), "digital-signature", env, SIGN_ELF, SIGN_ID);
    }

    #[wasm_bindgen_test(async)]
    async fn groth16_verifier_succinct_receipt_verifies() {
        use risc0_groth16::{ProofJson, PublicInputsJson, VerifyingKeyJson};
        use risc0_zkvm::sha::Digest;

        use groth16_verifier_methods::{GROTH16_VERIFIER_ELF, GROTH16_VERIFIER_ID};

        let prover = init_prover().await;
        let proof_json: ProofJson =
            serde_json::from_str(include_str!("../../groth16-verifier/src/data/proof.json"))
                .unwrap();
        let public_inputs_json = PublicInputsJson {
            values: serde_json::from_str(include_str!(
                "../../groth16-verifier/src/data/public.json"
            ))
            .unwrap(),
        };
        let verifying_key_json: VerifyingKeyJson = serde_json::from_str(include_str!(
            "../../groth16-verifier/src/data/verification_key.json"
        ))
        .unwrap();
        let env = ExecutorEnv::builder()
            .write(&(&proof_json, &public_inputs_json, &verifying_key_json))
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "groth16-verifier",
            env,
            GROTH16_VERIFIER_ELF,
            GROTH16_VERIFIER_ID,
        );
        let (_vk_digest, _public_inputs_digest): (Digest, Digest) =
            receipt.journal.decode().unwrap();
    }

    #[wasm_bindgen_test(async)]
    async fn prorata_succinct_receipt_verifies() {
        use prorata_core::{AllocationQuery, AllocationQueryResult};
        use prorata_methods::{PRORATA_GUEST_ELF, PRORATA_GUEST_ID};
        use rust_decimal::Decimal;

        let prover = init_prover().await;
        let query = AllocationQuery {
            amount: Decimal::new(10000, 2),
            recipients_csv: b"name,share\nAlice,0.5\nBob,0.25\nCarol,0.25\n".to_vec(),
            target: "Alice".to_string(),
        };
        let env = ExecutorEnv::builder()
            .write(&query)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "prorata",
            env,
            PRORATA_GUEST_ELF,
            PRORATA_GUEST_ID,
        );
        let result: AllocationQueryResult = receipt.journal.decode().unwrap();
        assert_eq!(result.allocation.unwrap().name, "Alice");
    }

    #[wasm_bindgen_test(async)]
    async fn wasm_interpreter_succinct_receipt_verifies() {
        use wasm_methods::{WASM_INTERP_ELF, WASM_INTERP_ID};

        let prover = init_prover().await;
        let wasm = wat::parse_str(
            r#"
            (module
                (export "fib" (func $fib))
                (func $fib (param $n i32) (result i32)
                    (local $a i32)
                    (local $b i32)
                    (local $tmp i32)
                    (local.set $a (i32.const 0))
                    (local.set $b (i32.const 1))
                    (block $exit
                        (loop $loop
                            (br_if $exit (i32.eqz (local.get $n)))
                            (local.set $tmp (local.get $a))
                            (local.set $a (local.get $b))
                            (local.set $b (i32.add (local.get $tmp) (local.get $b)))
                            (local.set $n (i32.sub (local.get $n) (i32.const 1)))
                            (br $loop)
                        )
                    )
                    (local.get $a)
                )
            )
        "#,
        )
        .unwrap();
        let iters = 10i32;
        let env = ExecutorEnv::builder()
            .write(&wasm)
            .unwrap()
            .write(&iters)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "wasm",
            env,
            WASM_INTERP_ELF,
            WASM_INTERP_ID,
        );
        assert_eq!(receipt.journal.decode::<i32>().unwrap(), 55);
    }

    #[wasm_bindgen_test(async)]
    async fn xgboost_succinct_receipt_verifies() {
        use forust_ml::GradientBooster;
        use xgboost_methods::{XGBOOST_ELF, XGBOOST_ID};

        let prover = init_prover().await;
        let model: GradientBooster =
            serde_json::from_str(include_str!("../../xgboost/res/trained_model.json")).unwrap();
        let model_bytes = rmp_serde::to_vec(&model).unwrap();
        let data: Vec<f64> = vec![18511304.0, 117.0];
        let env = ExecutorEnv::builder()
            .write(&data)
            .unwrap()
            .write(&model_bytes)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(prover.as_ref(), "xgboost", env, XGBOOST_ELF, XGBOOST_ID);
        assert_eq!(receipt.journal.decode::<f64>().unwrap(), 30.528042544062632);
    }

    #[wasm_bindgen_test(async)]
    async fn bn254_succinct_receipt_verifies() {
        use bn254_core::Inputs;
        use bn254_methods::{BN254_VERIFY_ELF, BN254_VERIFY_ID};

        let prover = init_prover().await;
        let input = Inputs {
            g1_compressed: hex::decode(
                "020000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            g2_compressed: hex::decode(
                "0A04D4BF3239F77CEE7B47C7245E9281B3E9C1182D6381A87BBF81F9F2A6254B731DF569CDA95E060BEE91BA69B3F2D103658A7AEA6B10E5BDC761E5715E7EE4BB",
            )
            .unwrap(),
            a: hex::decode("9c0d02eaaf8e7e7ad09595ef6e3b896f8915124ba5bef9287f0997557580caeb")
                .unwrap(),
            b: hex::decode("db6764642f7bb1f415d93fcd5aace586161ec2e4305f0d6fb57dbabf1d141a5b")
                .unwrap(),
        };
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "bn254",
            env,
            BN254_VERIFY_ELF,
            BN254_VERIFY_ID,
        );
        assert!(receipt.journal.decode::<bool>().unwrap());
    }

    #[wasm_bindgen_test(async)]
    async fn password_checker_succinct_receipt_verifies() {
        use password_checker_core::PasswordRequest;
        use password_checker_methods::{PW_CHECKER_ELF, PW_CHECKER_ID};

        let prover = init_prover().await;
        let request = PasswordRequest {
            password: "S00perSecr1t!!!".to_string(),
            salt: [0u8; 32],
        };
        let env = ExecutorEnv::builder()
            .write(&request)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "password-checker",
            env,
            PW_CHECKER_ELF,
            PW_CHECKER_ID,
        );
    }

    #[wasm_bindgen_test(async)]
    async fn voting_machine_succinct_receipts_verify() {
        use voting_machine_core::{
            Ballot, FreezeVotingMachineParams, FreezeVotingMachineResult, SubmitBallotParams,
            VotingMachineState,
        };
        use voting_machine_methods::{
            FREEZE_ELF, FREEZE_ID, INIT_ELF, INIT_ID, SUBMIT_ELF, SUBMIT_ID,
        };

        let prover = init_prover().await;
        let mut state = VotingMachineState {
            polls_open: true,
            voter_bitfield: 0,
            count: 0,
        };

        let init_env = ExecutorEnv::builder()
            .write(&state)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "voting-machine/init",
            init_env,
            INIT_ELF,
            INIT_ID,
        );

        let ballot = Ballot {
            voter: 1,
            vote_yes: true,
        };
        let params = SubmitBallotParams::new(state.clone(), ballot);
        let mut submit_output = Vec::new();
        let submit_env = ExecutorEnv::builder()
            .write(&params)
            .unwrap()
            .stdout(&mut submit_output)
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "voting-machine/submit",
            submit_env,
            SUBMIT_ELF,
            SUBMIT_ID,
        );
        state = from_slice(&submit_output).unwrap();

        let params = FreezeVotingMachineParams::new(state.clone());
        let mut freeze_output = Vec::new();
        let freeze_env = ExecutorEnv::builder()
            .write(&params)
            .unwrap()
            .stdout(&mut freeze_output)
            .build()
            .unwrap();
        prove_succinct(
            prover.as_ref(),
            "voting-machine/freeze",
            freeze_env,
            FREEZE_ELF,
            FREEZE_ID,
        );
        let result: FreezeVotingMachineResult = from_slice(&freeze_output).unwrap();
        assert!(!result.state.polls_open);
    }

    #[wasm_bindgen_test(async)]
    async fn keccak_succinct_receipt_verifies() {
        use keccak_methods::{KECCAK_ELF, KECCAK_ID};

        let prover = init_prover().await;
        let input = "abc";
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        prove_succinct(prover.as_ref(), "keccak", env, KECCAK_ELF, KECCAK_ID);
    }

    #[wasm_bindgen_test(async)]
    async fn smartcore_ml_succinct_receipt_verifies() {
        use smartcore::{
            linalg::basic::matrix::DenseMatrix,
            tree::decision_tree_classifier::DecisionTreeClassifier,
        };
        use smartcore_ml_methods::{ML_TEMPLATE_ELF, ML_TEMPLATE_ID};

        type Model = DecisionTreeClassifier<f64, u32, DenseMatrix<f64>, Vec<u32>>;

        let prover = init_prover().await;
        let is_svm = false;
        let model: Model = rmp_serde::from_slice(include_bytes!(
            "../../smartcore-ml/res/ml-model/tree_model_bytes.bin"
        ))
        .unwrap();
        let data: DenseMatrix<f64> = rmp_serde::from_slice(include_bytes!(
            "../../smartcore-ml/res/input-data/tree_model_data_bytes.bin"
        ))
        .unwrap();
        let env = ExecutorEnv::builder()
            .write(&is_svm)
            .unwrap()
            .write(&model)
            .unwrap()
            .write(&data)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "smartcore-ml",
            env,
            ML_TEMPLATE_ELF,
            ML_TEMPLATE_ID,
        );
        let result: Vec<u32> = receipt.journal.decode().unwrap();
        assert_eq!(result.len(), 150);
    }

    #[wasm_bindgen_test(async)]
    async fn wordle_succinct_receipt_verifies() {
        use wordle_core::{GameState, WordFeedback};
        use wordle_methods::{WORDLE_GUEST_ELF, WORDLE_GUEST_ID};

        let prover = init_prover().await;
        let secret = "world";
        let guess = "worry";
        let env = ExecutorEnv::builder()
            .write(&secret)
            .unwrap()
            .write(&guess)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "wordle",
            env,
            WORDLE_GUEST_ELF,
            WORDLE_GUEST_ID,
        );
        let state: GameState = receipt.journal.decode().unwrap();
        let feedback: WordFeedback = state.feedback;
        assert_eq!(feedback.0.len(), 5);
    }

    #[wasm_bindgen_test(async)]
    async fn sha_succinct_receipts_verify() {
        use sha_methods::{HASH_ELF, HASH_ID, HASH_RUST_CRYPTO_ELF, HASH_RUST_CRYPTO_ID};

        let prover = init_prover().await;

        for (name, elf, image_id) in [
            ("sha/hash", HASH_ELF, HASH_ID),
            (
                "sha/hash-rust-crypto",
                HASH_RUST_CRYPTO_ELF,
                HASH_RUST_CRYPTO_ID,
            ),
        ] {
            let input = "abc";
            let env = ExecutorEnv::builder()
                .write(&input)
                .unwrap()
                .build()
                .unwrap();
            prove_succinct(prover.as_ref(), name, env, elf, image_id);
        }
    }

    #[wasm_bindgen_test(async)]
    async fn c_guest_succinct_receipt_verifies() {
        use risc0_binfmt::ProgramBinary;
        use risc0_zkos_v1compat::V1COMPAT_ELF;
        use risc0_zkvm::compute_image_id;

        let prover = init_prover().await;
        let user_elf = include_bytes!(env!("C_GUEST_USER_ELF"));
        let elf = ProgramBinary::new(user_elf, V1COMPAT_ELF).encode();
        let digest = compute_image_id(&elf).unwrap();
        let mut image_id = [0u32; 8];
        image_id.copy_from_slice(digest.as_words());

        let env = ExecutorEnv::builder()
            .write_slice(&7u32.to_le_bytes())
            .write_slice(&11u32.to_le_bytes())
            .build()
            .unwrap();
        let receipt = prove_succinct(prover.as_ref(), "c-guest", env, &elf, image_id);
        assert_eq!(receipt.journal.decode::<u32>().unwrap(), 77);
    }

    #[wasm_bindgen_test(async)]
    async fn waldo_succinct_receipt_verifies() {
        use image::{DynamicImage, RgbImage};
        use waldo_core::{
            image::{ImageMerkleTree, IMAGE_CHUNK_SIZE},
            merkle::SYS_VECTOR_ORACLE,
            Journal, PrivateInput,
        };
        use waldo_methods::{IMAGE_CROP_ELF, IMAGE_CROP_ID};

        let prover = init_prover().await;
        let mut raw = Vec::new();
        for i in 0..16 * 16 {
            raw.extend_from_slice(&[i as u8, (i * 3) as u8, (255 - i) as u8]);
        }
        let image = DynamicImage::ImageRgb8(RgbImage::from_raw(16, 16, raw).unwrap());
        let tree = ImageMerkleTree::<{ IMAGE_CHUNK_SIZE }>::new(&image);
        let input = PrivateInput {
            root: tree.root(),
            image_dimensions: (16, 16),
            crop_location: (7, 7),
            crop_dimensions: (3, 3),
            mask: None,
        };
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .io_callback(SYS_VECTOR_ORACLE, tree.vector_oracle_callback())
            .build()
            .unwrap();
        let receipt = prove_succinct(prover.as_ref(), "waldo", env, IMAGE_CROP_ELF, IMAGE_CROP_ID);
        let journal: Journal = receipt.journal.decode().unwrap();
        assert_eq!(journal.subimage_dimensions, (3, 3));
    }

    #[wasm_bindgen_test(async)]
    async fn ecdsa_k256_succinct_receipt_verifies() {
        use k256::{
            ecdsa::{signature::Signer, Signature, SigningKey},
            EncodedPoint,
        };
        use k256_methods::{K256_VERIFY_ELF, K256_VERIFY_ID};

        let prover = init_prover().await;
        let signing_key = SigningKey::from_bytes((&[7u8; 32]).into()).unwrap();
        let message = b"This is a message that will be signed, and verified within the zkVM";
        let signature: Signature = signing_key.sign(message);
        let input = (
            signing_key.verifying_key().to_encoded_point(true),
            message.to_vec(),
            signature,
        );
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "ecdsa/k256",
            env,
            K256_VERIFY_ELF,
            K256_VERIFY_ID,
        );
        let (_key, msg): (EncodedPoint, Vec<u8>) = receipt.journal.decode().unwrap();
        assert_eq!(msg, message);
    }

    #[wasm_bindgen_test(async)]
    async fn ecdsa_p256_succinct_receipt_verifies() {
        use p256::{
            ecdsa::{signature::Signer, Signature, SigningKey},
            EncodedPoint,
        };
        use p256_methods::{P256_VERIFY_ELF, P256_VERIFY_ID};

        let prover = init_prover().await;
        let signing_key = SigningKey::from_bytes((&[9u8; 32]).into()).unwrap();
        let message = b"This is a message that will be signed, and verified within the zkVM";
        let signature: Signature = signing_key.sign(message);
        let input = (
            signing_key.verifying_key().to_encoded_point(true),
            message.to_vec(),
            signature,
        );
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        let receipt = prove_succinct(
            prover.as_ref(),
            "ecdsa/p256",
            env,
            P256_VERIFY_ELF,
            P256_VERIFY_ID,
        );
        let (_key, msg): (EncodedPoint, Vec<u8>) = receipt.journal.decode().unwrap();
        assert_eq!(msg, message);
    }
}

#[cfg(all(test, not(all(target_arch = "wasm32", target_os = "unknown"))))]
mod native_stats_tests {
    use std::{collections::BTreeMap, time::Instant};

    use risc0_zkvm::{
        default_executor, default_prover,
        sha::{Digest, Digestible},
        ExecutorEnv, ExitCode, ProveInfo, ProverOpts,
    };

    const WEBGPU_BASELINE_SEGMENT_LIMIT_PO2: u32 = 18;
    const WEBGPU_BASELINE_KECCAK_MAX_PO2: u32 = 14;

    fn prove_and_print_stats(
        name: &str,
        env: ExecutorEnv,
        elf: &[u8],
        image_id: [u32; 8],
    ) -> ProveInfo {
        prove_with_opts_and_print_stats(name, env, elf, image_id, &ProverOpts::succinct())
    }

    fn prove_with_opts_and_print_stats(
        name: &str,
        env: ExecutorEnv,
        elf: &[u8],
        image_id: [u32; 8],
        opts: &ProverOpts,
    ) -> ProveInfo {
        let prover = default_prover();
        let started = Instant::now();
        let prove_info = prover.prove_with_opts(env, elf, opts).unwrap();
        let elapsed = started.elapsed();
        prove_info.receipt.verify(image_id).unwrap();
        println!(
            "native_prove name={name} elapsed={elapsed:?} segments={} user_cycles={} total_cycles={}",
            prove_info.stats.segments, prove_info.stats.user_cycles, prove_info.stats.total_cycles
        );
        prove_info
    }

    fn prove_with_opts_and_print_integrity_stats(
        name: &str,
        env: ExecutorEnv,
        elf: &[u8],
        opts: &ProverOpts,
    ) -> ProveInfo {
        let prover = default_prover();
        let started = Instant::now();
        let prove_info = prover.prove_with_opts(env, elf, opts).unwrap();
        let elapsed = started.elapsed();
        prove_info
            .receipt
            .verify_integrity_with_context(&Default::default())
            .unwrap();
        println!(
            "native_prove name={name} elapsed={elapsed:?} segments={} user_cycles={} total_cycles={}",
            prove_info.stats.segments, prove_info.stats.user_cycles, prove_info.stats.total_cycles
        );
        prove_info
    }

    #[test]
    #[ignore = "manual helper for measuring the browser verifier-guest segment shape"]
    fn native_guest_verify_execute_stats() {
        use risc0_zkvm_methods::{HELLO_COMMIT_ELF, HELLO_COMMIT_ID, VERIFY_ELF, VERIFY_ID};

        let prover = default_prover();
        let hello_started = Instant::now();
        let hello_info = prover
            .prove_with_opts(
                ExecutorEnv::builder()
                    .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
                    .build()
                    .unwrap(),
                HELLO_COMMIT_ELF,
                &ProverOpts::succinct(),
            )
            .unwrap();
        let hello_elapsed = hello_started.elapsed();
        let hello_receipt = hello_info.receipt;
        println!(
            "native_hello_prove elapsed={hello_elapsed:?} segments={} user_cycles={} total_cycles={}",
            hello_info.stats.segments,
            hello_info.stats.user_cycles,
            hello_info.stats.total_cycles
        );

        let verify_input = || {
            (
                hello_receipt.clone(),
                Digest::from(HELLO_COMMIT_ID),
                false, /* dev_mode */
            )
        };
        let execute_started = Instant::now();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&verify_input())
            .unwrap()
            .build()
            .unwrap();
        let session = default_executor().execute(env, VERIFY_ELF).unwrap();
        let execute_elapsed = execute_started.elapsed();
        let segment_po2_counts =
            session
                .segments
                .iter()
                .fold(BTreeMap::new(), |mut counts, segment| {
                    *counts.entry(segment.po2).or_insert(0usize) += 1;
                    counts
                });
        let first_segment_cycles = session.segments.first().map(|segment| segment.cycles);
        let last_segment_cycles = session.segments.last().map(|segment| segment.cycles);
        println!(
            "native_verify_execute elapsed={execute_elapsed:?} segments={} total_user_cycles={} segment_po2_counts={segment_po2_counts:?} first_segment_cycles={first_segment_cycles:?} last_segment_cycles={last_segment_cycles:?} claim={}",
            session.segments.len(),
            session.cycles(),
            session.receipt_claim.unwrap().digest(),
        );
        if std::env::var_os("RISC0_PRINT_SEGMENTS").is_some() {
            println!(
                "native_verify_execute_segments segment_pos={:?} segment_user_cycles={:?}",
                session
                    .segments
                    .iter()
                    .map(|segment| segment.po2)
                    .collect::<Vec<_>>(),
                session
                    .segments
                    .iter()
                    .map(|segment| segment.cycles)
                    .collect::<Vec<_>>()
            );
        }

        let prove_started = Instant::now();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&verify_input())
            .unwrap()
            .build()
            .unwrap();
        let prove_info = prover
            .prove_with_opts(env, VERIFY_ELF, &ProverOpts::succinct())
            .unwrap();
        let prove_elapsed = prove_started.elapsed();
        prove_info.receipt.verify(VERIFY_ID).unwrap();
        println!(
            "native_verify_prove elapsed={prove_elapsed:?} segments={} user_cycles={} total_cycles={}",
            prove_info.stats.segments,
            prove_info.stats.user_cycles,
            prove_info.stats.total_cycles
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser cfg guest native baseline"]
    fn native_cfg_prove_stats() {
        use risc0_zkvm_methods::{CFG_ELF, CFG_ID};

        prove_and_print_stats(
            "risc0-zkvm-methods/cfg",
            ExecutorEnv::builder()
                .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
                .build()
                .unwrap(),
            CFG_ELF,
            CFG_ID,
        );
    }

    #[test]
    #[ignore = "manual helper for measuring browser prover API and execution mode native baselines"]
    fn native_prover_api_and_execution_modes_stats() {
        use risc0_zkvm_methods::{multi_test::MultiTestSpec, MULTI_TEST_ELF, MULTI_TEST_ID};

        const MEM_POS: u32 = 0x0020_0600;

        let prover = default_prover();
        let composite_env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::DoNothing)
            .unwrap()
            .build()
            .unwrap();
        let composite_info = prove_with_opts_and_print_stats(
            "multi_test/do_nothing/composite",
            composite_env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
            &ProverOpts::composite(),
        );
        composite_info.receipt.inner.composite().unwrap();

        let started = Instant::now();
        let compressed = prover
            .compress(&ProverOpts::succinct(), &composite_info.receipt)
            .unwrap();
        let elapsed = started.elapsed();
        compressed.inner.succinct().unwrap();
        compressed.verify(MULTI_TEST_ID).unwrap();
        println!("native_compress name=multi_test/do_nothing elapsed={elapsed:?}");

        let bytes = b"browser echo parity".to_vec();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::Echo {
                bytes: bytes.clone(),
            })
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("multi_test/echo", env, MULTI_TEST_ELF, MULTI_TEST_ID);
        assert_eq!(info.receipt.journal.bytes, bytes);

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::ShaCycleCount)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats(
            "multi_test/sha_cycle_count",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let mut output = Vec::new();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::ReadWriteMem {
                values: vec![(MEM_POS, 0x1234_5678), (MEM_POS, 0)],
            })
            .unwrap()
            .stdout(&mut output)
            .build()
            .unwrap();
        let info = prove_and_print_stats(
            "multi_test/read_write_mem",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
        assert!(info.receipt.journal.bytes.is_empty());
        assert_eq!(
            risc0_zkvm::serde::from_slice::<u32, u8>(&output).unwrap(),
            0x1234_5678
        );

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::PauseResume(7))
            .unwrap()
            .build()
            .unwrap();
        let started = Instant::now();
        let session = default_executor().execute(env, MULTI_TEST_ELF).unwrap();
        let elapsed = started.elapsed();
        println!(
            "native_execute name=multi_test/pause_resume elapsed={elapsed:?} segments={} user_cycles={} exit_code={:?}",
            session.segments.len(),
            session.cycles(),
            session.exit_code,
        );
        assert_eq!(session.exit_code, ExitCode::Paused(7));

        // `RunUnconstrained { unconstrained: true }` uses SYS_FORK, which is
        // disabled in the native syscall table in this checkout and is covered
        // by an ignored native test. It is classified as a native-disabled
        // fixture rather than active browser proving parity.
    }

    #[test]
    #[ignore = "manual helper for measuring browser syscall and IO native baselines"]
    fn native_syscall_and_io_stats() {
        use std::cell::RefCell;

        use bytes::Bytes;
        use risc0_zkvm::sha::{Digest, Digestible};
        use risc0_zkvm_methods::{
            multi_test::{MultiTestSpec, SYS_MULTI_TEST, SYS_MULTI_TEST_WORDS},
            MULTI_TEST_ELF, MULTI_TEST_ID,
        };

        const FD: u32 = 123;

        let expected: Vec<Bytes> = vec![
            Bytes::from_static(b""),
            Bytes::from_static(b"H"),
            Bytes::from_static(b"He"),
            Bytes::from_static(b"Hel"),
            Bytes::from_static(b"Hell"),
            Bytes::from_static(b"Hello"),
        ];
        let actual: RefCell<Vec<Bytes>> = RefCell::new(Vec::new());
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::Syscall {
                count: expected.len() as u32 - 1,
            })
            .unwrap()
            .io_callback(SYS_MULTI_TEST, |buf| {
                let mut actual = actual.borrow_mut();
                let response = expected[actual.len() + 1].clone();
                actual.push(buf);
                Ok(response)
            })
            .build()
            .unwrap();
        prove_and_print_stats("multi_test/syscall", env, MULTI_TEST_ELF, MULTI_TEST_ID);
        assert_eq!(*actual.borrow(), expected[..expected.len() - 1]);

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::SyscallWords)
            .unwrap()
            .io_callback(SYS_MULTI_TEST_WORDS, Ok)
            .build()
            .unwrap();
        prove_and_print_stats(
            "multi_test/syscall_words",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let digest = Digest::from([1, 2, 3, 4, 5, 6, 7, 8]);
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .input_digest(digest)
            .write(&MultiTestSpec::SysInput(digest))
            .unwrap()
            .build()
            .unwrap();
        let info = prove_with_opts_and_print_integrity_stats(
            "multi_test/sys_input",
            env,
            MULTI_TEST_ELF,
            &ProverOpts::succinct(),
        );
        let opened_claim = info.receipt.claim().unwrap();
        let claim = opened_claim.as_value().unwrap();
        assert_eq!(claim.exit_code, ExitCode::Halted(0));
        assert_eq!(claim.pre.digest(), Digest::from(MULTI_TEST_ID));
        assert_eq!(claim.input.digest(), digest);

        let initial = b"abcdefghijkl".to_vec();
        let readbuf = b"ABCDEFG".to_vec();
        let spec = MultiTestSpec::SysRead {
            fd: FD,
            buf: initial,
            pos_and_len: vec![(2, 6), (8, 4)],
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .read_fd(FD, &readbuf[..])
            .write(&spec)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("multi_test/sys_read", env, MULTI_TEST_ELF, MULTI_TEST_ID);
        let (actual, num_read): (Vec<u8>, Vec<usize>) = info.receipt.journal.decode().unwrap();
        assert_eq!(num_read, vec![6, 1]);
        assert_eq!(actual, b"abABCDEFG\0\0\0".to_vec());

        let mut stdout = Vec::new();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .read_fd(FD, "Hello world!".as_bytes())
            .write(&MultiTestSpec::EchoStdout { nbytes: 5, fd: FD })
            .unwrap()
            .stdout(&mut stdout)
            .build()
            .unwrap();
        prove_and_print_stats("multi_test/echo_stdout", env, MULTI_TEST_ELF, MULTI_TEST_ID);
        assert_eq!(stdout, b"Hello world!");

        let words: Vec<u32> = (0..32).collect();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .read_fd(FD, bytemuck::cast_slice(&words))
            .write(&MultiTestSpec::EchoWords {
                fd: FD,
                nwords: words.len() as u32,
            })
            .unwrap()
            .build()
            .unwrap();
        let info =
            prove_and_print_stats("multi_test/echo_words", env, MULTI_TEST_ELF, MULTI_TEST_ID);
        let actual: &[u32] = bytemuck::cast_slice(&info.receipt.journal.bytes);
        assert_eq!(actual, words.as_slice());
    }

    #[test]
    #[ignore = "manual focused CUDA repro for the Poseidon2 accelerator baseline"]
    fn native_poseidon2_basic_stats() {
        use risc0_zkvm_methods::{multi_test::MultiTestSpec, MULTI_TEST_ELF, MULTI_TEST_ID};

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::Poseidon2Basic)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats(
            "multi_test/poseidon2_basic",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );
    }

    #[test]
    #[ignore = "manual helper for measuring browser accelerator and precompile native baselines"]
    fn native_accelerator_and_precompile_stats() {
        use risc0_zkvm::sha::Digest;
        use risc0_zkvm_methods::{multi_test::MultiTestSpec, MULTI_TEST_ELF, MULTI_TEST_ID};

        fn prove_multi_stats(name: &str, spec: MultiTestSpec) -> ProveInfo {
            let env = ExecutorEnv::builder()
                .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
                .write(&spec)
                .unwrap()
                .build()
                .unwrap();
            prove_and_print_stats(name, env, MULTI_TEST_ELF, MULTI_TEST_ID)
        }

        fn prove_keccak_multi_stats(name: &str, spec: MultiTestSpec) -> ProveInfo {
            let mut builder = ExecutorEnv::builder();
            builder.segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2);
            builder
                .keccak_max_po2(WEBGPU_BASELINE_KECCAK_MAX_PO2)
                .unwrap();
            let env = builder.write(&spec).unwrap().build().unwrap();
            prove_and_print_stats(name, env, MULTI_TEST_ELF, MULTI_TEST_ID)
        }

        for (name, spec) in [
            ("multi_test/libm", MultiTestSpec::LibM),
            ("multi_test/poseidon2_basic", MultiTestSpec::Poseidon2Basic),
            ("multi_test/poseidon2_short", MultiTestSpec::Poseidon2Short),
            ("multi_test/poseidon2_long", MultiTestSpec::Poseidon2Long),
            (
                "multi_test/poseidon2_continue",
                MultiTestSpec::Poseidon2Continue,
            ),
            ("multi_test/sha_conforms", MultiTestSpec::ShaConforms),
            ("multi_test/rsa_compat", MultiTestSpec::RsaCompat),
            ("multi_test/do_random", MultiTestSpec::DoRandom),
            ("multi_test/aligned_alloc", MultiTestSpec::AlignedAlloc),
            ("multi_test/alloc_zeroed", MultiTestSpec::AllocZeroed),
        ] {
            prove_multi_stats(name, spec);
        }

        for (name, spec) in [
            ("multi_test/keccak_update", MultiTestSpec::KeccakUpdate),
            (
                "multi_test/sha_single_keccak",
                MultiTestSpec::ShaSingleKeccak,
            ),
            ("multi_test/sys_keccak", MultiTestSpec::SysKeccak),
        ] {
            prove_keccak_multi_stats(name, spec);
        }

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::ShaDigest {
                data: b"abc".to_vec(),
            })
            .unwrap()
            .build()
            .unwrap();
        let info =
            prove_and_print_stats("multi_test/sha_digest", env, MULTI_TEST_ELF, MULTI_TEST_ID);
        let digest = Digest::try_from(info.receipt.journal.bytes).unwrap();
        assert_eq!(
            hex::encode(digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::ShaDigestIter {
                data: vec![0u8; 32],
                num_iter: 16,
            })
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats(
            "multi_test/sha_digest_iter",
            env,
            MULTI_TEST_ELF,
            MULTI_TEST_ID,
        );

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::BigInt {
                count: 3,
                x: [1, 2, 3, 4, 5, 6, 7, 8],
                y: [9, 10, 11, 12, 13, 14, 15, 16],
                modulus: [17, 18, 19, 20, 21, 22, 23, 24],
            })
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("multi_test/bigint", env, MULTI_TEST_ELF, MULTI_TEST_ID);

        const BIGINT_LEGAL_ADDR: u32 = 0x3000_0000;
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&MultiTestSpec::BigIntRaw {
                result: BIGINT_LEGAL_ADDR,
                x: BIGINT_LEGAL_ADDR,
                y: BIGINT_LEGAL_ADDR,
                modulus: BIGINT_LEGAL_ADDR,
            })
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("multi_test/bigint_raw", env, MULTI_TEST_ELF, MULTI_TEST_ID);

        prove_keccak_multi_stats("multi_test/keccak_update2", MultiTestSpec::KeccakUpdate2);
        prove_keccak_multi_stats("multi_test/keccak_union", MultiTestSpec::KeccakUnion(3));
    }

    #[test]
    #[ignore = "manual helper for measuring the browser blst native baseline"]
    fn native_blst_prove_stats() {
        use risc0_zkvm_methods::{BLST_ELF, BLST_ID};

        let info = prove_and_print_stats(
            "risc0-zkvm-methods/blst",
            ExecutorEnv::builder()
                .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
                .build()
                .unwrap(),
            BLST_ELF,
            BLST_ID,
        );
        assert_eq!(
            info.receipt.journal.decode::<String>().unwrap(),
            "blst is such a blast"
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser benchmark native baseline"]
    fn native_bench_prove_stats() {
        use risc0_zkvm_methods::{bench::BenchmarkSpec, BENCH_ELF, BENCH_ID};

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&BenchmarkSpec::SimpleLoop { iters: 16 })
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats(
            "risc0-zkvm-methods/bench/simple_loop",
            env,
            BENCH_ELF,
            BENCH_ID,
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser test-feature native baseline"]
    fn native_test_feature_prove_stats() {
        use risc0_zkvm_methods::{TEST_FEATURE_ELF, TEST_FEATURE_ID};

        prove_and_print_stats(
            "risc0-zkvm-methods/test_feature",
            ExecutorEnv::builder()
                .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
                .build()
                .unwrap(),
            TEST_FEATURE_ELF,
            TEST_FEATURE_ID,
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser hello-world native baseline"]
    fn native_hello_world_prove_stats() {
        use hello_world_methods::{MULTIPLY_ELF, MULTIPLY_ID};

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&17u64)
            .unwrap()
            .write(&23u64)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("hello-world", env, MULTIPLY_ELF, MULTIPLY_ID);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser json native baseline"]
    fn native_json_prove_stats() {
        use json_methods::{SEARCH_JSON_ELF, SEARCH_JSON_ID};

        let data = include_str!("../../json/res/example.json");
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&data)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("json", env, SEARCH_JSON_ELF, SEARCH_JSON_ID);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser chess native baseline"]
    fn native_chess_prove_stats() {
        use chess_core::Inputs;
        use chess_methods::{CHECKMATE_ELF, CHECKMATE_ID};

        const BOARD: &str = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4";
        const MOVE: &str = "Qxf7";

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&Inputs {
                board: BOARD.to_string(),
                mv: MOVE.to_string(),
            })
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("chess", env, CHECKMATE_ELF, CHECKMATE_ID);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser composition native baseline"]
    fn native_composition_prove_stats() {
        use composition_example_methods::{EXPONENTIATE_ELF, EXPONENTIATE_ID};
        use hello_world_methods::{MULTIPLY_ELF, MULTIPLY_ID};

        let multiply_env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&17u64)
            .unwrap()
            .write(&23u64)
            .unwrap()
            .build()
            .unwrap();
        let multiply_info = prove_and_print_stats(
            "composition/multiply-assumption",
            multiply_env,
            MULTIPLY_ELF,
            MULTIPLY_ID,
        );
        let n: u64 = multiply_info.receipt.journal.decode().unwrap();

        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .add_assumption(multiply_info.receipt)
            .write(&(n, 9u64, 100u64))
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("composition", env, EXPONENTIATE_ELF, EXPONENTIATE_ID);
        assert_eq!(
            info.receipt.journal.decode::<(u64, u64, u64)>().unwrap(),
            (391, 9, 32)
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser jwt-validator native baseline"]
    fn native_jwt_validator_prove_stats() {
        use jwt_core::{CustomClaims, Issuer};
        use jwt_methods::{VALIDATOR_ELF, VALIDATOR_ID};

        const SECRET_KEY: &str = r#"
    {
      "alg": "RS256",
      "d": "YuO1XZkYSwDRgauXQe6q1u8fET3S7x7g4N8uE49rdt7g3-O9q-Hwn_nQNiRr9o7Uslf7X8sL6txraQy7TdPUuSkaULpRNo2FoVLLoO2eACWwPtCG4n9wuvjnz7qCh9s3tfgOKxMA_riKkS8O7BxPH54rd7Ry1i6HN3TSYKYwxZxG4HFLhcewX6Q1KdGXdP7xVAsZ5lEpCQbhY5IKUzBZ5WIZpSTk10AadkVuwS622QT-9efk6PBWDyM48_udMdDo1HEcHsAdxrUMRdw_5uzVajQzZhNAmALXHCPT79P0qahzdYlUSHauT1XxU7z-KoCYVqt3z6epgYDcKmLzGkqIkSXUHxcVN-MTSGNET_dhio0tHG-jV3wB5jfsgayoIZCeTPF-F-nDwn8Cyz18uee_Y7U53NTtEXGqB9npZyu7SibTztwSeLs6zH965d1VTmUCxH8CWqizugfQY8ibNgVCd42naAuWbOmxYEjyelmHf_BS0Vb7NwpW9cuaODOjpjCz",
      "dp": "DOIAbWzet_-ZSED61WWvG9Byao9uQh3SSvtvAUa4WhWEq3lfGqt1wEDneOds1IrxNF7Y2rV_iHBVA2DWB9ctdMxau3DteGumbMzEObQIjDs7SP45plImxHzZbXgTIB-DWiujJmwDNJUIaB80q1sjeeBTJ9rfaU0ZNMFO26koOKQGoNDuuJTgejnRwGdIGhoOLcT_dus-7CNWY1pRBvTGhcEOygRE_icb8JzNoKo90fwZf0ACdxiFc6G_RUCXapap",
      "dq": "0yFAtOVm0-fPLg62RcALyhIXsyEOd25W0YFmWIzb6Bh5kbMruA-befX-ANnNGcktBGgY7QGN6myb-K8zRCOYfVt5zs0EFEFCHc6NO8UoSJCItOZFMdaLsG21MqOdtQRQi4F_TJ2yoqu1S81O-Y08wtFE0F8hVe7sGuJIoRtY5yF_Swwaw3ST-XMfghpbhvc71zVF7VyPlyrqU-NeKimBpuEfHuTKQSSudY9eLNdypyE71RC6q_xWxWzTSqu3pih5",
      "e": "AQAB",
      "key_ops": ["sign"],
      "kty": "RSA",
      "n": "zcQwXx3EevOSkfH0VSWqtfmWTL4c2oIzW6u83qKO1W7XjLgTqpryL5vNCaxbVTkpU-GZctit0n6kj570tfny_sy6pb2q9wlvFBmDVyD-nL5oNjP5s3qEfvy15Bl9vMGFf3zycqMaVg_7VRVwK5d8QzpnVC0AGT10QdHnyGCadfPJqazTuVRp1f3ecK7bg7596sgVb8d9Wpaz2XPykQPfphsEb40vcp1tPN95-eRCgA24PwfUaKYHQQFMEQY_atJWbffyJ91zsBRy8fEQdfuQVZIRVQgO7FTsmLmQAHxR1dl2jP8B6zonWmtqWoMHoZfa-kmTPB4wNHa8EaLvtQ1060qYFmQWWumfNFnG7HNq2gTHt1cN1HCwstRGIaU_ZHubM_FKH_gLfJPKNW0KWML9mQQzf4AVov0Yfvk89WxY8ilSRx6KodJuIKKqwVh_58PJPLmBqszEfkTjtyxPwP8X8xRXfSz-vTU6vESCk3O6TRknoJkC2BJZ_ONQ0U5dxLcx",
      "p": "-TQVt9yl_0S0uvUM37L3WSDPkOn_gy34zpAEllhgx1HQUg_pVbqEDwKzEIpBlZfbrcszMlmiJhKL6q4y0_a6e3O5QnfB1vrGTjhLcfcaUK6o-I7bxabrpZmvLIsTqSdAgUijXe8yhQFIoCjc1MPD7icRPc-V7P9IYE2ls9X6sgo4lUZjQAuQtOo8ndlZ3uqP2sMKRR3CS7tHiF1r_zq_NXcf98Sve-1rRnqT6GpGcJRcvVFu2wy8TyCPMAvWh903",
      "q": "02DUlUJrcTQ-mHMmg-V5qjxrtTKMmjqXpN0pgkXhM8_DWCrqKL9sXb1MKXQcbAZYr-lWmtBwzXeF4Qn66dRHpjlQLhSA947UxjuEtbhWx3wKGG460ZH026qcRr3QspcKZuiX2zISHb8suMl2lhDDSggCAjybs0l72pNHPIny9pucnwqc9ihrbeu68LlUpnQtS-Okt4j5ndVc1l1Vwv2PFt2PxrLmQkqdwRMla1F7r0vtgM7NIZz9XPszSrkxTILX",
      "qi": "3yweZ6b2adwqUrCvyvK5ub5XAjKOh1N7AoFqYQFpD_ho41ThyWErfjTztDlgqqTHo3wHyR49cq-L6aAuerNTPW7VAXTobC8vZSxIKazOU9p0xcDYSaGGH_IES62MAxJu1rdyAOrq_MLsqvBckVancmW6lVWQr27wDNTNwskkPpgDXwAygWSCBbM-oZOsWamge0SadQJOCd7Rr33aLfWFKaajl7FnQzX6Wh8Q0gLn2PRDnC7V1gEVWY3fWSzs4obj",
      "use": "sig",
      "kid": "6ab0e8e4bc121fc287e35d3e5e0efb8a"
    }
"#;

        let claims = CustomClaims {
            subject: "Hello, world!".to_string(),
        };
        let issuer = SECRET_KEY.parse::<Issuer>().unwrap();
        let token = issuer.generate_token(&claims).unwrap();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&token)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("jwt-validator", env, VALIDATOR_ELF, VALIDATOR_ID);
        assert_eq!(
            info.receipt.journal.decode::<String>().unwrap(),
            claims.subject
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser bevy native baseline"]
    fn native_bevy_prove_stats() {
        use bevy_core::Outputs;
        use bevy_methods::{BEVY_GUEST_ELF, BEVY_GUEST_ID};

        let turns = 3u32;
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&turns)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("bevy", env, BEVY_GUEST_ELF, BEVY_GUEST_ID);
        let outputs: Outputs = info.receipt.journal.decode().unwrap();
        assert_eq!(outputs.position, turns as f32);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser digital-signature native baseline"]
    fn native_digital_signature_prove_stats() {
        use digital_signature_core::SigningRequest;
        use digital_signature_methods::{SIGN_ELF, SIGN_ID};
        use risc0_zkvm::sha::{Impl, Sha256};

        let request = SigningRequest {
            passphrase: *Impl::hash_bytes(b"passphr4ase"),
            msg: *Impl::hash_bytes(b"This message was signed by me"),
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&request)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("digital-signature", env, SIGN_ELF, SIGN_ID);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser groth16-verifier native baseline"]
    fn native_groth16_verifier_prove_stats() {
        use groth16_verifier_methods::{GROTH16_VERIFIER_ELF, GROTH16_VERIFIER_ID};
        use risc0_groth16::{ProofJson, PublicInputsJson, VerifyingKeyJson};

        let proof_json: ProofJson =
            serde_json::from_str(include_str!("../../groth16-verifier/src/data/proof.json"))
                .unwrap();
        let public_inputs_json = PublicInputsJson {
            values: serde_json::from_str(include_str!(
                "../../groth16-verifier/src/data/public.json"
            ))
            .unwrap(),
        };
        let verifying_key_json: VerifyingKeyJson = serde_json::from_str(include_str!(
            "../../groth16-verifier/src/data/verification_key.json"
        ))
        .unwrap();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&(&proof_json, &public_inputs_json, &verifying_key_json))
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats(
            "groth16-verifier",
            env,
            GROTH16_VERIFIER_ELF,
            GROTH16_VERIFIER_ID,
        );
        let (_vk_digest, _public_inputs_digest): (Digest, Digest) =
            info.receipt.journal.decode().unwrap();
    }

    #[test]
    #[ignore = "manual helper for measuring the browser prorata native baseline"]
    fn native_prorata_prove_stats() {
        use prorata_core::{AllocationQuery, AllocationQueryResult};
        use prorata_methods::{PRORATA_GUEST_ELF, PRORATA_GUEST_ID};
        use rust_decimal::Decimal;

        let query = AllocationQuery {
            amount: Decimal::new(10000, 2),
            recipients_csv: b"name,share\nAlice,0.5\nBob,0.25\nCarol,0.25\n".to_vec(),
            target: "Alice".to_string(),
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&query)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("prorata", env, PRORATA_GUEST_ELF, PRORATA_GUEST_ID);
        let result: AllocationQueryResult = info.receipt.journal.decode().unwrap();
        assert_eq!(result.allocation.unwrap().name, "Alice");
    }

    #[test]
    #[ignore = "manual helper for measuring the browser wasm interpreter native baseline"]
    fn native_wasm_interpreter_prove_stats() {
        use wasm_methods::{WASM_INTERP_ELF, WASM_INTERP_ID};

        let wasm = wat::parse_str(
            r#"
            (module
                (export "fib" (func $fib))
                (func $fib (param $n i32) (result i32)
                    (local $a i32)
                    (local $b i32)
                    (local $tmp i32)
                    (local.set $a (i32.const 0))
                    (local.set $b (i32.const 1))
                    (block $exit
                        (loop $loop
                            (br_if $exit (i32.eqz (local.get $n)))
                            (local.set $tmp (local.get $a))
                            (local.set $a (local.get $b))
                            (local.set $b (i32.add (local.get $tmp) (local.get $b)))
                            (local.set $n (i32.sub (local.get $n) (i32.const 1)))
                            (br $loop)
                        )
                    )
                    (local.get $a)
                )
            )
        "#,
        )
        .unwrap();
        let iters = 10i32;
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&wasm)
            .unwrap()
            .write(&iters)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("wasm", env, WASM_INTERP_ELF, WASM_INTERP_ID);
        assert_eq!(info.receipt.journal.decode::<i32>().unwrap(), 55);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser xgboost native baseline"]
    fn native_xgboost_prove_stats() {
        use forust_ml::GradientBooster;
        use xgboost_methods::{XGBOOST_ELF, XGBOOST_ID};

        let model: GradientBooster =
            serde_json::from_str(include_str!("../../xgboost/res/trained_model.json")).unwrap();
        let model_bytes = rmp_serde::to_vec(&model).unwrap();
        let data: Vec<f64> = vec![18511304.0, 117.0];
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&data)
            .unwrap()
            .write(&model_bytes)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("xgboost", env, XGBOOST_ELF, XGBOOST_ID);
        assert_eq!(
            info.receipt.journal.decode::<f64>().unwrap(),
            30.528042544062632
        );
    }

    #[test]
    #[ignore = "manual helper for measuring the browser bn254 native baseline"]
    fn native_bn254_prove_stats() {
        use bn254_core::Inputs;
        use bn254_methods::{BN254_VERIFY_ELF, BN254_VERIFY_ID};

        let input = Inputs {
            g1_compressed: hex::decode(
                "020000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            g2_compressed: hex::decode(
                "0A04D4BF3239F77CEE7B47C7245E9281B3E9C1182D6381A87BBF81F9F2A6254B731DF569CDA95E060BEE91BA69B3F2D103658A7AEA6B10E5BDC761E5715E7EE4BB",
            )
            .unwrap(),
            a: hex::decode("9c0d02eaaf8e7e7ad09595ef6e3b896f8915124ba5bef9287f0997557580caeb")
                .unwrap(),
            b: hex::decode("db6764642f7bb1f415d93fcd5aace586161ec2e4305f0d6fb57dbabf1d141a5b")
                .unwrap(),
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("bn254", env, BN254_VERIFY_ELF, BN254_VERIFY_ID);
        assert!(info.receipt.journal.decode::<bool>().unwrap());
    }

    #[test]
    #[ignore = "manual helper for measuring the browser password-checker native baseline"]
    fn native_password_checker_prove_stats() {
        use password_checker_core::PasswordRequest;
        use password_checker_methods::{PW_CHECKER_ELF, PW_CHECKER_ID};

        let request = PasswordRequest {
            password: "S00perSecr1t!!!".to_string(),
            salt: [0u8; 32],
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&request)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("password-checker", env, PW_CHECKER_ELF, PW_CHECKER_ID);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser voting-machine native baseline"]
    fn native_voting_machine_prove_stats() {
        use risc0_zkvm::serde::from_slice;
        use voting_machine_core::{
            Ballot, FreezeVotingMachineParams, FreezeVotingMachineResult, SubmitBallotParams,
            VotingMachineState,
        };
        use voting_machine_methods::{
            FREEZE_ELF, FREEZE_ID, INIT_ELF, INIT_ID, SUBMIT_ELF, SUBMIT_ID,
        };

        let mut state = VotingMachineState {
            polls_open: true,
            voter_bitfield: 0,
            count: 0,
        };

        let init_env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&state)
            .unwrap()
            .build()
            .unwrap();
        prove_and_print_stats("voting-machine/init", init_env, INIT_ELF, INIT_ID);

        let ballot = Ballot {
            voter: 1,
            vote_yes: true,
        };
        let params = SubmitBallotParams::new(state.clone(), ballot);
        let mut submit_output = Vec::new();
        let submit_env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&params)
            .unwrap()
            .stdout(&mut submit_output)
            .build()
            .unwrap();
        prove_and_print_stats("voting-machine/submit", submit_env, SUBMIT_ELF, SUBMIT_ID);
        state = from_slice(&submit_output).unwrap();

        let params = FreezeVotingMachineParams::new(state.clone());
        let mut freeze_output = Vec::new();
        let freeze_env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&params)
            .unwrap()
            .stdout(&mut freeze_output)
            .build()
            .unwrap();
        prove_and_print_stats("voting-machine/freeze", freeze_env, FREEZE_ELF, FREEZE_ID);
        let result: FreezeVotingMachineResult = from_slice(&freeze_output).unwrap();
        assert!(!result.state.polls_open);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser keccak native baseline"]
    fn native_keccak_prove_stats() {
        use keccak_methods::{KECCAK_ELF, KECCAK_ID};

        let input = "abc";
        let mut builder = ExecutorEnv::builder();
        builder.segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2);
        builder
            .keccak_max_po2(WEBGPU_BASELINE_KECCAK_MAX_PO2)
            .unwrap();
        let env = builder.write(&input).unwrap().build().unwrap();
        prove_and_print_stats("keccak", env, KECCAK_ELF, KECCAK_ID);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser smartcore-ml native baseline"]
    fn native_smartcore_ml_prove_stats() {
        use smartcore::{
            linalg::basic::matrix::DenseMatrix,
            tree::decision_tree_classifier::DecisionTreeClassifier,
        };
        use smartcore_ml_methods::{ML_TEMPLATE_ELF, ML_TEMPLATE_ID};

        type Model = DecisionTreeClassifier<f64, u32, DenseMatrix<f64>, Vec<u32>>;

        let is_svm = false;
        let model: Model = rmp_serde::from_slice(include_bytes!(
            "../../smartcore-ml/res/ml-model/tree_model_bytes.bin"
        ))
        .unwrap();
        let data: DenseMatrix<f64> = rmp_serde::from_slice(include_bytes!(
            "../../smartcore-ml/res/input-data/tree_model_data_bytes.bin"
        ))
        .unwrap();
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&is_svm)
            .unwrap()
            .write(&model)
            .unwrap()
            .write(&data)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("smartcore-ml", env, ML_TEMPLATE_ELF, ML_TEMPLATE_ID);
        let result: Vec<u32> = info.receipt.journal.decode().unwrap();
        assert_eq!(result.len(), 150);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser wordle native baseline"]
    fn native_wordle_prove_stats() {
        use wordle_core::{GameState, WordFeedback};
        use wordle_methods::{WORDLE_GUEST_ELF, WORDLE_GUEST_ID};

        let secret = "world";
        let guess = "worry";
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&secret)
            .unwrap()
            .write(&guess)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("wordle", env, WORDLE_GUEST_ELF, WORDLE_GUEST_ID);
        let state: GameState = info.receipt.journal.decode().unwrap();
        let feedback: WordFeedback = state.feedback;
        assert_eq!(feedback.0.len(), 5);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser sha native baseline"]
    fn native_sha_prove_stats() {
        use sha_methods::{HASH_ELF, HASH_ID, HASH_RUST_CRYPTO_ELF, HASH_RUST_CRYPTO_ID};

        for (name, elf, image_id) in [
            ("sha/hash", HASH_ELF, HASH_ID),
            (
                "sha/hash-rust-crypto",
                HASH_RUST_CRYPTO_ELF,
                HASH_RUST_CRYPTO_ID,
            ),
        ] {
            let input = "abc";
            let env = ExecutorEnv::builder()
                .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
                .write(&input)
                .unwrap()
                .build()
                .unwrap();
            prove_and_print_stats(name, env, elf, image_id);
        }
    }

    #[test]
    #[ignore = "manual helper for measuring the browser waldo native baseline"]
    fn native_waldo_prove_stats() {
        use image::{DynamicImage, RgbImage};
        use waldo_core::{
            image::{ImageMerkleTree, IMAGE_CHUNK_SIZE},
            merkle::SYS_VECTOR_ORACLE,
            Journal, PrivateInput,
        };
        use waldo_methods::{IMAGE_CROP_ELF, IMAGE_CROP_ID};

        let mut raw = Vec::new();
        for i in 0..16 * 16 {
            raw.extend_from_slice(&[i as u8, (i * 3) as u8, (255 - i) as u8]);
        }
        let image = DynamicImage::ImageRgb8(RgbImage::from_raw(16, 16, raw).unwrap());
        let tree = ImageMerkleTree::<{ IMAGE_CHUNK_SIZE }>::new(&image);
        let input = PrivateInput {
            root: tree.root(),
            image_dimensions: (16, 16),
            crop_location: (7, 7),
            crop_dimensions: (3, 3),
            mask: None,
        };
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&input)
            .unwrap()
            .io_callback(SYS_VECTOR_ORACLE, tree.vector_oracle_callback())
            .build()
            .unwrap();
        let info = prove_and_print_stats("waldo", env, IMAGE_CROP_ELF, IMAGE_CROP_ID);
        let journal: Journal = info.receipt.journal.decode().unwrap();
        assert_eq!(journal.subimage_dimensions, (3, 3));
    }

    #[test]
    #[ignore = "manual helper for measuring the browser ecdsa/k256 native baseline"]
    fn native_ecdsa_k256_prove_stats() {
        use k256::{
            ecdsa::{signature::Signer, Signature, SigningKey},
            EncodedPoint,
        };
        use k256_methods::{K256_VERIFY_ELF, K256_VERIFY_ID};

        let signing_key = SigningKey::from_bytes((&[7u8; 32]).into()).unwrap();
        let message = b"This is a message that will be signed, and verified within the zkVM";
        let signature: Signature = signing_key.sign(message);
        let input = (
            signing_key.verifying_key().to_encoded_point(true),
            message.to_vec(),
            signature,
        );
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("ecdsa/k256", env, K256_VERIFY_ELF, K256_VERIFY_ID);
        let (_key, msg): (EncodedPoint, Vec<u8>) = info.receipt.journal.decode().unwrap();
        assert_eq!(msg, message);
    }

    #[test]
    #[ignore = "manual helper for measuring the browser ecdsa/p256 native baseline"]
    fn native_ecdsa_p256_prove_stats() {
        use p256::{
            ecdsa::{signature::Signer, Signature, SigningKey},
            EncodedPoint,
        };
        use p256_methods::{P256_VERIFY_ELF, P256_VERIFY_ID};

        let signing_key = SigningKey::from_bytes((&[9u8; 32]).into()).unwrap();
        let message = b"This is a message that will be signed, and verified within the zkVM";
        let signature: Signature = signing_key.sign(message);
        let input = (
            signing_key.verifying_key().to_encoded_point(true),
            message.to_vec(),
            signature,
        );
        let env = ExecutorEnv::builder()
            .segment_limit_po2(WEBGPU_BASELINE_SEGMENT_LIMIT_PO2)
            .write(&input)
            .unwrap()
            .build()
            .unwrap();
        let info = prove_and_print_stats("ecdsa/p256", env, P256_VERIFY_ELF, P256_VERIFY_ID);
        let (_key, msg): (EncodedPoint, Vec<u8>) = info.receipt.journal.decode().unwrap();
        assert_eq!(msg, message);
    }
}
