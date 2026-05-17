use clap::Parser;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "phi_3_5_mini_instruct-genie-w4a16-qualcomm/genie_config.json"
    )]
    pub config: PathBuf,
    #[arg(
        short,
        long,
        default_value = "Explain quantum physics in one sentence."
    )]
    pub prompt: String,
    #[arg(short, long, default_value = "512")]
    pub max_tokens: usize,
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(long)]
    pub ort: bool,
    #[arg(short, long)]
    pub model: Option<PathBuf>,
    #[arg(long, default_value = "43")]
    pub soc_model: String,
}

pub struct Stats {
    done: AtomicBool,
    token_count: AtomicUsize,
    consecutive_whitespace: AtomicUsize,
    start_time: std::sync::Mutex<Option<Instant>>,
    max_tokens: usize,
}

impl Stats {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            done: AtomicBool::new(false),
            token_count: AtomicUsize::new(0),
            consecutive_whitespace: AtomicUsize::new(0),
            start_time: std::sync::Mutex::new(None),
            max_tokens,
        }
    }
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
    pub fn get_token_count(&self) -> usize {
        self.token_count.load(Ordering::SeqCst)
    }
}

pub extern "C" fn genie_callback(
    response: *const c_char,
    sentence_code: crate::genie::GenieDialog_SentenceCode_t,
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
            let is_stop = s.contains("<|end|>")
                || s.contains("<|user|>")
                || s.contains("<|endoftext|>")
                || s.contains("</s>");
            if is_stop {
                stats.done.store(true, Ordering::SeqCst);
                return;
            }
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
            let _ = std::io::stdout().flush();
            let current = stats.token_count.fetch_add(1, Ordering::SeqCst);
            if current >= stats.max_tokens {
                stats.done.store(true, Ordering::SeqCst);
            }
        }
    }
    if sentence_code == crate::genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_END
        || sentence_code == crate::genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_COMPLETE
        || sentence_code == crate::genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_ABORT
    {
        stats.done.store(true, Ordering::SeqCst);
    }
}
