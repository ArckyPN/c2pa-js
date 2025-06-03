// Copyright 2021 Adobe
// All Rights Reserved.
//
// NOTICE: Adobe permits you to use, modify, and distribute this file in
// accordance with the terms of the Adobe license agreement accompanying
// it.
use crate::error::{Error, Result};
use c2pa::{settings, Reader};

use std::io::Cursor;

pub async fn get_manifest_store_data(
    data: &[u8],
    mime_type: &str,
    settings: Option<&str>,
) -> Result<Reader> {
    if let Some(settings) = settings {
        settings::load_settings_from_str(settings, "json").map_err(Error::from)?;
    }
    let mut data = Cursor::new(data);
    Reader::from_stream_async(mime_type, &mut data)
        .await
        .map_err(Error::from)
}

pub async fn get_manifest_store_data_from_manifest_and_asset_bytes(
    manifest_bytes: &[u8],
    format: &str,
    asset_bytes: &[u8],
    settings: Option<&str>,
) -> Result<Reader> {
    if let Some(settings) = settings {
        settings::load_settings_from_str(settings, "json").map_err(Error::from)?;
    }
    let mut asset = Cursor::new(asset_bytes);
    Reader::from_manifest_data_and_stream_async(manifest_bytes, format, &mut asset)
        .await
        .map_err(Error::from)
}

pub async fn get_manifest_store_data_from_fragment(
    init_bytes: &[u8],
    fragment_bytes: &[u8],
    mime_type: &str,
    settings: Option<&str>,
) -> Result<Reader> {
    if let Some(settings) = settings {
        settings::load_settings_from_str(settings, "json").map_err(Error::from)?;
    }
    let mut init = Cursor::new(init_bytes);
    let mut fragment = Cursor::new(fragment_bytes);
    Reader::from_fragment_async(mime_type, &mut init, &mut fragment)
        .await
        .map_err(Error::from)
}

pub async fn get_manifest_store_from_rolling_hash(
    fragment_bytes: &[u8],
    anchor_point: &Option<Vec<u8>>,
    rolling_hash: &[u8],
    settings: Option<&str>,
) -> Result<Vec<u8>> {
    if let Some(settings) = settings {
        settings::load_settings_from_str(settings, "json").map_err(Error::from)?;
    }
    let mut fragment = Cursor::new(fragment_bytes);
    Ok(Reader::from_rolling_hash_memory_hack(
        &mut fragment,
        rolling_hash,
        anchor_point,
    )?)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    pub async fn test_manifest_store_data() {
        let test_asset = include_bytes!("../../../tools/testing/fixtures/images/CAICAI.jpg");

        let result = get_manifest_store_data(test_asset, "image/jpeg", None).await;
        assert!(result.is_ok());
    }
}
