use crate::adapters::cosmos::AssetRow;
use crate::common::errors::EngineError;

/// Convert an integer base-unit balance to a decimal display string without
/// floating-point rounding.
pub fn normalize_base_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
    let value = raw.trim().parse::<u128>()
        .map_err(|_| EngineError::Validation("invalid Cosmos base amount".into()))?;
    if decimals == 0 { return Ok(value.to_string()); }
    let scale = 10u128.checked_pow(decimals)
        .ok_or_else(|| EngineError::Validation("Cosmos decimal exponent is too large".into()))?;
    let whole = value / scale;
    let fraction = value % scale;
    if fraction == 0 { return Ok(whole.to_string()); }
    let mut fraction_text = format!("{:0width$}", fraction, width = decimals as usize);
    while fraction_text.ends_with('0') { fraction_text.pop(); }
    Ok(format!("{}.{}", whole, fraction_text))
}

pub fn normalize_assets(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
    assets.into_iter().map(|mut asset| {
        if let Some(decimals) = asset.decimals {
            asset.balance = normalize_base_amount(&asset.balance, decimals)?;
        }
        Ok(asset)
    }).collect()
}

pub fn normalize_assets_lossy(assets: Vec<AssetRow>) -> Vec<AssetRow> {
    assets.into_iter().map(|mut asset| {
        if let Some(decimals) = asset.decimals {
            asset.balance = normalize_base_amount(&asset.balance, decimals).unwrap_or_else(|_| "0".into());
        }
        asset
    }).collect()
}
