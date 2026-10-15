use crate::common::errors::EngineError;

pub fn validate_existing_signatures(
    transaction: &serde_json::Value,
    approved_list: Option<&Vec<serde_json::Value>>,
    signer_hex_address: &str,
) -> Result<(), EngineError> {
    let signatures = transaction.get("signature").and_then(|v| v.as_array());
    if let Some(signatures) = signatures {
        if signatures.len() > 5 {
            return Err(EngineError::Validation("TRON transaction contains too many signatures".into()));
        }
    }

    if let Some(approved) = approved_list {
        if approved.iter().any(|v| {
            v.as_str()
                .map(|s| s.eq_ignore_ascii_case(signer_hex_address))
                .unwrap_or(false)
        }) {
            return Err(EngineError::Validation(
                "TRON signer has already signed this transaction".into(),
            ));
        }
    }
    Ok(())
}

pub fn append_signature(
    transaction: &mut serde_json::Value,
    signature_hex: String,
) -> Result<(), EngineError> {
    let object = transaction
        .as_object_mut()
        .ok_or_else(|| EngineError::Validation("TRON transaction must be a JSON object".into()))?;
    let signatures = object
        .entry("signature".to_string())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    let signature_array = signatures
        .as_array_mut()
        .ok_or_else(|| EngineError::Validation("transaction.signature must be an array".into()))?;

    if signature_array.len() >= 5 {
        return Err(EngineError::Validation(
            "TRON transaction cannot contain more than 5 signatures".into(),
        ));
    }
    if signature_array.iter().any(|v| {
        v.as_str()
            .map(|s| s.eq_ignore_ascii_case(&signature_hex))
            .unwrap_or(false)
    }) {
        return Err(EngineError::Validation("duplicate TRON signature".into()));
    }
    signature_array.push(serde_json::Value::String(signature_hex));
    Ok(())
}
