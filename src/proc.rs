const ID_ENV_NAME: &str = "__REBOS_PROC_ID_THIS_ENV_VAR_SHOULD_NOT_BE_OVERWRITTEN__";

pub fn init_proc_id() {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let proc_id = format!("rebos_{}_{}", std::process::id(), timestamp);

    std::env::set_var(ID_ENV_NAME, proc_id);
}

pub fn get_proc_id() -> String {
    std::env::var(ID_ENV_NAME).unwrap()
}
