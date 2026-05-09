use anyhow::{Context, Result};
use clap::Parser;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

pub mod genie {
    #![allow(
        non_upper_case_globals,
        non_camel_case_types,
        non_snake_case,
        dead_code,
        clippy::all
    )]
    include!(concat!(env!("OUT_DIR"), "/genie_bindings.rs"));
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Path to the genie_config.json file
    #[arg(
        short,
        long,
        default_value = "phi_3_5_mini_instruct-genie-w4a16-qualcomm/genie_config.json"
    )]
    config: PathBuf,

    // Prompt to send to the model
    #[arg(
        short,
        long,
        default_value = "Explain quantum physics in one sentence."
    )]
    prompt: String,

    /// Maximum number of tokens to generate
    #[arg(short, long, default_value = "512")]
    max_tokens: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

struct Stats {
    done: AtomicBool,
    token_count: AtomicUsize,
    consecutive_whitespace: AtomicUsize,
    start_time: std::sync::Mutex<Option<Instant>>,
    max_tokens: usize,
}

extern "C" fn query_callback(
    response: *const c_char,
    sentence_code: genie::GenieDialog_SentenceCode_t,
    user_data: *const c_void,
) {
    let stats = unsafe { &*(user_data as *const Stats) };
    if stats.done.load(Ordering::SeqCst) {
        return;
    }

    if !response.is_null() {
        let mut start_lock = stats.start_time.lock().unwrap();
        if start_lock.is_none() {
            *start_lock = Some(Instant::now());
        }

        let c_str = unsafe { CStr::from_ptr(response) };
        if let Ok(s) = c_str.to_str() {
            // Check for stop tokens or empty/whitespace loops
            let is_stop = s.contains("<|end|>")
                || s.contains("<|user|>")
                || s.contains("<|endoftext|>")
                || s.contains("</s>");

            if is_stop {
                stats.done.store(true, Ordering::SeqCst);
                return;
            }

            // Detect infinite whitespace loops
            if s.trim().is_empty() && !s.is_empty() {
                let count = stats.consecutive_whitespace.fetch_add(1, Ordering::SeqCst);
                if count > 5 && stats.token_count.load(Ordering::SeqCst) > 0 {
                    stats.done.store(true, Ordering::SeqCst);
                    return;
                }
            } else {
                stats.consecutive_whitespace.store(0, Ordering::SeqCst);
            }

            print!("{}", s);
            use std::io::Write;
            std::io::stdout().flush().unwrap();
            let current_tokens = stats.token_count.fetch_add(1, Ordering::SeqCst);

            if current_tokens >= stats.max_tokens {
                stats.done.store(true, Ordering::SeqCst);
                return;
            }
        }
    }

    if sentence_code == genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_END
        || sentence_code == genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_COMPLETE
        || sentence_code == genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_ABORT
    {
        stats.done.store(true, Ordering::SeqCst);
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("--- QNN Runtime Diagnostics ---");
    println!(
        "QNN_SDK_ROOT:      {:?}",
        std::env::var("QNN_SDK_ROOT").unwrap_or_default()
    );
    println!(
        "ADSP_LIBRARY_PATH: {:?}",
        std::env::var("ADSP_LIBRARY_PATH").unwrap_or_default()
    );
    println!(
        "Current Directory: {:?}",
        std::env::current_dir().unwrap_or_default()
    );
    println!("-------------------------------\n");

    if args.verbose {
        println!("Arguments: {:?}", args);
    }

    let config_path = &args.config;
    if !config_path.exists() {
        return Err(anyhow::anyhow!("Config file not found: {:?}", config_path));
    }

    // Change working directory to the config file's directory so relative paths in JSON work
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let config_filename = config_path.file_name().context("Invalid config filename")?;

    if args.verbose {
        println!("Changing directory to: {:?}", config_dir);
    }
    std::env::set_current_dir(config_dir).context("Failed to change working directory")?;

    let config_json = std::fs::read_to_string(config_filename)
        .with_context(|| format!("Failed to read config file: {:?}", config_filename))?;

    let c_config_json = CString::new(config_json).unwrap();
    let mut config_handle: genie::GenieDialogConfig_Handle_t = ptr::null();

    if args.verbose {
        println!("Initializing Genie Dialog Config...");
    }

    let status = unsafe {
        genie::GenieDialogConfig_createFromJson(c_config_json.as_ptr(), &mut config_handle)
    };
    if status != genie::GENIE_STATUS_SUCCESS as i32 {
        return Err(anyhow::anyhow!(
            "GenieDialogConfig_createFromJson failed: 0x{:X}",
            status
        ));
    }

    let mut dialog_handle: genie::GenieDialog_Handle_t = ptr::null();
    if args.verbose {
        println!("Creating Genie Dialog...");
    }
    let status = unsafe { genie::GenieDialog_create(config_handle, &mut dialog_handle) };
    if status != genie::GENIE_STATUS_SUCCESS as i32 {
        unsafe { genie::GenieDialogConfig_free(config_handle) };
        return Err(anyhow::anyhow!("GenieDialog_create failed: 0x{:X}", status));
    }

    let stats = Stats {
        done: AtomicBool::new(false),
        token_count: AtomicUsize::new(0),
        consecutive_whitespace: AtomicUsize::new(0),
        start_time: std::sync::Mutex::new(None),
        max_tokens: args.max_tokens,
    };

    println!("\nPrompt: {}", args.prompt);
    println!("Response:");

    let total_start = Instant::now();
    let mut prompt = args.prompt.clone();
    if !prompt.contains("<|user|>") {
        prompt = format!("<|user|>\n{}<|end|>\n<|assistant|>\n", prompt);
    }

    let c_prompt = CString::new(prompt).unwrap();

    unsafe {
        genie::GenieDialog_query(
            dialog_handle,
            c_prompt.as_ptr(),
            genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_COMPLETE,
            Some(query_callback),
            &stats as *const _ as *const c_void,
        );
    }

    while !stats.done.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    println!();

    let total_duration = total_start.elapsed();
    let first_token_duration = stats
        .start_time
        .lock()
        .unwrap()
        .map(|t| t.duration_since(total_start));
    let tokens = stats.token_count.load(Ordering::SeqCst);

    println!("\n--- Performance Metrics ---");
    println!("Total tokens:    {}", tokens);
    println!("Total time:      {:.2?}", total_duration);
    if let Some(ftd) = first_token_duration {
        println!("TTFT:            {:.2?}", ftd);
        let generation_time = total_duration.saturating_sub(ftd);
        if generation_time.as_secs_f64() > 0.0 {
            let tps = (tokens.saturating_sub(1)) as f64 / generation_time.as_secs_f64();
            println!("TPS (gen):       {:.2} tokens/s", tps);
        }
    }

    unsafe {
        genie::GenieDialog_free(dialog_handle);
        genie::GenieDialogConfig_free(config_handle);
    }

    Ok(())
}
