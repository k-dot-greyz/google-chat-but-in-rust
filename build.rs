use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=modules");
    println!("cargo:rerun-if-changed=assets");

    let mut out = String::new();
    out.push_str("// Embedded by build.rs. Do not edit.\n");
    write_pairs(&mut out, "EMBEDDED_CARDS", &read_dir("modules/cards"));
    write_pairs(&mut out, "EMBEDDED_PIPES", &read_dir("modules/pipes"));
    out.push_str(&format!(
        "pub static EMBEDDED_POLICY: &str = {};\n",
        rust_string(&fs::read_to_string("modules/policy.json").expect("policy"))
    ));
    out.push_str(&format!(
        "pub static ASSET_CSS: &str = {};\n",
        rust_string(&fs::read_to_string("assets/shell.css").expect("css"))
    ));
    out.push_str(&format!(
        "pub static ASSET_JS: &str = {};\n",
        rust_string(&fs::read_to_string("assets/island.js").expect("js"))
    ));
    out.push_str(&format!(
        "pub static ASSET_SVG: &str = {};\n",
        rust_string(&fs::read_to_string("assets/mark.svg").expect("svg"))
    ));

    let dest = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("embedded.rs");
    fs::write(dest, out).expect("write embedded.rs");
}

fn read_dir(dir: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {dir}: {err}")) {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"));
        files.push((name, text));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    if files.is_empty() {
        panic!("no json in {dir}");
    }
    files
}

fn write_pairs(out: &mut String, name: &str, pairs: &[(String, String)]) {
    out.push_str(&format!("pub static {name}: &[(&str, &str)] = &[\n"));
    for (key, value) in pairs {
        out.push_str(&format!(
            "    ({}, {}),\n",
            rust_string(key),
            rust_string(value)
        ));
    }
    out.push_str("];\n");
}

fn rust_string(value: &str) -> String {
    format!("{value:?}")
}
