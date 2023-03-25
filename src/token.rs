use anyhow::{anyhow, Result};

pub fn get_payload(token: &str) -> Result<serde_json::Value> {
    let str = token.split('.').nth(1).unwrap();
    let decoded = base64::decode_config(str, base64::URL_SAFE_NO_PAD)?;
    let json = String::from_utf8(decoded)?;
    let value: serde_json::Value = serde_json::from_str(&json)?;
    Ok(value)
}

pub fn get_payload_field(token: &str, field: &str) -> Result<String> {
    let value = get_payload(token)?;
    let field = value.get(field).ok_or(anyhow!("invalid token"))?;
    Ok(field.as_str().unwrap().to_string())
}
