//! The ABI the hosted port calls. One wasm instance keeps the hull in memory.

use std::alloc::{Layout, alloc, dealloc};
use std::cell::RefCell;

use crate::http::Request;
use crate::hull::Hull;
use crate::json::{self, Value};

thread_local! {
    static HULL: RefCell<Hull> = RefCell::new(Hull::boot_embedded());
}

#[unsafe(no_mangle)]
pub extern "C" fn hull_alloc(len: u32) -> u32 {
    let size = (len as usize).clamp(1, 1_000_000);
    let Ok(layout) = Layout::from_size_align(size, 8) else {
        return 0;
    };
    unsafe {
        let ptr = alloc(layout);
        if ptr.is_null() { 0 } else { ptr as u32 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hull_free(ptr: u32, len: u32) {
    if ptr == 0 {
        return;
    }
    let size = (len as usize).clamp(1, 1_000_000);
    let Ok(layout) = Layout::from_size_align(size, 8) else {
        return;
    };
    unsafe { dealloc(ptr as *mut u8, layout) }
}

#[unsafe(no_mangle)]
pub extern "C" fn hull_handle(ptr: u32, len: u32) -> u32 {
    let input = if ptr == 0 || len == 0 || len > 1_000_000 {
        String::new()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
        let text = String::from_utf8_lossy(slice).into_owned();
        hull_free(ptr, len);
        text
    };
    let encoded = match handle_text(&input) {
        Ok(body) => body,
        Err(detail) => fail_envelope(&detail),
    };
    write_out(encoded.as_bytes())
}

fn handle_text(input: &str) -> Result<String, String> {
    let envelope = json::parse(input).map_err(|error| error.message)?;
    let method = envelope
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET");
    let path = envelope.get("path").and_then(Value::as_str).unwrap_or("/");
    let query = envelope.get("query").and_then(Value::as_str).unwrap_or("");
    let body = envelope
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .as_bytes()
        .to_vec();
    let mut headers = Vec::new();
    if let Some(Value::Object(pairs)) = envelope.get("headers") {
        for (key, value) in pairs {
            if let Some(text) = value.as_str() {
                headers.push((key.clone(), text.to_string()));
            }
        }
    }
    let request =
        Request::from_parts(method, path, query, headers, body).map_err(|detail| detail)?;
    let response = HULL.with(|cell| {
        let Ok(mut hull) = cell.try_borrow_mut() else {
            return crate::http::Response::fail(500, "busy", "The hull is already in a request.");
        };
        hull.dispatch(&request)
    });
    Ok(encode_response(&response))
}

fn encode_response(response: &crate::http::Response) -> String {
    let headers = Value::Array(
        response
            .headers
            .iter()
            .map(|(key, value)| {
                Value::Array(vec![
                    Value::String(key.clone()),
                    Value::String(value.clone()),
                ])
            })
            .collect(),
    );
    let body = String::from_utf8_lossy(&response.body).into_owned();
    Value::Object(vec![
        (
            "status".into(),
            Value::Number(crate::json::Number::Int(i64::from(response.status))),
        ),
        ("headers".into(), headers),
        ("body".into(), Value::String(body)),
    ])
    .to_json()
}

fn fail_envelope(detail: &str) -> String {
    encode_response(&crate::http::Response::fail(400, "bad_request", detail))
}

fn write_out(bytes: &[u8]) -> u32 {
    let total = 4 + bytes.len();
    let ptr = hull_alloc(total as u32);
    if ptr == 0 {
        return 0;
    }
    unsafe {
        let dest = ptr as *mut u8;
        std::ptr::copy_nonoverlapping((bytes.len() as u32).to_le_bytes().as_ptr(), dest, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest.add(4), bytes.len());
    }
    ptr
}
