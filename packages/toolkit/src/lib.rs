// Copyright 2021 Adobe
// All Rights Reserved.
//
// NOTICE: Adobe permits you to use, modify, and distribute this file in
// accordance with the terms of the Adobe license agreement accompanying
// it.

// See https://github.com/rustwasm/wasm-bindgen/issues/2774
#![allow(clippy::unused_unit)]
use c2pa::identity::{
    claim_aggregation::IcaSignatureVerifier, x509::X509SignatureVerifier, BuiltInSignatureVerifier,
    IdentityAssertion,
};
use log::Level;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use std::panic;
use wasm_bindgen::prelude::*;

mod error;
mod manifest_store;
mod util;

use error::Error;
use js_sys::Error as JsSysError;
use js_sys::Reflect;
use manifest_store::{
    get_manifest_store_data, get_manifest_store_data_from_fragment,
    get_manifest_store_data_from_manifest_and_asset_bytes, get_manifest_store_from_rolling_hash,
};
use util::log_time;

#[wasm_bindgen(typescript_custom_section)]
pub const TS_APPEND_CONTENT: &'static str = r#"
import { AssetReport } from './types'

export * from './types';

export function getManifestStoreFromArrayBuffer(
    buf: ArrayBuffer,
    mimeType: string,
    settings?: string
): Promise<AssetReport>;

export function getManifestStoreFromManifestAndAsset(
    manifestBuffer: ArrayBuffer,
    assetBuffer: ArrayBuffer,
    mimeType: string,
    settings?: string
): Promise<AssetReport>;

export function getManifestStoreFromFragment(
    initBuffer: ArrayBuffer,
    fragmentBuffer: ArrayBuffer,
    mimeType: string,
    settings?: string
): Promise<AssetReport>;

export function getManifestStoreFromRollingHash(
    fragmentBuffer: ArrayBuffer,
    rollingHash: ArrayBuffer,
    anchorPoint?: ArrayBuffer,
    settings?: string
): Promise<Uint8Array>;
"#;

#[wasm_bindgen(start)]
pub fn run() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(Level::Info).unwrap();
}

/// Creates a JavaScript Error with additional error info
///
/// We can't use wasm-bindgen's `JsError` since it only allows you to set a single message string
#[allow(unused_must_use)]
fn as_js_error(err: Error) -> JsSysError {
    let js_err = JsSysError::new(&err.to_string());
    js_err.set_name(&format!("{:?}", err));

    if let Error::C2pa(c2pa::Error::RemoteManifestUrl(url)) = err {
        js_err.set_name("Toolkit(RemoteManifestUrl)");
        Reflect::set(&js_err, &"url".into(), &url.into());
    }

    js_err
}

fn serde_error_as_js_error(err: serde_json::Error) -> JsSysError {
    let js_err = JsSysError::new(&err.to_string());
    js_err.set_name("Toolkit(SerdeJsonError)");
    js_err
}

#[derive(Serialize)]
struct AssetReport {
    manifest_store: c2pa::Reader,
    cawg_json: String,
}

#[wasm_bindgen(js_name = getManifestStoreFromArrayBuffer, skip_typescript)]
pub async fn get_manifest_store_from_array_buffer(
    buf: JsValue,
    mime_type: String,
    settings: Option<String>,
) -> Result<JsValue, JsSysError> {
    log_time("get_manifest_store_from_array_buffer::start");
    let asset: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(buf)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;
    log_time("get_manifest_store_from_array_buffer::from_bytes");
    let result = get_manifest_store_data(&asset, &mime_type, settings.as_deref())
        .await
        .map_err(as_js_error)?;

    let js_value = get_serialized_report_with_cawg_from_manifest_store(result).await?;

    Ok(js_value)
}

#[wasm_bindgen(js_name = getManifestStoreFromManifestAndAsset, skip_typescript)]
pub async fn get_manifest_store_from_manifest_and_asset(
    manifest_buffer: JsValue,
    asset_buffer: JsValue,
    mime_type: String,
    settings: Option<String>,
) -> Result<JsValue, JsSysError> {
    log_time("get_manifest_store_data_from_manifest_and_asset::start");
    let manifest: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(manifest_buffer)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;

    let asset: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(asset_buffer)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;

    log_time("get_manifest_store_data_from_manifest_and_asset::from_bytes");
    let result = get_manifest_store_data_from_manifest_and_asset_bytes(
        &manifest,
        &mime_type,
        &asset,
        settings.as_deref(),
    )
    .await
    .map_err(as_js_error)?;

    let js_value = get_serialized_report_with_cawg_from_manifest_store(result).await?;

    Ok(js_value)
}

async fn get_serialized_report_with_cawg_from_manifest_store(
    manifest_store: c2pa::Reader,
) -> Result<JsValue, JsSysError> {
    let verifier = BuiltInSignatureVerifier {
        ica_verifier: IcaSignatureVerifier {},
        x509_verifier: X509SignatureVerifier {},
    };
    let ia_summary = IdentityAssertion::summarize_from_reader(
        &manifest_store,
        &mut Default::default(),
        &verifier,
    )
    .await;

    let ia_json = serde_json::to_string(&ia_summary).map_err(serde_error_as_js_error)?;

    let report = AssetReport {
        manifest_store,
        cawg_json: ia_json,
    };

    let serializer = Serializer::new().serialize_maps_as_objects(true);
    let js_value = report
        .serialize(&serializer)
        .map_err(|_err| Error::JavaScriptConversion)
        .map_err(as_js_error)?;

    Ok(js_value)
}

#[wasm_bindgen(js_name = getManifestStoreFromFragment, skip_typescript)]
pub async fn get_manifest_store_from_fragment(
    init_buf: JsValue,
    fragment_buf: JsValue,
    mime_type: String,
    settings: Option<String>,
) -> Result<JsValue, JsSysError> {
    let init: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(init_buf)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;
    let fragment: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(fragment_buf)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;
    let result =
        get_manifest_store_data_from_fragment(&init, &fragment, &mime_type, settings.as_deref())
            .await
            .map_err(as_js_error)?;

    let js_value = get_serialized_report_with_cawg_from_manifest_store(result).await?;

    Ok(js_value)
}

#[wasm_bindgen(js_name = getManifestStoreFromRollingHash, skip_typescript)]
pub async fn get_manifest_from_rolling_hash(
    fragment_buf: JsValue,
    rolling_hash: JsValue,
    anchor_point: JsValue,
    settings: Option<String>,
) -> Result<JsValue, JsSysError> {
    let fragment: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(fragment_buf)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;
    let anchor_point: Option<serde_bytes::ByteBuf> = if anchor_point.is_truthy() {
        serde_wasm_bindgen::from_value(anchor_point)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?
    } else {
        None
    };
    let rolling_hash: serde_bytes::ByteBuf = serde_wasm_bindgen::from_value(rolling_hash)
        .map_err(Error::SerdeInput)
        .map_err(as_js_error)?;
    let result = get_manifest_store_from_rolling_hash(
        &fragment,
        &anchor_point.map(|ap| ap.to_vec()),
        &rolling_hash,
        settings.as_deref(),
    )
    .await
    .map_err(as_js_error)?;

    Ok(JsValue::from(result))
}
