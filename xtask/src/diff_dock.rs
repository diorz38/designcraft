//! `cargo xtask diff-dock`: checks whether `crates/ui-egui/src/dock.rs` (upstream baseline)
//! and `crates/ui-egui/src/dock_extended.rs` have drifted outside of the known Pubgen hooks.

use std::path::Path;

pub fn run(root: &Path) -> Result<(), String> {
    let dock_path = root.join("crates/ui-egui/src/dock.rs");
    let extended_path = root.join("crates/ui-egui/src/dock_extended.rs");

    if !dock_path.exists() {
        return Err(format!("dock.rs not found at {}", dock_path.display()));
    }
    if !extended_path.exists() {
        return Err(format!("dock_extended.rs not found at {}", extended_path.display()));
    }

    let dock_raw = std::fs::read_to_string(&dock_path).map_err(|e| format!("read dock.rs: {e}"))?;
    let ext_raw = std::fs::read_to_string(&extended_path).map_err(|e| format!("read dock_extended.rs: {e}"))?;

    let dock_lines = clean_lines(&dock_raw, false);
    let ext_lines = clean_lines(&ext_raw, true);

    if dock_lines == ext_lines {
        println!("✓ dock.rs and dock_extended.rs are in sync (Pubgen hooks intact).");
        Ok(())
    } else {
        println!("! Drift detected between dock.rs and dock_extended.rs:\n");
        let mut diff_count = 0;
        let max_len = dock_lines.len().max(ext_lines.len());
        for i in 0..max_len {
            let d_line = dock_lines.get(i).map(String::as_str).unwrap_or("<EOF>");
            let e_line = ext_lines.get(i).map(String::as_str).unwrap_or("<EOF>");
            if d_line != e_line {
                diff_count += 1;
                if diff_count <= 10 {
                    println!("Line {}:", i + 1);
                    println!("  upstream: {d_line}");
                    println!("  extended: {e_line}");
                }
            }
        }
        if diff_count > 10 {
            println!("  ... and {} more differing lines", diff_count - 10);
        }
        println!("\nRun reconciliation: merge upstream changes from dock.rs into dock_extended.rs.");
        Err(format!("dock.rs and dock_extended.rs differ ({diff_count} lines)"))
    }
}

/// Normalizes lines by trimming whitespace and removing Pubgen-specific hooks when `strip_hooks` is true.
fn clean_lines(content: &str, strip_hooks: bool) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| {
            if line.is_empty() {
                return false;
            }
            if strip_hooks {
                if line.contains("pub mod pubgen;")
                    || line.contains("panels/pubgen.rs")
                    || line.contains(r#""pubgen" => pubgen::show(app, ui),"#)
                {
                    return false;
                }
            }
            true
        })
        .map(|line| {
            if strip_hooks {
                line.replace(r#", ("pubgen", "Pubgen", "panel-pages")"#, "")
                    .replace(r#"("pubgen", "Pubgen", "panel-pages"),"#, "")
                    .replace(r#"("pubgen", "Pubgen", "panel-pages")"#, "")
            } else {
                line.to_string()
            }
        })
        .filter(|line| !line.is_empty())
        .collect()
}
