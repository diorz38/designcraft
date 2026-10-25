//! The table generation engine: CSV matrix -> native styled table items.

use super::*;

pub(crate) fn execute_insert_table(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    execute_insert_table_matrix(app, st, None, None)
}

/// One table cell of the tab-delimited text `table.convertFromText` reads. A paragraph break would
/// split the cell into extra rows and a tab into extra columns, so an Excel line break becomes the
/// story's forced line break (U+2028), which the table cell keeps on multiple lines.
pub(crate) fn table_cell_text(cell: &str) -> String {
    let normalized = cell.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(normalized.len());
    for (i, line) in normalized.split('\n').enumerate() {
        if i > 0 {
            out.push(designcraft_doc::FORCED_LINE_BREAK);
        }
        out.push_str(line.replace('\t', " ").trim());
    }
    out
}

#[allow(clippy::needless_range_loop)]
pub(crate) fn execute_insert_table_matrix(
    app: &mut DesignApp,
    st: &PubgenState,
    matrix_override: Option<&[Vec<String>]>,
    target_page_override: Option<usize>,
) -> Result<String, String> {
    let source_matrix = matrix_override.unwrap_or(&st.card2_matrix);
    if source_matrix.is_empty() {
        return Err("No tabular data to insert".into());
    }

    // Detect and extract any metadata rows (e.g. theme: #FFCB56 - #FFE9A9 or color: ...)
    let mut detected_theme = None;
    let mut matrix = source_matrix.to_vec();
    matrix.retain(|row| {
        let first = row.first().map(|s| s.trim()).unwrap_or("");
        let first_lower = first.to_lowercase();
        let is_theme_key = excel::THEME_KEYS.iter().any(|k| first_lower.starts_with(k));
        if is_theme_key {
            // Find theme value in second cell or after ':' in first cell
            let val = if let Some(pos) = first.find(':') {
                first[pos + 1..].trim().to_string()
            } else {
                row.get(1).map(|s| s.trim().to_string()).unwrap_or_default()
            };
            if !val.is_empty() {
                detected_theme = parse_table_color_theme(&val);
            }
            return false;
        }
        true
    });

    let active_theme: TableColorTheme =
        st.card2_theme_override.as_deref().and_then(parse_table_color_theme).or(detected_theme).unwrap_or_else(|| {
            TABLE_COLOR_THEMES
                .get(st.card2_color_theme)
                .map(|(_, hf, rn)| TableColorTheme::new(*hf, *rn))
                .unwrap_or_else(|| TableColorTheme::new("#FFCB56", "#FFE9A9"))
        });

    // Detect and extract structured table note / source rows (Catatan, Note, Sumber, Source, Keterangan, Ket, Ket.)
    let mut table_notes: Vec<excel::TableNoteItem> = Vec::new();
    matrix.retain(|row| {
        if let Some(item) = excel::parse_table_note_row(row) {
            table_notes.push(item);
            false
        } else {
            true
        }
    });

    // Detect and extract trailing footnote/source row(s) (where only first col has data and rest of cols are empty)
    let mut footnotes = Vec::new();
    let cols_count = matrix.iter().map(Vec::len).max().unwrap_or(0);
    if cols_count > 1 {
        while matrix.len() > 1 {
            let Some(last) = matrix.last() else { break };
            let first = last.first().map(|s| s.trim()).unwrap_or("");
            let rest_empty = last.iter().skip(1).all(|c| c.trim().is_empty());
            if !first.is_empty() && rest_empty {
                footnotes.insert(0, first.to_string());
                matrix.pop();
            } else {
                break;
            }
        }
    }
    let footnote_text = if footnotes.is_empty() { None } else { Some(footnotes.join("\n")) };

    let rows_count = matrix.len();

    // Auto-detect merged header columns (split by "--", capped at max 3 stacks)
    let col_levels: Vec<Vec<String>> = if st.card2_first_row_header {
        matrix
            .first()
            .map(|row| {
                (0..cols_count)
                    .map(|c| {
                        let val = row.get(c).map(|s| s.trim()).unwrap_or("");
                        if val.contains("--") { val.splitn(3, "--").map(|s| s.trim().to_string()).collect() } else { vec![val.to_string()] }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let merge_count = col_levels.iter().filter(|lvls| lvls.len() > 1).count();
    let num_header_rows = col_levels.iter().map(Vec::len).max().unwrap_or(1).clamp(1, 3);
    let has_merged_headers = st.card2_first_row_header && merge_count > 1 && num_header_rows > 1;

    let total_rows = if has_merged_headers {
        num_header_rows + (rows_count.saturating_sub(1)) + if st.card2_col_numbers { 1 } else { 0 }
    } else if st.card2_col_numbers {
        rows_count + 1
    } else {
        rows_count
    };

    // Format tabular data with tab separators and newlines for convertFromText
    let mut table_text = String::new();

    if has_merged_headers {
        // Build header grid: num_header_rows × cols_count
        let mut header_grid = vec![vec![String::new(); cols_count]; num_header_rows];

        for c in 0..cols_count {
            let lvls = col_levels.get(c).map(|v| v.as_slice()).unwrap_or(&[]);
            let k = lvls.len();
            for r in 0..num_header_rows {
                if r < k.saturating_sub(1) {
                    // Ancestor group level: emit text only at the start of the group
                    let is_start = c == 0 || (0..=r).any(|l| col_levels.get(c.saturating_sub(1)).and_then(|prev| prev.get(l)) != lvls.get(l));
                    if is_start && let Some(txt) = lvls.get(r) {
                        header_grid[r][c] = txt.trim().to_string();
                    }
                } else if r == k.saturating_sub(1) {
                    // Leaf level
                    if let Some(txt) = lvls.get(r) {
                        header_grid[r][c] = txt.trim().to_string();
                    }
                }
                // When r > k.saturating_sub(1), remains empty String
            }
        }

        // Push header rows
        for (r_idx, row) in header_grid.iter().enumerate() {
            if r_idx > 0 {
                table_text.push('\n');
            }
            for (c_idx, cell) in row.iter().enumerate() {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&table_cell_text(cell));
            }
        }

        // If column numbers row is enabled with header, insert immediately after header rows
        if st.card2_col_numbers {
            table_text.push('\n');
            for c_idx in 0..cols_count {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&format!("({})", c_idx + 1));
            }
        }

        // Data rows (skip row 0)
        for row in matrix.iter().skip(1) {
            table_text.push('\n');
            for (c_idx, cell) in row.iter().enumerate() {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&table_cell_text(cell));
            }
        }
    } else {
        // If column numbers row is enabled without header, insert at top
        if st.card2_col_numbers && !st.card2_first_row_header {
            for c_idx in 0..cols_count {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&format!("({})", c_idx + 1));
            }
        }

        for (r_idx, row) in matrix.iter().enumerate() {
            if !table_text.is_empty() {
                table_text.push('\n');
            }
            for (c_idx, cell) in row.iter().enumerate() {
                if c_idx > 0 {
                    table_text.push('\t');
                }
                table_text.push_str(&table_cell_text(cell));
            }
            // If column numbers row is enabled with header, insert immediately after header row (r_idx == 0)
            if st.card2_col_numbers && st.card2_first_row_header && r_idx == 0 {
                table_text.push('\n');
                for c_idx in 0..cols_count {
                    if c_idx > 0 {
                        table_text.push('\t');
                    }
                    table_text.push_str(&format!("({})", c_idx + 1));
                }
            }
        }
    }

    // Step 1: Ensure we have a text target
    let active = app.session.active().ok_or("No active document")?;
    let has_caret = active.selection.text.is_some();

    let cur_page = target_page_override.unwrap_or_else(|| target_page_index(app));
    let effective_placement = if target_page_override.is_some() { 2 } else { st.card2_placement };

    if effective_placement == 0 && !has_caret {
        return Err("No active text insertion point. Select a text frame or choose 'New frame' / 'At page'.".into());
    }

    let (story_id, target_page_info, created_frame_info, fn_target) = match effective_placement {
        0 => {
            // At cursor
            let story = active.selection.text.map(|t| t.story.0).ok_or("No active story")?;
            let mut fn_target = None;
            if let Some(fid) = active.selection.text.and_then(|t| t.frame)
                && let Some(loc) = active.doc.find(fid)
                && let designcraft_doc::SpreadRef::Doc(si) = loc.spread
                && let Some(spread) = active.doc.spreads.get(si)
                && let Some(item) = spread.items.iter().find(|it| it.id == fid)
            {
                let b = item.bounds();
                let fn_y0 = b.y1 + 6.0;
                let fn_y1 = fn_y0 + 24.0;
                fn_target = Some((si, [b.x0, fn_y0, b.x1, fn_y1]));
            }
            (story, None, None, fn_target)
        }
        2 => {
            // At page: place below Generated Page Title on active / selected page
            let (spread_idx, page_in_spread) = active.doc.page_loc(cur_page).unwrap_or((0, 0));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Spread missing")?;
            let page = spread.pages.get(page_in_spread).ok_or("Page missing")?;
            let margin = page.margin_rect();
            let page_r = page.bounds();

            // Find bottom-most edge of generated page title / header items on this page
            let mut title_bottom = margin.y0;
            for it in &spread.items {
                let b = it.bounds();
                // Check if item lies on this page horizontally and in header region vertically
                if b.x0 < page_r.x1 && b.x1 > page_r.x0 && b.y0 < margin.y0 + 260.0 {
                    // Ignore text frames that already contain tables
                    let is_table = it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|st| !st.tables.is_empty());
                    if !is_table && b.y1 > title_bottom {
                        title_bottom = b.y1;
                    }
                }
            }

            let has_footnote = footnote_text.is_some() || !table_notes.is_empty();
            let note_reserve_h = if !table_notes.is_empty() {
                (table_notes.len() as f64 * (if st.bilingual_output { 32.0 } else { 16.0 }) + 20.0).clamp(32.0, 160.0)
            } else if footnote_text.is_some() {
                32.0
            } else {
                0.0
            };
            let gap_below_title = 12.0;
            let table_y0 = if title_bottom > margin.y0 { title_bottom + gap_below_title } else { margin.y0 };
            let available_h = (margin.y1 - table_y0).max(60.0);
            let max_table_h = if has_footnote { (available_h - note_reserve_h).max(40.0) } else { available_h };
            let frame_h = max_table_h;

            let res = app.run(
                "frame.create",
                json!({
                    "spread": spread_idx,
                    "rect": [margin.x0, table_y0, margin.x1, table_y0 + frame_h],
                    "content": "text",
                    "caret": true
                }),
            )?;
            let sid = res["story"].as_u64().ok_or("Failed to get story ID")?;
            let fid = res["id"].as_u64().map(designcraft_doc::ItemId);
            let created_info = fid.map(|id| (id, spread_idx, margin.x0, margin.x1, table_y0, max_table_h, margin.y1));
            (sid, Some(cur_page), created_info, None)
        }
        _ => {
            // 1 or default: New frame on active / selected page
            let cur_page = target_page_index(app);
            let (spread_idx, page_in_spread) = active.doc.page_loc(cur_page).unwrap_or((0, 0));
            let spread = active.doc.spreads.get(spread_idx).ok_or("Spread missing")?;
            let page = spread.pages.get(page_in_spread).ok_or("Page missing")?;
            let margin = page.margin_rect();
            let has_footnote = footnote_text.is_some() || !table_notes.is_empty();
            let note_reserve_h = if !table_notes.is_empty() {
                (table_notes.len() as f64 * (if st.bilingual_output { 32.0 } else { 16.0 }) + 20.0).clamp(32.0, 160.0)
            } else if footnote_text.is_some() {
                32.0
            } else {
                0.0
            };
            let max_table_h = if has_footnote { (margin.height() - note_reserve_h).max(40.0) } else { margin.height() };
            let frame_h = max_table_h;

            let res = app.run(
                "frame.create",
                json!({
                    "spread": spread_idx,
                    "rect": [margin.x0, margin.y0, margin.x1, margin.y0 + frame_h],
                    "content": "text",
                    "caret": true
                }),
            )?;
            let sid = res["story"].as_u64().ok_or("Failed to get story ID")?;
            let fid = res["id"].as_u64().map(designcraft_doc::ItemId);
            let created_info = fid.map(|id| (id, spread_idx, margin.x0, margin.x1, margin.y0, max_table_h, margin.y1));
            (sid, Some(cur_page), created_info, None)
        }
    };

    // Step 2: Insert the tab-delimited text into story
    app.run("text.insert", json!({"text": table_text}))?;

    // Step 3: Select all paragraphs in the inserted story and convert to table
    let text_len = {
        let act = app.session.active().ok_or("No active document")?;
        let story = act.doc.story(designcraft_doc::StoryId(story_id)).ok_or("Story not found")?;
        story.text.len()
    };
    app.run("text.select", json!({"story": story_id, "anchor": 0, "focus": text_len}))?;

    // Step 4: Convert Text to Table
    let res = app.run("table.convertFromText", json!({"columnSeparator": "tab"}))?;

    let header_rows_count = if has_merged_headers {
        num_header_rows
    } else if st.card2_first_row_header && rows_count > 1 {
        1
    } else {
        0
    };
    let footer_rows_count = if st.card2_table_footer && rows_count > header_rows_count + 1 { 1 } else { 0 };
    if header_rows_count > 0 || footer_rows_count > 0 {
        let mut opts = json!({});
        if header_rows_count > 0 {
            opts["headerRows"] = json!(header_rows_count);
        }
        if footer_rows_count > 0 {
            opts["footerRows"] = json!(footer_rows_count);
        }
        let _ = app.run("table.options", opts);
    }

    // Step 5: Table Style (if not Horizontal Rules, apply via command first)
    let style_names = ["Basic Grid", "Corporate Navy", "Minimalist", "Horizontal Rules"];
    if st.card2_style != 3
        && let Some(&style_name) = style_names.get(st.card2_style)
    {
        let border_weight = if st.card2_style == 1 { 2.0 } else { 0.5 };
        let _ = app.run(
            "style.table.create",
            json!({
                "name": style_name,
                "border": {"weight": border_weight}
            }),
        );
        let _ = app.run("style.table.apply", json!({"name": style_name}));
    }

    let table_id = res["table"]
        .as_u64()
        .or_else(|| {
            let act = app.session.active()?;
            let story = act.doc.story(designcraft_doc::StoryId(story_id))?;
            story.tables.keys().copied().next()
        })
        .ok_or("Failed to find table ID")?;

    app.session
        .edit(|doc, _| {
            let (hf_swatch, rn_swatch) = if !active_theme.is_none() {
                (ensure_swatch(doc, &active_theme.header_footer), ensure_swatch(doc, &active_theme.row_number))
            } else {
                (String::new(), String::new())
            };

            if st.card2_style == 3 && !doc.styles.table.iter().any(|s| s.name == "Horizontal Rules") {
                let ts = designcraft_doc::TableStyle {
                    name: "Horizontal Rules".to_string(),
                    border: Some(designcraft_doc::CellStroke::none()),
                    ..designcraft_doc::TableStyle::default()
                };
                doc.styles_mut().table.push(ts);
            }

            let story = doc
                .story_mut(designcraft_doc::StoryId(story_id))
                .ok_or_else(|| designcraft_engine::EngineError::Other("Story not found".to_string()))?;
            let table = story.table_mut(table_id).ok_or_else(|| designcraft_engine::EngineError::Other("Table not found".to_string()))?;

            if has_merged_headers {
                // 1. Vertical merges: for columns where leaf row is before (num_header_rows - 1)
                for (c, lvls) in col_levels.iter().enumerate() {
                    let k = lvls.len();
                    if k < num_header_rows {
                        let _ = table.merge(designcraft_doc::CellRange { r0: k.saturating_sub(1), c0: c, r1: num_header_rows - 1, c1: c });
                    }
                }

                // 2. Horizontal merges: consecutive columns sharing common ancestor prefix at level r
                for r in 0..num_header_rows.saturating_sub(1) {
                    let mut c = 0;
                    while c < cols_count {
                        if col_levels.get(c).map(|lvls| lvls.len() > r + 1).unwrap_or(false) {
                            let start = c;
                            let prefix = &col_levels[c][0..=r];
                            while c + 1 < cols_count
                                && col_levels.get(c + 1).map(|lvls| lvls.len() > r + 1).unwrap_or(false)
                                && &col_levels[c + 1][0..=r] == prefix
                            {
                                c += 1;
                            }
                            let end = c;
                            if end > start {
                                let _ = table.merge(designcraft_doc::CellRange { r0: r, c0: start, r1: r, c1: end });
                            }
                        }
                        c += 1;
                    }
                }
            }

            let nr = table.nrows();
            let nc = table.ncols();

            let header_rows = if has_merged_headers {
                num_header_rows
            } else if st.card2_first_row_header {
                1
            } else {
                0
            };

            let mut data_start = if header_rows > 0 {
                if st.card2_col_numbers { header_rows + 1 } else { header_rows }
            } else if st.card2_col_numbers {
                1
            } else {
                0
            };

            // Also check if row at data_start is an explicit row of column numbers e.g. (1), (2)...
            if data_start < nr && !st.card2_col_numbers && nc > 0 {
                let is_explicit_col_num = (0..nc).all(|c| {
                    table
                        .cell(data_start, c)
                        .map(|cell| {
                            let t = cell.text.text.trim();
                            t == format!("({})", c + 1) || t == format!("{}", c + 1)
                        })
                        .unwrap_or(false)
                });
                if is_explicit_col_num {
                    data_start += 1;
                }
            }

            // 1. All table header rows and row number / column numbers row are center-aligned
            for r in 0..data_start.min(nr) {
                for col_idx in 0..nc {
                    if let Some(cell) = table.cell_mut(r, col_idx) {
                        cell.vj = designcraft_doc::VerticalJustification::Center;
                        for p in &mut cell.text.paras {
                            p.para.align = Some(designcraft_doc::Align::Center);
                        }
                        cell.text.rev += 1;
                    }
                }
            }

            // 2. For data rows, except col 1 (index 0), check if majority cell contains numeric or "-" -> set column to align center
            for c in 1..nc {
                let mut total_non_empty = 0usize;
                let mut numeric_or_dash_count = 0usize;
                for r in data_start..nr {
                    if let Some(cell) = table.cell(r, c) {
                        let txt = cell.text.text.trim();
                        if !txt.is_empty() {
                            total_non_empty += 1;
                            if is_numeric_or_dash(txt) {
                                numeric_or_dash_count += 1;
                            }
                        }
                    }
                }
                if total_non_empty > 0 && numeric_or_dash_count * 2 >= total_non_empty {
                    for r in data_start..nr {
                        if let Some(cell) = table.cell_mut(r, c) {
                            cell.vj = designcraft_doc::VerticalJustification::Center;
                            for p in &mut cell.text.paras {
                                p.para.align = Some(designcraft_doc::Align::Center);
                            }
                            cell.text.rev += 1;
                        }
                    }
                }
            }

            // 3. Bilingual table content styling: all English text after " / " or on separate lines is Italic
            for r in 0..nr {
                for c in 0..nc {
                    if let Some(cell) = table.cell_mut(r, c) {
                        let text = cell.text.text.clone();
                        // Case A: Separated by " / "
                        if let Some(pos) = text.find(" / ") {
                            let en_byte_start = pos + " / ".len();
                            if en_byte_start < text.len() {
                                cell.text.format_chars(en_byte_start..text.len(), |f| {
                                    let is_bold = f.over.font_style.as_deref().is_some_and(|s| s.to_lowercase().contains("bold"));
                                    f.over.font_style = Some(if is_bold { "Bold Italic".to_string() } else { "Italic".to_string() });
                                });
                            }
                        } else if let Some(pos) = text.find(designcraft_doc::FORCED_LINE_BREAK).or_else(|| text.find('\n')) {
                            // Case B: Separated by line break (Indonesian line 1, English line 2+)
                            let break_len = if text[pos..].starts_with(designcraft_doc::FORCED_LINE_BREAK) {
                                designcraft_doc::FORCED_LINE_BREAK.len_utf8()
                            } else {
                                1
                            };
                            let en_byte_start = pos + break_len;
                            if en_byte_start < text.len() {
                                cell.text.format_chars(en_byte_start..text.len(), |f| {
                                    let is_bold = f.over.font_style.as_deref().is_some_and(|s| s.to_lowercase().contains("bold"));
                                    f.over.font_style = Some(if is_bold { "Bold Italic".to_string() } else { "Italic".to_string() });
                                });
                            }
                        }
                    }
                }
            }

            if st.card2_style == 3 {
                table.style = "Horizontal Rules".to_string();
                table.options.border = designcraft_doc::CellStroke::none();
                table.options.borders = Default::default();
                table.options.row_strokes = None;
                table.options.column_strokes = None;
                table.options.alt_rows = None;
                table.options.alt_cols = None;

                // Clear all cell strokes and set border_overrides so no default or table borders show
                for cell in &mut table.cells {
                    cell.strokes = std::array::from_fn(|_| designcraft_doc::CellStroke::none());
                    cell.border_overrides = [true; 4];
                    cell.stroke_defined = [true; 4];
                    cell.stroke_priorities = [0; 4];
                }

                if nr > 0 && nc > 0 {
                    let rule_thick = designcraft_doc::CellStroke {
                        weight: 1.0,
                        color: designcraft_color::swatch::BLACK.into(),
                        tint: 1.0,
                        kind: designcraft_doc::StrokeType::Solid,
                        ..Default::default()
                    };
                    let rule_thin = designcraft_doc::CellStroke {
                        weight: 0.5,
                        color: designcraft_color::swatch::BLACK.into(),
                        tint: 1.0,
                        kind: designcraft_doc::StrokeType::Solid,
                        ..Default::default()
                    };
                    if header_rows > 0 {
                        // Top of header (row 0 top)
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(0, c) {
                                cell.strokes[0] = rule_thick.clone();
                            }
                        }
                        // Sub-header rule under group headers (if has_merged_headers)
                        if has_merged_headers {
                            for r in 0..num_header_rows.saturating_sub(1) {
                                for (c, lvls) in col_levels.iter().enumerate() {
                                    if lvls.len() > r + 1 {
                                        if let Some(cell) = table.cell_mut(r, c) {
                                            cell.strokes[2] = rule_thin.clone();
                                        }
                                        if let Some(cell) = table.cell_mut(r + 1, c) {
                                            cell.strokes[0] = rule_thin.clone();
                                        }
                                    }
                                }
                            }
                        }
                        // Bottom of header (row header_rows - 1 bottom)
                        let bottom_header_row = header_rows - 1;
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(bottom_header_row, c) {
                                cell.strokes[2] = rule_thin.clone();
                            }
                            if nr > header_rows
                                && let Some(cell) = table.cell_mut(header_rows, c)
                            {
                                cell.strokes[0] = rule_thin.clone();
                            }
                        }

                        // Col numbers row (if checked)
                        if st.card2_col_numbers && nr > header_rows {
                            let col_num_row = header_rows;
                            for c in 0..nc {
                                if let Some(cell) = table.cell_mut(col_num_row, c) {
                                    cell.strokes[2] = rule_thin.clone();
                                }
                                if nr > col_num_row + 1
                                    && let Some(cell) = table.cell_mut(col_num_row + 1, c)
                                {
                                    cell.strokes[0] = rule_thin.clone();
                                }
                            }
                        }
                    } else if st.card2_col_numbers {
                        // Col number row is row 0
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(0, c) {
                                cell.strokes[0] = rule_thick.clone();
                                cell.strokes[2] = rule_thin.clone();
                            }
                            if nr > 1
                                && let Some(cell) = table.cell_mut(1, c)
                            {
                                cell.strokes[0] = rule_thin.clone();
                            }
                        }
                    } else {
                        // Neither header nor col numbers: top of table
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(0, c) {
                                cell.strokes[0] = rule_thick.clone();
                            }
                        }
                    }

                    // Table footer row rules (if checked)
                    if st.card2_table_footer && nr > header_rows + 1 {
                        let footer_row = nr.saturating_sub(1);
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(footer_row, c) {
                                cell.strokes[0] = rule_thin.clone();
                            }
                            if footer_row > 0
                                && let Some(cell) = table.cell_mut(footer_row - 1, c)
                            {
                                cell.strokes[2] = rule_thin.clone();
                            }
                        }
                    }

                    // Row formatting conditions: if col 1 starts with Jumlah, Tahun, Kab, Kabupaten, Kec, Kecamatan
                    if st.card2_row_conditions {
                        let data_start = if header_rows > 0 {
                            if st.card2_col_numbers { header_rows + 1 } else { header_rows }
                        } else if st.card2_col_numbers {
                            1
                        } else {
                            0
                        };

                        for r in data_start..nr {
                            let matches = {
                                let col0 = table.cell(r, 0).map(|c| c.text.text.as_str()).unwrap_or("");
                                let col1 = table.cell(r, 1).map(|c| c.text.text.as_str()).unwrap_or("");
                                matches_pubgen_row_condition(col0) || (is_index_or_empty(col0) && matches_pubgen_row_condition(col1))
                            };

                            if matches {
                                for c in 0..nc {
                                    // Top border
                                    if let Some(cell) = table.cell_mut(r, c) {
                                        cell.strokes[0] = rule_thin.clone();
                                    }
                                    if r > 0
                                        && let Some(cell) = table.cell_mut(r - 1, c)
                                    {
                                        cell.strokes[2] = rule_thin.clone();
                                    }
                                    // Bottom border
                                    if r + 1 < nr {
                                        if let Some(cell) = table.cell_mut(r, c) {
                                            cell.strokes[2] = rule_thin.clone();
                                        }
                                        if let Some(cell) = table.cell_mut(r + 1, c) {
                                            cell.strokes[0] = rule_thin.clone();
                                        }
                                    } else if let Some(cell) = table.cell_mut(r, c) {
                                        cell.strokes[2] = rule_thick.clone();
                                    }
                                }
                            }
                        }
                    }

                    // Bottom of table rule
                    let last_row = nr.saturating_sub(1);
                    for c in 0..nc {
                        if let Some(cell) = table.cell_mut(last_row, c) {
                            cell.strokes[2] = rule_thick.clone();
                        }
                    }
                }
            }

            if st.card2_table_footer
                && table.nrows() > header_rows_count + 1
                && let Some(r) = table.rows.last_mut()
            {
                r.kind = designcraft_doc::RowKind::Footer;
            }

            // Apply Table Color Theme fills
            if !active_theme.is_none() {
                // 1. Header rows fill
                if !hf_swatch.is_empty() && header_rows > 0 {
                    for r in 0..header_rows.min(nr) {
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(r, c) {
                                cell.fill = hf_swatch.clone();
                                cell.fill_tint = 1.0;
                            }
                        }
                    }
                }

                // 2. Row number row fill
                if !rn_swatch.is_empty() && data_start > header_rows {
                    for r in header_rows..data_start.min(nr) {
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(r, c) {
                                cell.fill = rn_swatch.clone();
                                cell.fill_tint = 1.0;
                            }
                        }
                    }
                }

                // 3. Footer row fill
                if !hf_swatch.is_empty() && st.card2_table_footer && nr > data_start + 1 {
                    let footer_row = nr - 1;
                    for c in 0..nc {
                        if let Some(cell) = table.cell_mut(footer_row, c) {
                            cell.fill = hf_swatch.clone();
                            cell.fill_tint = 1.0;
                        }
                    }
                }
            }

            Ok(())
        })
        .map_err(|e| e.to_string())?;

    let mut actual_fn_target = fn_target;
    if let Some((fid, spread_idx, x0, x1, y0, max_h, margin_y1)) = created_frame_info {
        let fit_h = {
            let active = app.session.active().ok_or("No active document")?;
            let cs = designcraft_compose::compose_story(&active.doc, designcraft_doc::StoryId(story_id), &Default::default());
            let measured_h = cs.frames.first().and_then(|f| f.tables.first().map(|t| t.rect.height()).or(Some(f.content_height)));

            if let Some(th) = measured_h { (th + 4.0).min(max_h).max(40.0) } else { (total_rows as f64 * 32.0 + 40.0).min(max_h).max(60.0) }
        };

        let _ = app.session.edit(|doc, _| {
            if let Some(it) = doc.item_mut(fid) {
                it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(x0, y0, x1, y0 + fit_h));
            }
            Ok(serde_json::Value::Null)
        });

        if footnote_text.is_some() || !table_notes.is_empty() {
            let fn_y0 = y0 + fit_h + 8.0;
            let fn_y1 = (fn_y0 + 24.0).min(margin_y1);
            actual_fn_target = Some((spread_idx, [x0, fn_y0, x1, fn_y1]));
        }
    }

    if !table_notes.is_empty()
        && let Some((target_spread, fn_rect)) = actual_fn_target
    {
        let max_label_len = table_notes.iter().map(|n| n.label.chars().count()).max().unwrap_or(6);
        let colon_pos = if max_label_len > 7 { 58.0 } else { 46.0 };
        let text_pos = colon_pos + 8.0;

        let mut note_lines: Vec<(String, bool)> = Vec::new();
        for item in &table_notes {
            let id_text = if item.content.is_empty() {
                format!("{}\t{}", item.label, item.colon)
            } else {
                format!("{}\t{}\t{}", item.label, item.colon, item.content)
            };
            note_lines.push((id_text, item.is_italic));

            if st.bilingual_output {
                let (label_en, colon_en, content_en) = ai::translate_note_item(&item.label, &item.colon, &item.content, &st.translations);
                let en_text =
                    if content_en.is_empty() { format!("{}\t{}", label_en, colon_en) } else { format!("{}\t{}\t{}", label_en, colon_en, content_en) };
                note_lines.push((en_text, true));
            }
        }

        let note_text = note_lines.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>().join("\n");

        if let Ok(res) = app.run(
            "frame.create",
            json!({
                "spread": target_spread,
                "rect": [fn_rect[0], fn_rect[1], fn_rect[2], fn_rect[1] + 60.0],
                "content": "text",
                "text": note_text,
                "caret": false
            }),
        ) {
            let fn_story_id = res.get("story").and_then(Value::as_u64);
            let fn_item_id = res.get("id").and_then(Value::as_u64).map(designcraft_doc::ItemId);

            if let Some(fn_sid) = fn_story_id {
                let tab_stop_colon = designcraft_doc::TabStop {
                    position: colon_pos,
                    align: designcraft_doc::TabAlign::Left,
                    leader: String::new(),
                    align_on: String::new(),
                };
                let tab_stop_text = designcraft_doc::TabStop {
                    position: text_pos,
                    align: designcraft_doc::TabAlign::Left,
                    leader: String::new(),
                    align_on: String::new(),
                };

                let _ = app.session.edit(|doc, _| {
                    if let Some(story) = doc.story_mut(designcraft_doc::StoryId(fn_sid)) {
                        for p in &mut story.paras {
                            p.para.left_indent = Some(text_pos);
                            p.para.first_line_indent = Some(-text_pos);
                            p.para.tabs = Some(vec![tab_stop_colon.clone(), tab_stop_text.clone()]);
                            p.para.space_after = Some(2.0);
                        }
                        let ranges = story.para_ranges();
                        for (idx, (_, is_italic)) in note_lines.iter().enumerate() {
                            if let Some(r) = ranges.get(idx) {
                                let end = if idx + 1 < ranges.len() { ranges[idx + 1].start } else { r.end };
                                if r.start < end {
                                    story.format_chars(r.start..end, |f| {
                                        f.over.size = Some(8.5);
                                        f.over.font_style = Some(if *is_italic { "Italic".to_string() } else { "Regular".to_string() });
                                    });
                                }
                            }
                        }
                        story.rev += 1;
                    }
                    Ok(serde_json::Value::Null)
                });

                let fit_note_h = {
                    if let Some(active) = app.session.active() {
                        let cs = designcraft_compose::compose_story(&active.doc, designcraft_doc::StoryId(fn_sid), &Default::default());
                        let h = cs.frames.first().map(|f| f.content_height).unwrap_or(24.0);
                        (h + 4.0).max(18.0)
                    } else {
                        (note_lines.len() as f64 * 16.0 + 8.0).max(20.0)
                    }
                };

                if let Some(fid) = fn_item_id {
                    let _ = app.session.edit(|doc, _| {
                        if let Some(it) = doc.item_mut(fid) {
                            it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(
                                fn_rect[0],
                                fn_rect[1],
                                fn_rect[2],
                                fn_rect[1] + fit_note_h,
                            ));
                        }
                        Ok(serde_json::Value::Null)
                    });
                }
            }
        }
    } else if let (Some(fn_str), Some((target_spread, fn_rect))) = (footnote_text.as_ref(), actual_fn_target) {
        let _ = app.run(
            "frame.create",
            json!({
                "spread": target_spread,
                "rect": fn_rect,
                "content": "text",
                "text": fn_str,
                "caret": false
            }),
        );
    }

    let target_msg = match target_page_info {
        Some(pi) => {
            let label = app.session.page_label(pi);
            format!(" on Page {label}")
        }
        None => String::new(),
    };

    let fn_msg = if !table_notes.is_empty() {
        " + notes"
    } else if footnote_text.is_some() {
        " + footnote"
    } else {
        ""
    };
    Ok(format!("Inserted table{target_msg} ({total_rows} rows × {cols_count} cols){fn_msg}"))
}

