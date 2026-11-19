//! REZ's own share code: `REZ1-` + base64url (no padding) of the crosshair JSON
//! without its id and image.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use super::model::Crosshair;

const PREFIX: &str = "REZ1-";

pub fn encode(c: &Crosshair) -> String {
    let mut shared = c.clone();
    shared.id.clear();
    shared.image = None;
    let json = serde_json::to_vec(&shared).unwrap_or_default();
    format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
}

pub fn decode(code: &str) -> Result<Crosshair, String> {
    let body = code.trim().strip_prefix(PREFIX).ok_or("Code REZ invalide")?;
    let json = URL_SAFE_NO_PAD.decode(body).map_err(|_| "Code REZ invalide")?;
    let mut c: Crosshair = serde_json::from_slice(&json).map_err(|_| "Code REZ invalide")?;
    c.id.clear();
    c.image = None;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::presets::presets;

    #[test]
    fn share_roundtrip_drops_id_and_image() {
        let mut c = presets()[0].clone();
        c.image = Some("x.png".into());
        let code = encode(&c);
        assert!(code.starts_with("REZ1-"));
        let back = decode(&code).unwrap();
        assert_eq!(back.id, "");
        assert_eq!(back.image, None);
        assert_eq!((back.color, back.inner), (c.color, c.inner));
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode("REZ1-!!!").is_err());
        assert!(decode("REZ2-abc").is_err());
    }
}
