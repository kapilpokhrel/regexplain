use std::error::Error;
use std::fs;
use std::io;

const STATE_FILE: &str = "/tmp/regexplain_state";

pub fn set_state(pattern: &str, text_to_match: &str) -> io::Result<()> {
    let data = format!("{}\x03{}", pattern, text_to_match);
    fs::write(STATE_FILE, data)
}

pub fn restore_state() -> Result<(String, String), Box<dyn Error>> {
    let data = fs::read(STATE_FILE)?;
    let parts: Vec<&[u8]> = data.split(|&x| x == 0x03).collect();
    Ok((str::from_utf8(parts[0])?.into(), str::from_utf8(parts[1])?.into()))
}
