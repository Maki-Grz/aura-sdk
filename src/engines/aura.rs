#[cfg(feature = "aura-engine")]
use anyhow::Context;
#[cfg(feature = "aura-engine")]
use ort::{
    execution_providers::{
        qnn::PerformanceMode, ArbitrarilyConfigurableExecutionProvider, QNNExecutionProvider,
    },
    session::Session,
};
#[cfg(feature = "aura-engine")]
use std::path::Path;

#[cfg(feature = "aura-engine")]
pub struct OrtSession {
    session: Session,
}

#[cfg(feature = "aura-engine")]
impl OrtSession {
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        backend_path: &str,
        context_path: &str,
        soc_model: &str,
    ) -> anyhow::Result<Self> {
        let qnn_provider = QNNExecutionProvider::default()
            .with_backend_path(backend_path)
            .with_performance_mode(PerformanceMode::Burst)
            .with_arbitrary_config("htp_arch", "v73")
            .with_arbitrary_config("soc_model", soc_model)
            .with_arbitrary_config("enable_htp_fp16_precision", "1")
            .with_arbitrary_config("rpc_control_latency", "100")
            .with_arbitrary_config("qnn_context_cache_enable", "1")
            .with_arbitrary_config("qnn_context_cache_path", context_path)
            .build();

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("Session builder error: {}", e))?
            .with_execution_providers([qnn_provider])
            .map_err(|e| anyhow::anyhow!("Provider error: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| anyhow::anyhow!("Commit error: {}", e))?;

        Ok(Self { session })
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

#[cfg(feature = "aura-engine")]
pub fn run_aura_engine(args: &crate::common::Args) -> anyhow::Result<()> {
    use ort::value::{DynTensor, Value};
    use std::collections::HashMap;
    use tokenizers::Tokenizer;

    if std::env::var("ADSP_LIBRARY_PATH").is_err() {
        if let Ok(sdk_root) = std::env::var("QNN_SDK_ROOT") {
            let adsp_path = format!("{}\\lib\\hexagon-v73\\unsigned", sdk_root);
            if Path::new(&adsp_path).exists() {
                std::env::set_var("ADSP_LIBRARY_PATH", &adsp_path);
            }
        }
    }

    if let Ok(sdk_root) = std::env::var("QNN_SDK_ROOT") {
        let lib_path = format!("{}\\lib\\aarch64-windows-msvc", sdk_root);
        if let Ok(current_path) = std::env::var("PATH") {
            if !current_path.contains(&lib_path) {
                std::env::set_var("PATH", format!("{};{}", lib_path, current_path));
            }
        }
    }

    let sdk_root = std::env::var("QNN_SDK_ROOT").context("QNN_SDK_ROOT not set")?;
    let backend_path = format!("{}\\lib\\aarch64-windows-msvc\\QnnHtp.dll", sdk_root);
    let model_path = args
        .model
        .as_ref()
        .context("Model path required for ORT mode")?;
    let model_dir = model_path.parent().unwrap_or(Path::new("."));

    let mut tokenizer_path = model_dir.join("tokenizer.json");
    if !tokenizer_path.exists() {
        if let Some(p) = model_dir.parent() {
            let pt = p.join("tokenizer.json");
            if pt.exists() {
                tokenizer_path = pt;
            }
        }
    }
    let tokenizer = Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| anyhow::anyhow!("Tokenizer load failed: {}", e))?;

    let abs_model_dir = std::fs::canonicalize(model_dir)?;
    let model_stem = model_path.file_stem().unwrap_or_default().to_string_lossy();
    let mut output_bin_str = abs_model_dir
        .join(format!("{}_qnn_cache.bin", model_stem))
        .to_string_lossy()
        .into_owned();
    if output_bin_str.starts_with(r"\\?\") {
        output_bin_str = output_bin_str.trim_start_matches(r"\\?\").to_string();
    }

    let mut session_wrapper =
        OrtSession::new(model_path, &backend_path, &output_bin_str, &args.soc_model)?;
    let session = session_wrapper.session_mut();

    let mut formatted_prompt = args.prompt.clone();
    if !formatted_prompt.contains("<|start_header_id|>") {
        formatted_prompt = format!("<|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n", formatted_prompt);
    }
    let mut tokens: Vec<i64> = tokenizer
        .encode(formatted_prompt.as_str(), true)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .get_ids()
        .iter()
        .map(|&id| id as i64)
        .collect();
    let mut current_tokens_to_send = tokens.clone();
    let mut current_past_key_values: HashMap<String, Value> = HashMap::new();
    let output_names: Vec<String> = session
        .outputs()
        .iter()
        .map(|o| o.name().to_string())
        .collect();

    println!("Assistant (Aura):");
    let gen_start = std::time::Instant::now();
    for step in 0..args.max_tokens {
        let input_descriptors: Vec<(String, ort::value::ValueType)> = session
            .inputs()
            .iter()
            .map(|input| (input.name().to_string(), input.dtype().clone()))
            .collect();
        let mut inputs = Vec::new();
        let total_len = tokens.len() as i64;
        let past_len = if step == 0 { 0 } else { total_len - 1 };
        for (name, ty) in &input_descriptors {
            let lowered = name.to_ascii_lowercase();
            if lowered.contains("input_ids") {
                inputs.push((
                    name.clone(),
                    Value::from_array((
                        vec![1, current_tokens_to_send.len() as i64],
                        current_tokens_to_send.clone(),
                    ))?
                    .into_dyn(),
                ));
            } else if lowered.contains("attention_mask") {
                inputs.push((
                    name.clone(),
                    Value::from_array((vec![1, total_len], vec![1i64; total_len as usize]))?
                        .into_dyn(),
                ));
            } else if lowered.contains("position_ids") {
                let pos: Vec<i64> = (0..current_tokens_to_send.len() as i64)
                    .map(|i| past_len + i)
                    .collect();
                inputs.push((
                    name.clone(),
                    Value::from_array((vec![1, current_tokens_to_send.len() as i64], pos))?
                        .into_dyn(),
                ));
            } else if lowered.contains("past_key_values") || lowered.contains("past") {
                let search_name = name
                    .replace("past_key_values", "present")
                    .replace("past", "present");
                if let Some(val) = current_past_key_values.remove(&search_name) {
                    inputs.push((name.clone(), val));
                } else if let ort::value::ValueType::Tensor {
                    ty: el_ty, shape, ..
                } = ty
                {
                    let mut res_shape: Vec<i64> =
                        shape.iter().map(|&d| if d < 0 { 1 } else { d }).collect();
                    if res_shape.len() >= 3 {
                        res_shape[2] = 0;
                    }
                    inputs.push((
                        name.clone(),
                        DynTensor::new(
                            &ort::memory::Allocator::default(),
                            el_ty.clone(),
                            res_shape,
                        )?
                        .into_dyn(),
                    ));
                }
            }
        }
        let outputs = session.run(inputs)?;
        let next_token = {
            let idx = output_names
                .iter()
                .position(|n| n.contains("logits") || n == "output" || n == "output_0")
                .unwrap_or(0);
            let (logits_shape, logits_data) = outputs[idx].try_extract_tensor::<f32>()?;
            let logits_view = ndarray::ArrayViewD::from_shape(
                logits_shape.iter().map(|&d| d as usize).collect::<Vec<_>>(),
                logits_data,
            )?;
            let last_logits = logits_view.slice(ndarray::s![0, -1, ..]);
            let token = last_logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i as i64)
                .unwrap();
            token
        };
        if matches!(next_token, 128001 | 128009 | 2 | 1) {
            break;
        }
        tokens.push(next_token);
        current_tokens_to_send = vec![next_token];
        current_past_key_values.clear();
        for (name, value) in outputs.into_iter() {
            if name.contains("present") {
                current_past_key_values.insert(name.to_string(), value);
            }
        }
        print!(
            "{}",
            tokenizer
                .decode(&[next_token as u32], true)
                .unwrap_or_default()
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    println!();
    let elapsed = gen_start.elapsed();
    if elapsed.as_secs_f64() > 0.0 {
        let n_tokens = tokens.len() as f64;
        println!(
            "\n--- Aura Engine (NPU) ---\nTokens: {} | Time: {:.2?} | TPS: {:.2}",
            n_tokens,
            elapsed,
            n_tokens / elapsed.as_secs_f64()
        );
    }
    Ok(())
}
