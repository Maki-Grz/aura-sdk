use anyhow::Result;
use clap::Parser;
use std::time::Instant;

fn main() -> Result<()> {
    let args = aura_sdk::common::Args::parse();

    if args.ort {
        #[cfg(feature = "aura-engine")]
        {
            return aura_sdk::engines::run_aura_engine(&args);
        }
        #[cfg(not(feature = "aura-engine"))]
        {
            return Err(anyhow::anyhow!(
                "Aura Engine not enabled. Build with --features aura-engine"
            ));
        }
    }

    if std::env::var("ADSP_LIBRARY_PATH").is_err() {
        if let Ok(sdk_root) = std::env::var("QNN_SDK_ROOT") {
            let adsp_path = format!("{}\\lib\\hexagon-v73\\unsigned", sdk_root);
            if std::path::Path::new(&adsp_path).exists() {
                std::env::set_var("ADSP_LIBRARY_PATH", &adsp_path);
            }
        }
    }

    println!("🚀 Aura SDK starting (Native Mode)...");
    println!("--- Snapdragon AI Runtime Diagnostics ---");
    println!(
        "SDK_ROOT:  {:?}",
        std::env::var("QNN_SDK_ROOT").unwrap_or_default()
    );
    println!(
        "ADSP_PATH: {:?}",
        std::env::var("ADSP_LIBRARY_PATH").unwrap_or_default()
    );
    println!("------------------------------------\n");

    let engine = aura_sdk::engines::genie::GenieEngine::new(&args.config)?;
    let stats = aura_sdk::common::Stats::new(args.max_tokens);

    println!("\nPrompt: {}\nResponse:", args.prompt);
    let total_start = Instant::now();

    engine.query(&args.prompt, &stats)?;

    while !stats.is_done() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    println!();

    let total_duration = total_start.elapsed();
    let tokens = stats.get_token_count();
    println!(
        "\n--- Performance Metrics ---\nTotal tokens:    {}\nTotal time:      {:.2?}",
        tokens, total_duration
    );
    if total_duration.as_secs_f64() > 0.0 {
        println!(
            "TPS:             {:.2} tokens/s",
            tokens as f64 / total_duration.as_secs_f64()
        );
    }

    Ok(())
}