pub(crate) fn to_proper_case(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut chars = trimmed.chars();
    let first = match chars.next() {
        Some(c) => c.to_uppercase().to_string(),
        None => return String::new(),
    };
    let rest: String = chars.map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    format!("{first}{rest}")
}

pub(crate) fn is_plausible_type(txt: &str) -> bool {
    let trimmed = txt.trim();
    let lower = trimmed.to_lowercase();
    // Exclude footnotes, sources, notes, captions
    if lower.starts_with("source")
        || lower.starts_with("sumber")
        || lower.starts_with("note")
        || lower.starts_with("catatan")
        || lower.starts_with("ket")
        || lower.contains(':')
    {
        return false;
    }
    // Publication kickers/types are short (e.g. "Tabel", "Grafik", "Bab", "Section")
    // Never more than 2 words or 20 characters
    if trimmed.len() > 20 || trimmed.split_whitespace().count() > 2 {
        return false;
    }
    true
}

pub(crate) fn is_plausible_number(txt: &str) -> bool {
    let trimmed = txt.trim();
    if !trimmed.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    if trimmed.len() > 12 || trimmed.split_whitespace().count() > 1 {
        return false;
    }
    trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '(' || c == ')')
}

pub(crate) fn find_type_and_number_on_page(doc: &designcraft_doc::Document, page_str: &str, title: &str) -> Option<(String, String)> {
    let mut target_abs = None;
    for abs in 0..doc.page_count() {
        if doc.page_name(abs) == page_str {
            target_abs = Some(abs);
            break;
        }
    }
    let abs = target_abs?;
    let (si, pi) = doc.page_loc(abs)?;
    let spread = doc.spreads.get(si)?;

    let mut type_candidate = None;
    let mut num_candidate = None;

    for item in &spread.items {
        if spread.page_at_x(item.bounds().center().x) != Some(pi) {
            continue;
        }
        let Some(tf) = item.text_frame() else { continue };
        let Some(st) = doc.story(tf.story) else { continue };

        for line in st.text.split(['\n', designcraft_doc::story::FORCED_LINE_BREAK]) {
            let txt = line.trim();
            if txt.is_empty() || txt == title {
                continue;
            }

            if is_plausible_number(txt) {
                if num_candidate.is_none() {
                    num_candidate = Some(txt.to_string());
                }
            } else if is_plausible_type(txt) {
                if type_candidate.is_none() {
                    type_candidate = Some(to_proper_case(txt));
                }
            } else {
                for prefix in &["Grafik", "Chart", "Tabel", "Table"] {
                    if let Some(rest) = txt.strip_prefix(prefix) {
                        let candidate = rest.split_whitespace().next().unwrap_or("");
                        if is_plausible_number(candidate) {
                            if type_candidate.is_none() {
                                type_candidate = Some(to_proper_case(prefix));
                            }
                            if num_candidate.is_none() {
                                num_candidate = Some(candidate.to_string());
                            }
                        }
                    } else if let Some(rest) = txt.to_lowercase().strip_prefix(&prefix.to_lowercase()) {
                        let candidate = rest.split_whitespace().next().unwrap_or("");
                        if is_plausible_number(candidate) {
                            if type_candidate.is_none() {
                                type_candidate = Some(to_proper_case(prefix));
                            }
                            if num_candidate.is_none() {
                                num_candidate = Some(candidate.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    match (type_candidate, num_candidate) {
        (Some(t), Some(n)) => Some((t, n)),
        (Some(t), None) => Some((t, String::new())),
        (None, Some(n)) => Some((String::new(), n)),
        (None, None) => None,
    }
}

/// Check if an entry line contains only a kicker type word and/or a number (e.g. "Grafik 02.01", "Tabel 01.01", "02.01")
/// with no actual descriptive title words.
pub(crate) fn is_type_and_number_only(t: &str) -> bool {
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 2 {
        return false;
    }
    if words.len() == 1 {
        let w = words[0].trim_end_matches('.');
        w.chars().any(|c| c.is_ascii_digit()) && w.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    } else {
        let w0_lower = words[0].to_lowercase();
        let is_type = ["grafik", "chart", "tabel", "table", "bab", "section"].contains(&w0_lower.as_str());
        let w1 = words[1].trim_end_matches('.');
        let is_num = w1.chars().any(|c| c.is_ascii_digit()) && w1.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-');
        is_type && is_num
    }
}
/// Detect row indices (0-based relative to matrix) matching the summary/break keyword condition.
pub fn detect_condition_rows(matrix: &[Vec<String>], first_row_header: bool) -> Vec<usize> {
    let start_idx = if first_row_header { 1 } else { 0 };
    let mut matching = Vec::new();
    for (i, row) in matrix.iter().enumerate().skip(start_idx) {
        let col0 = row.first().map(String::as_str).unwrap_or("");
        let col1 = row.get(1).map(String::as_str).unwrap_or("");
        if matches_pubgen_row_condition(col0) || (is_index_or_empty(col0) && matches_pubgen_row_condition(col1)) {
            matching.push(i);
        }
    }
    matching
}

/// Estimate raw advance width of a string in points (based on standard 12pt proportional font).
fn estimate_text_advance(text: &str) -> f64 {
    let mut w: f64 = 0.0;
    for c in text.chars() {
        w += match c {
            ' ' => 5.0,
            '.' | ',' | ':' | ';' | '!' | '\'' | '|' | '/' | '\\' => 3.2,
            'i' | 'l' | 'j' => 4.0,
            'I' | 'J' => 4.8,
            'f' | 't' | 'r' => 4.8,
            '-' | '–' | '—' => 5.5,
            '0'..='9' => 7.5,
            'a'..='z' => match c {
                'm' | 'w' => 10.0,
                _ => 6.8,
            },
            'A'..='Z' => match c {
                'M' | 'W' => 12.0,
                'C' | 'D' | 'G' | 'H' | 'K' | 'N' | 'O' | 'Q' | 'U' | 'V' | 'X' | 'Y' => 9.0,
                'E' | 'F' | 'L' | 'P' | 'S' | 'T' | 'Z' => 7.5,
                _ => 8.2,
            },
            _ => 7.5,
        };
    }
    w
}

/// Estimate single-line text advance width in points (based on standard 12pt proportional font).
fn estimate_text_width(text: &str, is_uppercase_kicker: bool) -> f64 {
    let w = estimate_text_advance(text);
    // Headroom buffer so Knuth-Plass / greedy line breaker never breaks single-line text
}
