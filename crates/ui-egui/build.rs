//! Bake the contributor credits into the binary as static Rust tables. Nothing is read at run time.
//!
//! - contributors/contributors.json: commit stats per GitHub username, with a mark for whose people.toml
//!   entry is verified.
//! - contributors/people.toml: the names people chose for themselves. A name is used only when the
//!   JSON says the entry was verified (`people_entry`).
//!
//! A missing or malformed file gives empty tables (or no names) and a warning.

use std::fmt::Write as _;

use serde_json::Value;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contributors");
    let json = read(&dir.join("contributors.json"), "the About window shows no contributors", |t| {
        serde_json::from_str::<Value>(t).map_err(|e| e.to_string())
    })
    .unwrap_or(Value::Null);
    let people = read(&dir.join("people.toml"), "contributors are shown by username only", |t| t.parse::<toml::Table>().map_err(|e| e.to_string()))
        .unwrap_or_default();
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("credits.rs");
    if let Err(e) = std::fs::write(&out, generate(&json, &people)) {
        println!("cargo::warning=writing {}: {e}", out.display());
    }

    generate_embedded_maps();
}

fn to_title_case(s: &str) -> String {
    let mut out = String::new();
    for word in s.replace('_', " ").split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(&chars.as_str().to_lowercase());
        }
    }
    out
}

fn generate_embedded_maps() {
    let maps_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/pubgen/maps");
    println!("cargo::rerun-if-changed={}", maps_dir.display());

    let mut chart_entries = Vec::new();
    let mut cover_entries = Vec::new();

    if maps_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&maps_dir)
    {
        let mut files: Vec<_> = entries.flatten().filter(|e| e.path().extension().is_some_and(|ext| ext == "geojson")).collect();
        files.sort_by_key(|e| e.file_name());

        for entry in files {
            let path = entry.path();
            println!("cargo::rerun-if-changed={}", path.display());
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let stem = fname.trim_end_matches(".geojson");
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let escaped_text = format!("{text:?}");

            if let Some(prefix) = stem.strip_suffix("_DESA") {
                let id = prefix.to_uppercase();
                let disp = format!("{} (Desa)", to_title_case(prefix));
                chart_entries
                    .push(format!("    EmbeddedMap {{ id: \"{id}\", display_name: \"{disp}\", filename: \"{fname}\", geojson: {escaped_text} }},"));
            } else {
                let id = stem.to_uppercase();
                let disp = format!("{} (Kecamatan)", to_title_case(stem));
                cover_entries
                    .push(format!("    EmbeddedMap {{ id: \"{id}\", display_name: \"{disp}\", filename: \"{fname}\", geojson: {escaped_text} }},"));
            }
        }
    }

    if chart_entries.is_empty() && cover_entries.is_empty() {
        println!(
            "cargo::warning=no GeoJSON stock maps found in {}; choropleth/cover maps will only load from disk next to the executable",
            maps_dir.display()
        );
    }

    let mut out_src = String::new();
    out_src.push_str(
        "pub static EMBEDDED_CHART_MAPS: &[EmbeddedMap] = &[
",
    );
    for entry in chart_entries {
        out_src.push_str(&entry);
        out_src.push('\n');
    }
    out_src.push_str("];\n\n");

    out_src.push_str(
        "pub static EMBEDDED_COVER_MAPS: &[EmbeddedMap] = &[
",
    );
    for entry in cover_entries {
        out_src.push_str(&entry);
        out_src.push('\n');
    }
    out_src.push_str("];\n");

    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("embedded_maps.rs");
    if let Err(e) = std::fs::write(&out, out_src) {
        println!("cargo::warning=writing {}: {e}", out.display());
    }
}

fn read<T>(path: &std::path::Path, fallback: &str, parse: impl Fn(&str) -> Result<T, String>) -> Option<T> {
    println!("cargo::rerun-if-changed={}", path.display());
    let r = std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| parse(&t));
    if let Err(e) = &r {
        println!("cargo::warning={}: {e}; {fallback}", path.display());
    }
    r.ok()
}

/// A non-empty string field of someone's people.toml entry, as a Rust `Option<&str>` literal.
fn name(v: Option<&str>) -> String {
    match v.map(str::trim).filter(|s| !s.is_empty()) {
        Some(x) => format!("Some({x:?})"),
        None => "None".into(),
    }
}

fn s(v: &Value, k: &str) -> String {
    format!("{:?}", v.get(k).and_then(Value::as_str).unwrap_or(""))
}

fn n(v: &Value, k: &str) -> u64 {
    v.get(k).and_then(Value::as_u64).unwrap_or(0)
}

fn rows(json: &Value, k: &str) -> Vec<Value> {
    json.get(k).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn generate(json: &Value, people: &toml::Table) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "pub static TOTAL_COMMITS: u64 = {};", n(json, "commits"));
    let _ = writeln!(o, "pub static CONTRIBUTORS: &[Contributor] = &[");
    for c in rows(json, "contributors") {
        let login = c.get("login").and_then(Value::as_str).unwrap_or("");
        if login.is_empty() {
            continue;
        }
        // Names only from a verified, self-chosen people.toml entry.
        let entry = people.get("people").and_then(|p| p.get(login)).filter(|_| c.get("people_entry").and_then(Value::as_bool) == Some(true));
        let field = |k: &str| entry.and_then(|e| e.get(k)).and_then(toml::Value::as_str);
        let synced = entry.and_then(|e| e.get("sync_github_name")).and_then(toml::Value::as_bool) == Some(true);
        let display = field("display_name")
            .filter(|s| !s.trim().is_empty())
            .or_else(|| if synced { c.get("github_name").and_then(Value::as_str) } else { None });
        let _ = writeln!(
            o,
            "    Contributor {{ login: {login:?}, display_name: {}, real_name: {}, prs: {}, commits: {}, lines_added: {}, lines_deleted: {}, binary_added: {}, binary_deleted: {}, first_commit: {}, last_commit: {} }},",
            name(display),
            name(field("real_name")),
            n(&c, "prs"),
            n(&c, "commits"),
            n(&c, "lines_added"),
            n(&c, "lines_deleted"),
            n(&c, "binary_added"),
            n(&c, "binary_deleted"),
            s(&c, "first_commit"),
            s(&c, "last_commit"),
        );
    }
    let _ = writeln!(o, "];");
    let _ = writeln!(o, "pub static MODELS: &[Model] = &[");
    for m in rows(json, "models") {
        let _ = writeln!(
            o,
            "    Model {{ company: {}, model: {}, version: {}, commits: {}, lines_added: {}, lines_deleted: {} }},",
            s(&m, "company"),
            s(&m, "model"),
            s(&m, "version"),
            n(&m, "commits"),
            n(&m, "lines_added"),
            n(&m, "lines_deleted"),
        );
    }
    let _ = writeln!(o, "];");
    o
}
