use crate::common::{genie_callback, Stats};
use crate::genie;
use anyhow::{Context, Result};
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::Path;
use std::ptr;

pub struct GenieEngine {
    config_handle: genie::GenieDialogConfig_Handle_t,
    dialog_handle: genie::GenieDialog_Handle_t,
}

impl GenieEngine {
    pub fn new(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            return Err(anyhow::anyhow!("Config file not found: {:?}", config_path));
        }
        let config_dir = config_path.parent().unwrap_or(Path::new("."));
        let config_filename = config_path.file_name().context("Invalid config filename")?;
        std::env::set_current_dir(config_dir).context("Failed to change working directory")?;
        let config_json = std::fs::read_to_string(config_filename)?;
        let c_config_json = CString::new(config_json).unwrap();
        let mut config_handle: genie::GenieDialogConfig_Handle_t = ptr::null();
        let status = unsafe {
            genie::GenieDialogConfig_createFromJson(c_config_json.as_ptr(), &mut config_handle)
        };
        if status != genie::GENIE_STATUS_SUCCESS as i32 {
            return Err(anyhow::anyhow!("Config creation failed: 0x{:X}", status));
        }
        let mut dialog_handle: genie::GenieDialog_Handle_t = ptr::null();
        let status = unsafe { genie::GenieDialog_create(config_handle, &mut dialog_handle) };
        if status != genie::GENIE_STATUS_SUCCESS as i32 {
            unsafe { genie::GenieDialogConfig_free(config_handle) };
            return Err(anyhow::anyhow!("Genie creation failed: 0x{:X}", status));
        }
        Ok(Self {
            config_handle,
            dialog_handle,
        })
    }

    pub fn query(&self, prompt: &str, stats: &Stats) -> Result<()> {
        let mut formatted_prompt = prompt.to_string();
        if !formatted_prompt.contains("<|user|>") {
            formatted_prompt = format!("<|user|>\n{}<|end|>\n<|assistant|>\n", formatted_prompt);
        }
        let c_prompt = CString::new(formatted_prompt).unwrap();
        unsafe {
            genie::GenieDialog_query(
                self.dialog_handle,
                c_prompt.as_ptr(),
                genie::GenieDialog_SentenceCode_t_GENIE_DIALOG_SENTENCE_COMPLETE,
                Some(genie_callback),
                stats as *const _ as *const c_void,
            );
        }
        Ok(())
    }
}

impl Drop for GenieEngine {
    fn drop(&mut self) {
        unsafe {
            genie::GenieDialog_free(self.dialog_handle);
            genie::GenieDialogConfig_free(self.config_handle);
        }
    }
}
