//! The table generation engine: CSV matrix -> native styled table items.

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct TableInsertOutput {
    pub message: String,
    pub story_id: designcraft_doc::StoryId,
    pub frame_id: designcraft_doc::ItemId,
    pub page_idx: usize,
}

pub(crate) fn execute_insert_table(app: &mut DesignApp, st: &PubgenState) -> Result<String, String> {
    let doc_margins = {
        let active = app.session.active().ok_or("No active document")?;
        page_utils::detect_document_margins(&active.doc)
    };
    let info = execute_insert_table_matrix_info(app, st, None, None)?;
    let mut cur_page = info.page_idx;
    thread_overset_table(app, &mut cur_page, info.story_id, info.frame_id, doc_margins);
    Ok(info.message)
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

#[allow(dead_code)]
pub(crate) fn execute_insert_table_matrix(
    app: &mut DesignApp,
    st: &PubgenState,
    matrix_override: Option<&[Vec<String>]>,
    target_page_override: Option<usize>,
) -> Result<String, String> {
    execute_insert_table_matrix_info(app, st, matrix_override, target_page_override).map(|o| o.message)
}

#[allow(clippy::needless_range_loop)]
pub(crate) fn execute_insert_table_matrix_info(
    app: &mut DesignApp,
    st: &PubgenState,
    matrix_override: Option<&[Vec<String>]>,
    target_page_override: Option<usize>,
) -> Result<TableInsertOutput, String> {
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

            // Find bottom-most edge of generated page title / header items on this page.
            // Template pages: only the table-title slots count — a full-page scan would pick
            // up the low-placed source slot and push the table below the bottom margin.
            let slot_title_bottom = spread
                .items
                .iter()
                .filter_map(|it| {
                    if it.label.starts_with("#slot:text:table_") {
                        let b = it.bounds();
                        (b.x0 < page_r.x1 && b.x1 > page_r.x0).then_some(b.y1)
                    } else {
                        None
                    }
                })
                .fold(f64::NEG_INFINITY, f64::max);

            let mut title_bottom = margin.y0;
            if slot_title_bottom.is_finite() {
                title_bottom = slot_title_bottom;
            } else {
                for it in &spread.items {
                    let b = it.bounds();
                    // Check if item lies on this page horizontally and in header region vertically
                    if b.x0 < page_r.x1 && b.x1 > page_r.x0 && (st.active_profile().id == "bps_brs" || b.y0 < margin.y0 + 260.0) {
                        // Ignore text frames that already contain tables
                        let is_table = it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|st| !st.tables.is_empty());
                        if !is_table && b.y1 > title_bottom {
                            title_bottom = b.y1;
                        }
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
            let gap_below_title = if st.active_profile().id == "bps_brs" { 2.0 } else { 6.0 };
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
    let is_brs = st.active_profile().id == "bps_brs";
    let footer_enabled = !is_brs && st.card2_table_footer && st.active_profile().table_styling.include_table_footer;
    let footer_rows_count = if footer_enabled && rows_count > header_rows_count + 1 { 1 } else { 0 };
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

    // Step 5: Table Style (if not Horizontal Rules / BRS6, apply via command first)
    let style_names = ["Basic Grid", "Corporate Navy", "Minimalist", "Horizontal Rules", "BRS6"];
    if st.card2_style != 3 && st.card2_style != 4
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
            let (hf_swatch, rn_swatch, header_text_swatch, odd_swatch, even_swatch) = if !active_theme.is_none() {
                let hf = ensure_swatch(doc, &active_theme.header_footer);
                let rn = ensure_swatch(doc, &active_theme.row_number);
                let ht = active_theme.header_text_color.as_deref().map(|col| ensure_swatch(doc, col));
                let odd = active_theme.data_odd_fill.as_deref().map(|col| ensure_swatch(doc, col));
                let even = active_theme.data_even_fill.as_deref().map(|col| ensure_swatch(doc, col));
                (hf, rn, ht, odd, even)
            } else {
                (String::new(), String::new(), None, None, None)
            };

            if (st.card2_style == 3 || st.card2_style == 4) && !doc.styles.table.iter().any(|s| s.name == "Horizontal Rules") {
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

            // One profile font for every cell so tables never fall back to the document default.
            let fam = st.body_font();
            let base_font_size = 9.0;
            let table_font_size = if let Some(delta) = active_theme.font_size_delta {
                (base_font_size + delta as f64).max(6.0)
            } else if is_brs {
                8.0
            } else {
                base_font_size
            };
            for r in 0..nr {
                for c in 0..nc {
                    if let Some(cell) = table.cell_mut(r, c) {
                        let len = cell.text.text.len();
                        cell.text.format_chars(0..len, |f| {
                            f.over.font_family = Some(fam.clone());
                            f.over.size = Some(table_font_size);
                        });
                    }
                }
            }

            if st.card2_style == 3 || st.card2_style == 4 {
                table.style = if st.card2_style == 4 { "BRS6".to_string() } else { "Horizontal Rules".to_string() };
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
                    if footer_enabled && nr > header_rows + 1 {
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

            if footer_enabled
                && table.nrows() > header_rows_count + 1
                && let Some(r) = table.rows.last_mut()
            {
                r.kind = designcraft_doc::RowKind::Footer;
            }

            // Apply Table Color Theme fills
            if !active_theme.is_none() {
                // 1. Header rows fill & font color
                if !hf_swatch.is_empty() && header_rows > 0 {
                    for r in 0..header_rows.min(nr) {
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(r, c) {
                                cell.fill = hf_swatch.clone();
                                cell.fill_tint = 1.0;
                                if let Some(ref text_sw) = header_text_swatch {
                                    let len = cell.text.text.len();
                                    cell.text.format_chars(0..len, |f| {
                                        f.over.fill = Some(text_sw.clone());
                                    });
                                }
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

                // 3. Alternating data rows fill (Odd & Even)
                if let (Some(odd_sw), Some(even_sw)) = (&odd_swatch, &even_swatch) {
                    for r in data_start..nr {
                        if footer_enabled && r == nr - 1 {
                            continue;
                        }
                        let data_idx = r - data_start;
                        let swatch = if data_idx % 2 == 0 { odd_sw } else { even_sw };
                        for c in 0..nc {
                            if let Some(cell) = table.cell_mut(r, c) {
                                cell.fill = swatch.clone();
                                cell.fill_tint = 1.0;
                            }
                        }
                    }
                }

                // 4. Footer row fill
                if !hf_swatch.is_empty() && footer_enabled && nr > data_start + 1 {
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
    let mut below_table_y: Option<f64> = None;
    let created_spread = created_frame_info.as_ref().map(|(_, si, ..)| *si);
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
        } else {
            below_table_y = Some(y0 + fit_h + 6.0);
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
                let (label_en, colon_en, content_en) =
                    ai::translate_note_item(&item.label, &item.colon, &item.content, &st.translations, &st.glossary);
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
                below_table_y = Some(fn_rect[1] + fit_note_h + 6.0);

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
        below_table_y = Some(fn_rect[3] + 6.0);
    }

    // A template page keeps its `#slot:text:source` frame at a fixed low y; slide it
    // below the inserted table (and any notes) so the table never runs over it.
    if let (Some(src_y), Some(spread_idx)) = (below_table_y, created_spread) {
        let src_id = app
            .session
            .active()
            .and_then(|a| a.doc.spreads.get(spread_idx).and_then(|sp| sp.items.iter().find(|it| it.label == "#slot:text:source").map(|it| it.id)));
        if let Some(src_id) = src_id {
            let _ = app.session.edit(|doc, _| {
                if let Some(it) = doc.item_mut(src_id)
                    && let Some(pb) = it.path.bounds()
                {
                    it.path = designcraft_geom::shapes::rectangle(designcraft_geom::Rect::new(pb.x0, src_y, pb.x1, src_y + pb.height()));
                }
                Ok(serde_json::Value::Null)
            });
        }
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
    let fid = created_frame_info
        .map(|(fid, ..)| fid)
        .or_else(|| {
            app.session
                .active()
                .and_then(|a| a.doc.story(designcraft_doc::StoryId(story_id)).and_then(|s| s.frames.first().copied()))
        })
        .ok_or("No frame created for table")?;

    Ok(TableInsertOutput {
        message: format!("Inserted table{target_msg} ({total_rows} rows × {cols_count} cols){fn_msg}"),
        story_id: designcraft_doc::StoryId(story_id),
        frame_id: fid,
        page_idx: target_page_info.unwrap_or(cur_page),
    })
}

/// Thread any table story that oversets its frame onto freshly inserted continuation pages
/// immediately following `cur_page_idx`. The caller's `cur_page_idx` is advanced so subsequent
/// content is placed after the table continuation, never colliding with other items or jumping
/// to the end of the document.
pub(crate) fn thread_overset_table(
    app: &mut DesignApp,
    cur_page_idx: &mut usize,
    sid: designcraft_doc::StoryId,
    first_fid: designcraft_doc::ItemId,
    doc_margins: designcraft_doc::Margins,
) -> usize {
    let mut added_pages = 0usize;
    let mut last_fid = first_fid.0;

    for _ in 0..50 {
        if !story_is_overset(app, sid) {
            break;
        }

        // 1. Insert 1 page immediately AFTER cur_page_idx
        let insert_res = app.run("layout.pages.insert", json!({"after": *cur_page_idx, "count": 1}));
        if insert_res.is_err() {
            break;
        }
        *cur_page_idx += 1;
        added_pages += 1;

        // 2. Set margins of the newly inserted page to doc_margins
        let new_page_idx = *cur_page_idx;
        let _ = app.session.edit(|doc, _| {
            if let Some((si, pi)) = doc.page_loc(new_page_idx)
                && let Some(sp) = doc.spreads.get_mut(si)
            {
                let sp = std::sync::Arc::make_mut(sp);
                if let Some(pg) = sp.pages.get_mut(pi) {
                    pg.margins = doc_margins;
                    pg.local_margins = true;
                }
                sp.relayout();
            }
            Ok(serde_json::Value::Null)
        });

        // 3. Get margin rectangle and spread index of the new page
        let (margin_rect, spread_idx) = {
            let active = app.session.active();
            if let Some(active) = active
                && let Some((si, pi)) = active.doc.page_loc(new_page_idx)
                && let Some(spread) = active.doc.spreads.get(si)
                && let Some(page) = spread.pages.get(pi)
            {
                (page.margin_rect(), si)
            } else {
                break;
            }
        };

        // 4. Thread from last_fid to a new frame on the new page
        let thread_res = app.run(
            "text.thread",
            json!({
                "from": last_fid,
                "spread": spread_idx,
                "rect": [margin_rect.x0, margin_rect.y0, margin_rect.x1, margin_rect.y1],
                "autoflow": false,
            }),
        );

        match thread_res {
            Ok(val) => {
                if let Some(to_fid) = val.get("to").and_then(Value::as_u64) {
                    last_fid = to_fid;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if added_pages > 0 {
        // Fit the final continuation frame to its content height
        let _ = app.run("object.fit", json!({"mode": "fitFrameToContent", "axis": "height", "ids": [last_fid]}));
    }

    added_pages
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
pub(crate) fn estimate_text_advance(text: &str) -> f64 {
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
pub(crate) fn estimate_text_width(text: &str, is_uppercase_kicker: bool) -> f64 {
    let w = estimate_text_advance(text);
    // Headroom buffer so Knuth-Plass / greedy line breaker never breaks single-line text
    let buffer = if is_uppercase_kicker { 12.0 } else { 8.0 };
    (w + buffer).max(20.0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_table_from_csv_defaults_tick_column_numbers_and_footer() {
        let st = PubgenState::default();
        assert!(st.card2_first_row_header, "First row is header is on by default");
        assert!(st.card2_col_numbers, "Add column numbers is on by default");
        assert!(st.card2_table_footer, "Table footer is on by default");
    }

    #[test]
    fn test_execute_insert_table() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Header 1".into(), "Header 2".into()],
                vec!["Cell A1".into(), "Cell A2".into()],
                vec!["Cell B1".into(), "Cell B2".into()],
            ],
            card2_first_row_header: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with Horizontal Rules should succeed: {:?}", res);
    }

    #[test]
    fn test_brs6_table_style_and_colors() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let matrix = vec![
            vec!["Provinsi".into(), "NTP".into(), "Perubahan".into()],
            vec!["DKI Jakarta".into(), "108.74".into(), "-2.07".into()],
            vec!["Jawa Barat".into(), "119.36".into(), "0.99".into()],
            vec!["Jawa Tengah".into(), "115.20".into(), "1.12".into()],
        ];

        let st = PubgenState {
            active_profile_id: "bps_brs".into(),
            card2_matrix: matrix.clone(),
            card2_col_numbers: true,
            card2_first_row_header: true,
            card2_table_footer: false,
            card2_style: 4, // BRS6
            card2_color_theme: 4, // BRS6
            card2_theme_override: Some("BRS6".into()),
            ..Default::default()
        };

        let res = execute_insert_table_matrix(&mut app, &st, Some(&matrix), None);
        assert!(res.is_ok(), "Insert table matrix failed: {res:?}");

        let active = app.session.active().unwrap();
        let story = active.doc.stories.values().find(|s| !s.tables.is_empty()).expect("Table story must exist");
        let table = story.tables.values().next().expect("Table must exist");

        // Row 0: Header -> fill #D07C38, font color [Paper] (white)
        let cell_header = table.cell(0, 0).expect("Header cell must exist");
        assert_eq!(cell_header.fill, "#D07C38", "Header fill must be #D07C38");
        assert_eq!(cell_header.text.chars[0].format.over.fill.as_deref(), Some("[Paper]"), "Header font color must be [Paper] (white)");

        // Row 1: Col numbers row -> fill #E7CFB5
        let cell_col_num = table.cell(1, 0).expect("Col num cell must exist");
        assert_eq!(cell_col_num.fill, "#E7CFB5", "Col number row fill must be #E7CFB5");

        // Row 2: 1st Data row (Odd in 1-based) -> fill #FEF0D2
        let cell_odd = table.cell(2, 0).expect("Odd data cell must exist");
        assert_eq!(cell_odd.fill, "#FEF0D2", "Odd data row fill must be #FEF0D2");

        // Row 3: 2nd Data row (Even in 1-based) -> fill #FEF7E9
        let cell_even = table.cell(3, 0).expect("Even data cell must exist");
        assert_eq!(cell_even.fill, "#FEF7E9", "Even data row fill must be #FEF7E9");

        // Font size reduced by 1pt: 8.0pt
        assert_eq!(cell_header.text.chars[0].format.over.size, Some(8.0), "Font size must be 8.0pt (reduced 1pt)");
        assert_eq!(cell_odd.text.chars[0].format.over.size, Some(8.0), "Font size must be 8.0pt (reduced 1pt)");
    }

    #[test]
    fn test_execute_insert_table_at_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Generate pages with Card 1 (preset 0: 1 Row · 3 Columns)
        let st_card1 = PubgenState {
            card1_records: vec![vec!["Chapter".into(), "1".into(), "Introduction to Layout".into()]],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_range_from: 1,
            card1_range_to: 1,
            card1_preset: 0,
            card1_append: true,
            ..Default::default()
        };
        let res1 = execute_generate_pages(&mut app, &st_card1);
        assert!(res1.is_ok(), "generate_pages should succeed: {:?}", res1);

        // 2. Insert table with placement = 2 ("At page")
        let st_card2 = PubgenState {
            card2_matrix: vec![vec!["Item".into(), "Price".into()], vec!["Widget A".into(), "$10".into()], vec!["Widget B".into(), "$20".into()]],
            card2_first_row_header: true,
            card2_placement: 2, // At page
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };
        let res2 = execute_insert_table(&mut app, &st_card2);
        assert!(res2.is_ok(), "insert_table at page should succeed: {:?}", res2);

        // 3. Verify table frame position is below generated title
        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        let page = spread.pages.last().unwrap();
        let margin = page.margin_rect();

        // Find the table frame (its story contains tables)
        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|st| !st.tables.is_empty()))
            .expect("Table item should exist on spread");

        // In preset 0, title row has row_y1 = margin.y0 + 60.0 and gap_below_title = 6.0
        // The table frame should start at margin.y0 + 60.0 + 6.0 = margin.y0 + 66.0
        assert_eq!(table_item.bounds().y0, margin.y0 + 66.0);
        assert_eq!(table_item.bounds().x0, margin.x0);
        assert_eq!(table_item.bounds().x1, margin.x1);
    }

    #[test]
    fn test_execute_insert_table_with_col_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item".into(), "Category".into(), "Price".into()],
                vec!["Paper".into(), "Stock".into(), "$20".into()],
                vec!["Pen".into(), "Tools".into(), "$10".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // Total rows should be 4 (Header + Col numbers + 2 data rows)
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 3);

        // Row 0 is header
        assert_eq!(table.cell(0, 0).unwrap().text.text.trim(), "Item");
        assert_eq!(table.cell(0, 1).unwrap().text.text.trim(), "Category");
        assert_eq!(table.cell(0, 2).unwrap().text.text.trim(), "Price");

        // Row 1 is column numbers (1), (2), (3)
        assert_eq!(table.cell(1, 0).unwrap().text.text.trim(), "(1)");
        assert_eq!(table.cell(1, 1).unwrap().text.text.trim(), "(2)");
        assert_eq!(table.cell(1, 2).unwrap().text.text.trim(), "(3)");

        // Row 2 is data
        assert_eq!(table.cell(2, 0).unwrap().text.text.trim(), "Paper");
    }

    #[test]
    fn test_execute_insert_table_horizontal_rules() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Header A".into(), "Header B".into()],
                vec!["Data 1A".into(), "Data 1B".into()],
                vec!["Data 2A".into(), "Data 2B".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: false, // pin it off: this test wants plain interior data rows
            card2_placement: 1,        // New frame
            card2_style: 3,            // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with Horizontal Rules should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 4 rows: 0 (Header), 1 (Col numbers), 2 (Data 1), 3 (Data 2)
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 2);

        // Verify table-level border is none
        assert_eq!(table.options.border.weight, 0.0);

        // Row 0: Table Header
        // Top border should be 1.0pt, bottom border should be 0.5pt, left/right 0.0pt
        let r0_c0 = table.cell(0, 0).unwrap();
        assert_eq!(r0_c0.strokes[0].weight, 1.0, "Header top border must be 1.0pt");
        assert_eq!(r0_c0.strokes[2].weight, 0.5, "Header bottom border must be 0.5pt");
        assert_eq!(r0_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r0_c0.strokes[3].weight, 0.0, "No vertical right border");

        // Row 1: Col Numbers
        // Top border 0.5pt, bottom border 0.5pt, left/right 0.0pt
        let r1_c0 = table.cell(1, 0).unwrap();
        assert_eq!(r1_c0.strokes[0].weight, 0.5, "Col numbers top border must be 0.5pt");
        assert_eq!(r1_c0.strokes[2].weight, 0.5, "Col numbers bottom border must be 0.5pt");
        assert_eq!(r1_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r1_c0.strokes[3].weight, 0.0, "No vertical right border");

        // Row 2: Data Row 1 (Interior data row)
        // Top border 0.5pt (from row 1 bottom), bottom border 0.0pt (no border between data rows!)
        let r2_c0 = table.cell(2, 0).unwrap();
        assert_eq!(r2_c0.strokes[0].weight, 0.5);
        assert_eq!(r2_c0.strokes[2].weight, 0.0, "Interior data row bottom must have no border");
        assert_eq!(r2_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r2_c0.strokes[3].weight, 0.0, "No vertical right border");

        // Row 3: Data Row 2 (Bottom table row)
        // Top border 0.0pt, bottom border 1.0pt (bottom of table rule!)
        let r3_c0 = table.cell(3, 0).unwrap();
        assert_eq!(r3_c0.strokes[0].weight, 0.0, "Interior data row top must have no border");
        assert_eq!(r3_c0.strokes[2].weight, 1.0, "Bottom of table must have 1.0pt border");
        assert_eq!(r3_c0.strokes[1].weight, 0.0, "No vertical left border");
        assert_eq!(r3_c0.strokes[3].weight, 0.0, "No vertical right border");
    }

    #[test]
    fn test_execute_insert_table_horizontal_rules_no_col_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Header A".into(), "Header B".into()],
                vec!["Data 1A".into(), "Data 1B".into()],
                vec!["Data 2A".into(), "Data 2B".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false, // pin it off: this test wants plain interior data rows
            card2_placement: 1,        // New frame
            card2_style: 3,            // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok());

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 rows: 0 (Header), 1 (Data 1), 2 (Data 2)
        assert_eq!(table.rows.len(), 3);

        // Row 0: Header (top 1.0, bottom 0.5)
        let r0 = table.cell(0, 0).unwrap();
        assert_eq!(r0.strokes[0].weight, 1.0);
        assert_eq!(r0.strokes[2].weight, 0.5);
        assert_eq!(r0.strokes[1].weight, 0.0);
        assert_eq!(r0.strokes[3].weight, 0.0);

        // Row 1: Data 1 (top 0.5, bottom 0.0)
        let r1 = table.cell(1, 0).unwrap();
        assert_eq!(r1.strokes[0].weight, 0.5);
        assert_eq!(r1.strokes[2].weight, 0.0);
        assert_eq!(r1.strokes[1].weight, 0.0);
        assert_eq!(r1.strokes[3].weight, 0.0);

        // Row 2: Data 2 (top 0.0, bottom 1.0)
        let r2 = table.cell(2, 0).unwrap();
        assert_eq!(r2.strokes[0].weight, 0.0);
        assert_eq!(r2.strokes[2].weight, 1.0);
        assert_eq!(r2.strokes[1].weight, 0.0);
        assert_eq!(r2.strokes[3].weight, 0.0);
    }

    #[test]
    fn test_execute_insert_table_auto_merged_headers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item Name".into(), "Category".into(), "Item--Price".into(), "Item--Stock".into(), "Rating".into()],
                vec!["A4 Coated Art Paper".into(), "Print Stock".into(), "$24.50".into(), "150".into(), "4.8".into()],
                vec!["Pantone Color Bridge".into(), "Tools".into(), "$185.00".into(), "24".into(), "4.9".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with auto merged headers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 2 header rows + 2 data rows = 4 total rows
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 5);

        // Header has 2 rows
        assert_eq!(table.header_rows(), 2);

        // Row 0 Col 0 ("Item Name"): vertical merge (row_span 2, col_span 1)
        let c00 = table.cell(0, 0).unwrap();
        assert_eq!(c00.row_span, 2);
        assert_eq!(c00.col_span, 1);
        assert_eq!(c00.text.text.trim(), "Item Name");
        assert_eq!(c00.vj, designcraft_doc::VerticalJustification::Center);
        assert_eq!(c00.text.paras[0].para.align, Some(designcraft_doc::Align::Center));

        // Row 0 Col 1 ("Category"): vertical merge (row_span 2, col_span 1)
        let c01 = table.cell(0, 1).unwrap();
        assert_eq!(c01.row_span, 2);
        assert_eq!(c01.col_span, 1);
        assert_eq!(c01.text.text.trim(), "Category");
        assert_eq!(c01.vj, designcraft_doc::VerticalJustification::Center);

        // Row 0 Col 2 ("Item"): horizontal merge across Col 2 and Col 3 (row_span 1, col_span 2)
        let c02 = table.cell(0, 2).unwrap();
        assert_eq!(c02.row_span, 1);
        assert_eq!(c02.col_span, 2);
        assert_eq!(c02.text.text.trim(), "Item");
        assert_eq!(c02.vj, designcraft_doc::VerticalJustification::Center);
        assert_eq!(c02.text.paras[0].para.align, Some(designcraft_doc::Align::Center));

        // Row 1 Col 2 ("Price"): sub-header cell
        let c12 = table.cell(1, 2).unwrap();
        assert_eq!(c12.row_span, 1);
        assert_eq!(c12.col_span, 1);
        assert_eq!(c12.text.text.trim(), "Price");
        assert_eq!(c12.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 3 ("Stock"): sub-header cell
        let c13 = table.cell(1, 3).unwrap();
        assert_eq!(c13.row_span, 1);
        assert_eq!(c13.col_span, 1);
        assert_eq!(c13.text.text.trim(), "Stock");
        assert_eq!(c13.vj, designcraft_doc::VerticalJustification::Center);

        // Row 0 Col 4 ("Rating"): vertical merge (row_span 2, col_span 1)
        let c04 = table.cell(0, 4).unwrap();
        assert_eq!(c04.row_span, 2);
        assert_eq!(c04.col_span, 1);
        assert_eq!(c04.text.text.trim(), "Rating");
        assert_eq!(c04.vj, designcraft_doc::VerticalJustification::Center);

        // Row 2 is first data row
        assert_eq!(table.cell(2, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");
        assert_eq!(table.cell(2, 1).unwrap().text.text.trim(), "Print Stock");
        assert_eq!(table.cell(2, 2).unwrap().text.text.trim(), "$24.50");
        assert_eq!(table.cell(2, 3).unwrap().text.text.trim(), "150");
        assert_eq!(table.cell(2, 4).unwrap().text.text.trim(), "4.8");
    }

    #[test]
    fn test_execute_insert_table_auto_merged_headers_with_col_numbers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item Name".into(), "Category".into(), "Item--Price".into(), "Item--Stock".into(), "Rating".into()],
                vec!["A4 Coated Art Paper".into(), "Print Stock".into(), "$24.50".into(), "150".into(), "4.8".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with auto merged headers + col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 2 header rows + 1 col numbers row + 1 data row = 4 total rows
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 5);

        // Row 2 is column numbers row: (1), (2), (3), (4), (5)
        assert_eq!(table.cell(2, 0).unwrap().text.text.trim(), "(1)");
        assert_eq!(table.cell(2, 1).unwrap().text.text.trim(), "(2)");
        assert_eq!(table.cell(2, 2).unwrap().text.text.trim(), "(3)");
        assert_eq!(table.cell(2, 3).unwrap().text.text.trim(), "(4)");
        assert_eq!(table.cell(2, 4).unwrap().text.text.trim(), "(5)");

        // Row 3 is data row
        assert_eq!(table.cell(3, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");
    }

    #[test]
    fn test_multiline_csv_header_cells_stay_in_one_table_row() {
        // Excel wraps long header cells; the generated CSV quotes the line break. It must not
        // become extra table rows when the CSV is converted to a table.
        let csv = concat!(
            "Desa/Kelurahan / Village/Subdistric,\"Luas / Total Area\n(km2 / sq.km)\",",
            "\"Persentase terhadap Luas Kecamatan\n/ Percentage to District Area\"\n",
            "Warungbambu,1.65,21.32\n",
        );
        let matrix = parse_csv_content_with_delim(csv, ',');
        assert_eq!(matrix.len(), 2);
        assert!(matrix[0][1].contains('\n'));

        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: matrix,
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };
        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "multiline header table should insert: {res:?}");

        let active = app.session.active().unwrap();
        let table = active.doc.stories.values().find_map(|s| s.tables.values().next()).expect("Table should exist in story");
        assert_eq!(table.rows.len(), 2, "one header row + one data row");
        assert_eq!(table.header_rows(), 1);
        assert_eq!(table.cell(0, 1).unwrap().text.text.trim(), "Luas / Total Area\u{2028}(km2 / sq.km)", "the header keeps its Excel line break");
        assert_eq!(table.cell(0, 2).unwrap().text.text.trim(), "Persentase terhadap Luas Kecamatan\u{2028}/ Percentage to District Area");
    }

    #[test]
    fn test_execute_insert_table_3stack_headers() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec![
                    "Item".into(),
                    "Category".into(),
                    "Item--Price".into(),
                    "Item--Stock".into(),
                    "Item--Location--Province".into(),
                    "Item--Location--Region".into(),
                    "Rating".into(),
                ],
                vec![
                    "A4 Coated Art Paper".into(),
                    "Print Stock".into(),
                    "$24.50".into(),
                    "150".into(),
                    "West Java".into(),
                    "Bekasi".into(),
                    "48".into(),
                ],
                vec![
                    "Pantone Color Bridge".into(),
                    "Tools".into(),
                    "$185.00".into(),
                    "24".into(),
                    "West Java".into(),
                    "Karawang".into(),
                    "49".into(),
                ],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with 3-stack headers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 header rows + 2 data rows = 5 total rows
        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.columns.len(), 7);
        assert_eq!(table.header_rows(), 3);

        // Col 0 ("Item"): 3-row vertical merge
        let c0 = table.cell(0, 0).unwrap();
        assert_eq!(c0.row_span, 3);
        assert_eq!(c0.col_span, 1);
        assert_eq!(c0.text.text.trim(), "Item");
        assert_eq!(c0.vj, designcraft_doc::VerticalJustification::Center);
        assert_eq!(c0.text.paras[0].para.align, Some(designcraft_doc::Align::Center));

        // Col 1 ("Category"): 3-row vertical merge
        let c1 = table.cell(0, 1).unwrap();
        assert_eq!(c1.row_span, 3);
        assert_eq!(c1.col_span, 1);
        assert_eq!(c1.text.text.trim(), "Category");
        assert_eq!(c1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 0 Col 2 ("Item"): 4-column horizontal merge across Cols 2..=5
        let c2_r0 = table.cell(0, 2).unwrap();
        assert_eq!(c2_r0.row_span, 1);
        assert_eq!(c2_r0.col_span, 4);
        assert_eq!(c2_r0.text.text.trim(), "Item");
        assert_eq!(c2_r0.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 2 ("Price"): 2-row vertical merge across Rows 1..=2
        let c2_r1 = table.cell(1, 2).unwrap();
        assert_eq!(c2_r1.row_span, 2);
        assert_eq!(c2_r1.col_span, 1);
        assert_eq!(c2_r1.text.text.trim(), "Price");
        assert_eq!(c2_r1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 3 ("Stock"): 2-row vertical merge across Rows 1..=2
        let c3_r1 = table.cell(1, 3).unwrap();
        assert_eq!(c3_r1.row_span, 2);
        assert_eq!(c3_r1.col_span, 1);
        assert_eq!(c3_r1.text.text.trim(), "Stock");
        assert_eq!(c3_r1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 1 Col 4 ("Location"): 2-column horizontal merge across Cols 4..=5
        let c4_r1 = table.cell(1, 4).unwrap();
        assert_eq!(c4_r1.row_span, 1);
        assert_eq!(c4_r1.col_span, 2);
        assert_eq!(c4_r1.text.text.trim(), "Location");
        assert_eq!(c4_r1.vj, designcraft_doc::VerticalJustification::Center);

        // Row 2 Col 4 ("Province"): 1x1 leaf cell
        let c4_r2 = table.cell(2, 4).unwrap();
        assert_eq!(c4_r2.row_span, 1);
        assert_eq!(c4_r2.col_span, 1);
        assert_eq!(c4_r2.text.text.trim(), "Province");
        assert_eq!(c4_r2.vj, designcraft_doc::VerticalJustification::Center);

        // Row 2 Col 5 ("Region"): 1x1 leaf cell
        let c5_r2 = table.cell(2, 5).unwrap();
        assert_eq!(c5_r2.row_span, 1);
        assert_eq!(c5_r2.col_span, 1);
        assert_eq!(c5_r2.text.text.trim(), "Region");
        assert_eq!(c5_r2.vj, designcraft_doc::VerticalJustification::Center);

        // Col 6 ("Rating"): 3-row vertical merge
        let c6 = table.cell(0, 6).unwrap();
        assert_eq!(c6.row_span, 3);
        assert_eq!(c6.col_span, 1);
        assert_eq!(c6.text.text.trim(), "Rating");
        assert_eq!(c6.vj, designcraft_doc::VerticalJustification::Center);

        // Row 3 (first data row)
        assert_eq!(table.cell(3, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");
        assert_eq!(table.cell(3, 1).unwrap().text.text.trim(), "Print Stock");
        assert_eq!(table.cell(3, 2).unwrap().text.text.trim(), "$24.50");
        assert_eq!(table.cell(3, 3).unwrap().text.text.trim(), "150");
        assert_eq!(table.cell(3, 4).unwrap().text.text.trim(), "West Java");
        assert_eq!(table.cell(3, 5).unwrap().text.text.trim(), "Bekasi");
        assert_eq!(table.cell(3, 6).unwrap().text.text.trim(), "48");
    }

    #[test]
    fn test_execute_insert_table_3stack_headers_with_col_numbers_and_horizontal_rules() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec![
                    "Item".into(),
                    "Category".into(),
                    "Item--Price".into(),
                    "Item--Stock".into(),
                    "Item--Location--Province".into(),
                    "Item--Location--Region".into(),
                    "Rating".into(),
                ],
                vec![
                    "A4 Coated Art Paper".into(),
                    "Print Stock".into(),
                    "$24.50".into(),
                    "150".into(),
                    "West Java".into(),
                    "Bekasi".into(),
                    "48".into(),
                ],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with 3-stack headers + col numbers should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 header rows + 1 col numbers row + 1 data row = 5 total rows
        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.columns.len(), 7);

        // Row 3 is column numbers row: (1), (2), (3), (4), (5), (6), (7)
        for c in 0..7 {
            assert_eq!(table.cell(3, c).unwrap().text.text.trim(), format!("({})", c + 1));
        }

        // Row 4 is first data row
        assert_eq!(table.cell(4, 0).unwrap().text.text.trim(), "A4 Coated Art Paper");

        // Check horizontal rules stroke weights:
        // Top of table (row 0, col 0)
        assert_eq!(table.cell(0, 0).unwrap().strokes[0].weight, 1.0);
        // Sub-header under "Item" (row 0, col 2)
        assert_eq!(table.cell(0, 2).unwrap().strokes[2].weight, 0.5);
        // Sub-header under "Location" (row 1, col 4)
        assert_eq!(table.cell(1, 4).unwrap().strokes[2].weight, 0.5);
        // Bottom of header (row 2, col 0)
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.5);
        // Bottom of column numbers row (row 3, col 0)
        assert_eq!(table.cell(3, 0).unwrap().strokes[2].weight, 0.5);
        // Bottom of table (row 4, col 0)
        assert_eq!(table.cell(4, 0).unwrap().strokes[2].weight, 1.0);
    }

    #[test]
    fn test_detect_footnote_row() {
        let rows = vec![
            vec!["Header 1".into(), "Header 2".into()],
            vec!["Data 1".into(), "Data 2".into()],
            vec!["Sources: Karawang Region Government Office".into(), "".into()],
        ];
        assert_eq!(detect_footnote_row(&rows), Some("Sources: Karawang Region Government Office".to_string()));

        let rows_normal = vec![vec!["Header 1".into(), "Header 2".into()], vec!["Data 1".into(), "Data 2".into()]];
        assert_eq!(detect_footnote_row(&rows_normal), None);

        let rows_single_col = vec![vec!["Single 1".into()], vec!["Single 2".into()]];
        assert_eq!(detect_footnote_row(&rows_single_col), None);
    }

    #[test]
    fn test_execute_insert_table_footer_and_footnote() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Item".into(), "Category".into(), "Price".into()],
                vec!["Paper".into(), "Stock".into(), "$20".into()],
                vec!["Pen".into(), "Tools".into(), "$10".into()],
                vec!["Total".into(), "".into(), "$30".into()],
                vec!["Sources: Karawang Region Government Office".into(), "".into(), "".into()],
            ],
            card2_first_row_header: true,
            card2_table_footer: true,
            card2_col_numbers: false, // pin it off: the row indices below count from the header only
            card2_placement: 1,       // New frame
            card2_style: 3,           // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with footer and footnote should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        // 1. Verify table exists and has 4 rows (1 header + 2 data + 1 total) - NOT 5 rows!
        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.columns.len(), 3);

        // Row 3 is footer row ("Total")
        assert_eq!(table.cell(3, 0).unwrap().text.text.trim(), "Total");
        assert_eq!(table.cell(3, 2).unwrap().text.text.trim(), "$30");
        assert_eq!(table.rows[3].kind, designcraft_doc::RowKind::Footer);

        // Verify Horizontal Rules styling on footer row:
        // Top border of footer row (strokes[0]) is rule_thin (0.5)
        assert_eq!(table.cell(3, 0).unwrap().strokes[0].weight, 0.5);
        // Bottom border of preceding row (strokes[2]) is rule_thin (0.5)
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.5);
        // Bottom border of footer row (strokes[2]) is rule_thick (1.0)
        assert_eq!(table.cell(3, 0).unwrap().strokes[2].weight, 1.0);

        // 2. Verify normal text box below table was created with footnote text
        let fn_item = spread
            .items
            .iter()
            .find(|it| {
                if it.id == table_item.id {
                    return false;
                }
                it.text_frame()
                    .and_then(|tf| active.doc.story(tf.story))
                    .is_some_and(|s| s.text.contains("Sources: Karawang Region Government Office"))
            })
            .expect("Footnote text frame should exist on spread");

        // Footnote frame is positioned below table frame
        assert!(fn_item.bounds().y0 >= table_item.bounds().y1);
    }

    #[test]
    fn test_execute_insert_table_3stack_sample_file_with_footer_and_footnote() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        // Load actual sample CSV file
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/sample_3stack_table.csv");
        if !std::path::Path::new(path).exists() {
            return;
        }
        load_card2_csv(&mut st, path);
        assert_eq!(st.card2_matrix.len(), 8);
        assert_eq!(detect_footnote_row(&st.card2_matrix), Some("Sources: Karawang Region Government Office".to_string()));

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with sample_3stack_table.csv should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        // 3 header rows + 1 col numbers row + 5 data rows + 1 total row = 10 rows
        assert_eq!(table.rows.len(), 10);
        assert_eq!(table.columns.len(), 7);

        // Row 9 is the Total row (footer)
        assert_eq!(table.cell(9, 0).unwrap().text.text.trim(), "Total");
        assert_eq!(table.cell(9, 2).unwrap().text.text.trim(), "$298.50");
        assert_eq!(table.cell(9, 3).unwrap().text.text.trim(), "359");
        assert_eq!(table.rows[9].kind, designcraft_doc::RowKind::Footer);

        // Footer borders: top is thin (0.5), bottom is thick (1.0)
        assert_eq!(table.cell(9, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(9, 0).unwrap().strokes[2].weight, 1.0);

        // Footnote frame exists below table
        let fn_item = spread
            .items
            .iter()
            .find(|it| {
                if it.id == table_item.id {
                    return false;
                }
                it.text_frame()
                    .and_then(|tf| active.doc.story(tf.story))
                    .is_some_and(|s| s.text.contains("Sources: Karawang Region Government Office"))
            })
            .expect("Footnote text frame should exist on spread");

        assert!(fn_item.bounds().y0 >= table_item.bounds().y1);
    }

    #[test]
    fn test_user_reproduction_at_page() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // 1. Generate pages with Card 1
        let st_card1 = PubgenState {
            card1_records: vec![
                vec!["Table".into(), "01.01".into(), "The Rise of Computational Layouts".into()],
                vec!["Table".into(), "01.02".into(), "Typography in the Digital Renaissance and Print Production in the 21st Century".into()],
            ],
            card1_headers: vec!["type".into(), "number".into(), "title".into()],
            card1_type_col: 0,
            card1_num_col: 1,
            card1_title_col: 2,
            card1_range_all: true,
            card1_preset: 0,
            card1_append: true,
            ..Default::default()
        };
        execute_generate_pages(&mut app, &st_card1).unwrap();

        // 2. Insert table with Card 2 at page
        let mut st_card2 = PubgenState {
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_placement: 2, // At page
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plan/pubgen/samples/sample_3stack_table.csv");
        if !std::path::Path::new(path).exists() {
            return;
        }
        load_card2_csv(&mut st_card2, path);

        let res = execute_insert_table(&mut app, &st_card2);
        assert!(res.is_ok(), "insert_table should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.last().unwrap();
        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        for (i, row) in table.rows.iter().enumerate() {
            let col0 = table.cell(i, 0).map(|c| c.text.text.trim().to_string()).unwrap_or_default();
            println!("Row {i} (kind={:?}): col0 = '{col0}'", row.kind);
        }
        assert_eq!(table.rows.len(), 10);

        let cs = designcraft_compose::compose_story(&active.doc, tf.story, &Default::default());
        println!("Composed story overset: {:?}", cs.is_overset());
        assert!(!cs.is_overset(), "Story must not be overset! All rows must fit in table frame");
        if let Some(frame) = cs.frames.first() {
            println!("Frame content_height: {}", frame.content_height);
            if let Some(tfrag) = frame.tables.first() {
                println!("TableFrag rect: {:?}", tfrag.rect);
                println!("TableFrag placed cells count: {}", tfrag.cells.len());
                let placed_rows: std::collections::BTreeSet<_> = tfrag.cells.iter().map(|c| c.row).collect();
                println!("Placed rows in frame: {:?}", placed_rows);
                assert_eq!(placed_rows.len(), 10, "All 10 rows must be placed in the frame");
                for r in 0..10 {
                    assert!(placed_rows.contains(&r), "Row {r} should be placed in the frame");
                }
            }
        }
    }

    #[test]
    fn test_matches_pubgen_row_condition_keywords() {
        assert!(matches_pubgen_row_condition("Jumlah"));
        assert!(matches_pubgen_row_condition("JUMLAH / TOTAL"));
        assert!(matches_pubgen_row_condition("Jumlah:"));
        assert!(matches_pubgen_row_condition("Tahun 2005"));
        assert!(matches_pubgen_row_condition("TAHUN"));
        assert!(matches_pubgen_row_condition("- Jumlah Subtotal"));
        assert!(matches_pubgen_row_condition("Total"));
        assert!(matches_pubgen_row_condition("Sum"));
        assert!(matches_pubgen_row_condition("Subtotal"));
        assert!(matches_pubgen_row_condition("Year 2026"));

        // Negative tests (Kecamatan / Kabupaten / Desa are region names, not summary break rows)
        assert!(!matches_pubgen_row_condition("Kab. Karawang"));
        assert!(!matches_pubgen_row_condition("Kabupaten Karawang"));
        assert!(!matches_pubgen_row_condition("Kec. Klari"));
        assert!(!matches_pubgen_row_condition("Kecamatan Klari"));
        assert!(!matches_pubgen_row_condition("District 1"));
        assert!(!matches_pubgen_row_condition("Subdistrict"));
        assert!(!matches_pubgen_row_condition("Kabel Listrik"));
        assert!(!matches_pubgen_row_condition("Kecap"));
        assert!(!matches_pubgen_row_condition("Tahunan"));
        assert!(!matches_pubgen_row_condition("Meninggal Dunia"));
        assert!(!matches_pubgen_row_condition(""));
    }

    #[test]
    fn test_execute_insert_table_row_conditions_kbda_2005() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut st = PubgenState {
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false, // Notice table_footer is FALSE, row condition formats it!
            card2_row_conditions: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let content = "\
Keadaan Korban (Condition of Victims);Persentase 2005 (%);Korban 2005 (Orang);Persentase 2006 (%);Korban 2006 (Orang)
Meninggal Dunia (Died);8,96;10;42,54;77
Luka Berat (Badly Injured);28,89;33;34,81;63
Luka Ringan (Slightly Injured);62,15;70;22,65;41
JUMLAH / TOTAL;100,00;113;100,00;181";
        st.card2_delim = Some(detect_delimiter(content));
        st.card2_matrix = parse_csv_content_with_delim(content, detect_delimiter(content));
        assert_eq!(st.card2_matrix.len(), 5);
        assert_eq!(st.card2_delim, Some(';'));

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert_table with kbda_2005.csv should succeed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();

        let table_item = spread
            .items
            .iter()
            .find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty()))
            .expect("Table item should exist on spread");

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().expect("Table should exist in story");

        assert_eq!(table.rows.len(), 5);
        assert_eq!(table.columns.len(), 5);

        // Row 0: Header
        assert_eq!(table.cell(0, 0).unwrap().strokes[0].weight, 1.0, "Top of table rule 1.0pt");
        assert_eq!(table.cell(0, 0).unwrap().strokes[2].weight, 0.5, "Header bottom rule 0.5pt");

        // Row 1: Data row 1 ("Meninggal Dunia")
        assert_eq!(table.cell(1, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(1, 0).unwrap().strokes[2].weight, 0.0, "Interior data row unbordered");

        // Row 2: Data row 2 ("Luka Berat")
        assert_eq!(table.cell(2, 0).unwrap().strokes[0].weight, 0.0);
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.0);

        // Row 3: Data row 3 ("Luka Ringan")
        assert_eq!(table.cell(3, 0).unwrap().strokes[0].weight, 0.0);
        // Bottom of Row 3 is top of Row 4 (0.5pt)!
        assert_eq!(table.cell(3, 0).unwrap().strokes[2].weight, 0.5);

        // Row 4: JUMLAH / TOTAL
        assert_eq!(table.cell(4, 0).unwrap().text.text.trim(), "JUMLAH / TOTAL");
        assert_eq!(table.cell(4, 0).unwrap().strokes[0].weight, 0.5, "Row 4 top border 0.5pt");
        assert_eq!(table.cell(4, 0).unwrap().strokes[2].weight, 1.0, "Row 4 bottom border 1.0pt (bottom of table)");
    }

    #[test]
    fn test_execute_insert_table_intermediate_condition_rows() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Kategori".into(), "Jumlah Kasus".into()],
                vec!["Tahun 2005".into(), "100".into()], // Condition row 1 (intermediate)
                vec!["Penyakit A".into(), "40".into()],
                vec!["Penyakit B".into(), "60".into()],
                vec!["Jumlah Subtotal".into(), "100".into()], // Condition row 2 (intermediate)
                vec!["Tahun 2006".into(), "150".into()],      // Condition row 3 (intermediate)
                vec!["Penyakit A".into(), "70".into()],
                vec!["Jumlah Total".into(), "250".into()], // Condition row 4 (last row)
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false,
            card2_row_conditions: true,
            card2_placement: 1,
            card2_style: 3, // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok());

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();

        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        assert_eq!(table.rows.len(), 8);

        // Row 1: "Tahun 2005" (intermediate condition row)
        assert_eq!(table.cell(1, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(1, 0).unwrap().strokes[2].weight, 0.5);

        // Row 2: "Penyakit A" (normal data row)
        assert_eq!(table.cell(2, 0).unwrap().strokes[0].weight, 0.5); // from row 1 bottom
        assert_eq!(table.cell(2, 0).unwrap().strokes[2].weight, 0.0); // unbordered

        // Row 4: "Jumlah Subtotal" (intermediate condition row)
        assert_eq!(table.cell(4, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(4, 0).unwrap().strokes[2].weight, 0.5);

        // Row 7: "Jumlah Total" (last row)
        assert_eq!(table.cell(7, 0).unwrap().strokes[0].weight, 0.5);
        assert_eq!(table.cell(7, 0).unwrap().strokes[2].weight, 1.0);
    }

    #[test]
    fn test_is_numeric_or_dash() {
        // Dashes and missing data symbols
        assert!(is_numeric_or_dash("-"));
        assert!(is_numeric_or_dash("—"));
        assert!(is_numeric_or_dash("–"));
        assert!(is_numeric_or_dash("- "));
        assert!(is_numeric_or_dash("--"));
        assert!(is_numeric_or_dash("−"));
        assert!(is_numeric_or_dash("..."));
        assert!(is_numeric_or_dash("…"));
        assert!(is_numeric_or_dash("N/A"));
        assert!(is_numeric_or_dash("n/a"));
        assert!(is_numeric_or_dash("NA"));
        assert!(is_numeric_or_dash("na"));

        // Integers and floats
        assert!(is_numeric_or_dash("0"));
        assert!(is_numeric_or_dash("7"));
        assert!(is_numeric_or_dash("4.02"));
        assert!(is_numeric_or_dash("1.65"));
        assert!(is_numeric_or_dash("-12.5"));
        assert!(is_numeric_or_dash("−12.5"));
        assert!(is_numeric_or_dash("100"));

        // Accounting parentheses
        assert!(is_numeric_or_dash("(12.5)"));
        assert!(is_numeric_or_dash("(100)"));

        // Thousands and European / Indonesian formats
        assert!(is_numeric_or_dash("1,234.56"));
        assert!(is_numeric_or_dash("1.234,56"));
        assert!(is_numeric_or_dash("1,000"));

        // Percentages
        assert!(is_numeric_or_dash("21.32%"));
        assert!(is_numeric_or_dash("100%"));

        // Text strings (must return false)
        assert!(!is_numeric_or_dash(""));
        assert!(!is_numeric_or_dash("   "));
        assert!(!is_numeric_or_dash("Warungbambu"));
        assert!(!is_numeric_or_dash("Desa / Village"));
        assert!(!is_numeric_or_dash("Jumlah"));
        assert!(!is_numeric_or_dash("Total"));
        assert!(!is_numeric_or_dash("Plawad"));
    }

    #[test]
    fn test_execute_insert_table_header_and_col_alignments() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Desa/Kelurahan".into(), "Luas Wilayah".into(), "Persentase".into(), "Keterangan".into()],
                vec!["Warungbambu".into(), "1.65".into(), "21.32".into(), "Valid".into()],
                vec!["Kondangjaya".into(), "2.44".into(), "8.84".into(), "Valid".into()],
                vec!["Tegalsawah".into(), "-".into(), "13.48".into(), "Special".into()],
                vec!["Total".into(), "4.09".into(), "43.64".into(), "Summary".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_placement: 1, // New frame
            card2_style: 3,     // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 1 header row + 1 column number row + 4 data rows = 6 rows
        assert_eq!(table.rows.len(), 6);
        assert_eq!(table.columns.len(), 4);

        // Row 0: Table Header -> all cells must be Center aligned and VerticalJustification::Center
        for c in 0..4 {
            let cell = table.cell(0, c).unwrap();
            assert_eq!(cell.vj, designcraft_doc::VerticalJustification::Center, "Header col {c} vj should be Center");
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Header col {c} align should be Center");
        }

        // Row 1: Column number row (1), (2), (3), (4) -> all cells must be Center aligned
        for c in 0..4 {
            let cell = table.cell(1, c).unwrap();
            assert_eq!(cell.vj, designcraft_doc::VerticalJustification::Center, "Col number {c} vj should be Center");
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Col number {c} align should be Center");
        }

        // Data rows (rows 2..6):
        // Col 0: Column 1 (index 0) is excluded from centering -> align should NOT be Center
        for r in 2..6 {
            let cell = table.cell(r, 0).unwrap();
            assert_ne!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 0 should remain left aligned");
        }

        // Col 1: Majority numeric / dash ("1.65", "2.44", "-", "4.09") -> must be Center aligned
        for r in 2..6 {
            let cell = table.cell(r, 1).unwrap();
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 1 should be Center aligned");
        }

        // Col 2: All numeric ("21.32", "8.84", "13.48", "43.64") -> must be Center aligned
        for r in 2..6 {
            let cell = table.cell(r, 2).unwrap();
            assert_eq!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 2 should be Center aligned");
        }

        // Col 3: Text descriptions ("Valid", "Special", "Summary") -> must NOT be Center aligned
        for r in 2..6 {
            let cell = table.cell(r, 3).unwrap();
            assert_ne!(cell.text.paras[0].para.align, Some(designcraft_doc::Align::Center), "Data row {r} col 3 should remain left aligned");
        }
    }

    #[test]
    fn test_execute_insert_table_color_theme_default() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Kecamatan".into(), "2021".into(), "2022".into(), "2025".into()],
                vec!["Cikampek".into(), "10".into(), "20".into(), "30".into()],
                vec!["Klari".into(), "15".into(), "25".into(), "35".into()],
                vec!["Karawang".into(), "25".into(), "45".into(), "65".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_color_theme: 0, // Default: #FFCB56 - #FFE9A9
            card2_placement: 1,   // New frame
            card2_style: 3,       // Horizontal Rules
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 1 header row + 1 col numbers row + 3 data rows (last is footer) = 5 rows
        assert_eq!(table.rows.len(), 5);

        // Header row (row 0) must have fill == "#FFCB56"
        for c in 0..4 {
            let cell = table.cell(0, c).unwrap();
            assert_eq!(cell.fill, "#FFCB56");
            assert_eq!(cell.fill_tint, 1.0);
        }

        // Col number row (row 1) must have fill == "#FFE9A9"
        for c in 0..4 {
            let cell = table.cell(1, c).unwrap();
            assert_eq!(cell.fill, "#FFE9A9");
            assert_eq!(cell.fill_tint, 1.0);
        }

        // Interior data rows (rows 2, 3) must have uncolored fill
        for r in 2..4 {
            for c in 0..4 {
                let cell = table.cell(r, c).unwrap();
                assert_eq!(cell.fill, designcraft_color::swatch::NONE);
            }
        }

        // Footer row (row 4) must have fill == "#FFCB56"
        for c in 0..4 {
            let cell = table.cell(4, c).unwrap();
            assert_eq!(cell.fill, "#FFCB56");
            assert_eq!(cell.fill_tint, 1.0);
        }

        // Swatches must be registered in doc
        assert!(active.doc.swatches.iter().any(|s| s.name == "#FFCB56"));
        assert!(active.doc.swatches.iter().any(|s| s.name == "#FFE9A9"));
    }

    #[test]
    fn test_execute_insert_table_color_theme_metadata_override() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["theme".into(), "#F3D2AF - #F8EADC".into()],
                vec!["Kecamatan".into(), "2021".into(), "2025".into()],
                vec!["Cikampek".into(), "10".into(), "30".into()],
                vec!["Karawang".into(), "25".into(), "65".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_color_theme: 0, // default is 0, but metadata overrides to #F3D2AF - #F8EADC
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 1 header row + 1 col numbers row + 2 data rows = 4 rows
        assert_eq!(table.rows.len(), 4);

        // Header row must have #F3D2AF
        assert_eq!(table.cell(0, 0).unwrap().fill, "#F3D2AF");
        // Col number row must have #F8EADC
        assert_eq!(table.cell(1, 0).unwrap().fill, "#F8EADC");
        // Interior data row must have NONE
        assert_eq!(table.cell(2, 0).unwrap().fill, designcraft_color::swatch::NONE);
        // Footer row must have #F3D2AF
        assert_eq!(table.cell(3, 0).unwrap().fill, "#F3D2AF");
    }

    #[test]
    fn test_execute_insert_table_bilingual_english_italic() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let st = PubgenState {
            card2_matrix: vec![
                vec!["Kecamatan / District".into(), "Luas / Area".into(), "Keterangan\nDescription".into()],
                vec!["Cikampek".into(), "10".into(), "Aktif / Active".into()],
                vec!["Karawang".into(), "25".into(), "Selesai / Finished".into()],
            ],
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // Check Header Cell (0, 0): "Kecamatan / District"
        let cell_0_0 = table.cell(0, 0).unwrap();
        assert_eq!(cell_0_0.text.text, "Kecamatan / District");
        // "Kecamatan / " is upright (not italic)
        let fmt_id = cell_0_0.text.char_format_of(0);
        assert_ne!(fmt_id.over.font_style.as_deref(), Some("Italic"));
        assert_ne!(fmt_id.over.font_style.as_deref(), Some("Bold Italic"));
        // "District" (starting at byte 12) must be Italic / Bold Italic
        let fmt_en = cell_0_0.text.char_format_of(12);
        let en_style = fmt_en.over.font_style.as_deref().unwrap_or("");
        assert!(en_style.contains("Italic"), "English text 'District' must be italic, got: {en_style}");

        // Check Data Cell (1, 2): "Aktif / Active"
        let cell_1_2 = table.cell(1, 2).unwrap();
        assert_eq!(cell_1_2.text.text, "Aktif / Active");
        let fmt_aktif = cell_1_2.text.char_format_of(0);
        assert_ne!(fmt_aktif.over.font_style.as_deref(), Some("Italic"));
        let fmt_active = cell_1_2.text.char_format_of(8);
        let active_style = fmt_active.over.font_style.as_deref().unwrap_or("");
        assert!(active_style.contains("Italic"), "English text 'Active' must be italic, got: {active_style}");

        // Check Cell with line break (0, 2): "Keterangan\u{2028}Description"
        let cell_0_2 = table.cell(0, 2).unwrap();
        let desc_byte = cell_0_2.text.text.find("Description").unwrap();
        let fmt_desc = cell_0_2.text.char_format_of(desc_byte);
        let desc_style = fmt_desc.over.font_style.as_deref().unwrap_or("");
        assert!(desc_style.contains("Italic"), "Line 2 English text 'Description' must be italic, got: {desc_style}");

        // One profile font for every cell
        let fam = st.body_font();
        for (r, c, pos) in [(0usize, 0usize, 0usize), (1, 2, 0), (0, 2, desc_byte)] {
            let cell = table.cell(r, c).unwrap();
            assert_eq!(
                cell.text.char_format_of(pos).over.font_family.as_deref(),
                Some(fam.as_str()),
                "Cell ({r},{c}) must use the profile body font"
            );
        }
    }

    #[test]
    fn test_execute_insert_table_with_theme_from_div_csv() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        // Simulate CSV generated with theme row from DIV sheet (table_theme: #9E7EB5 - #C4A4DA)
        let matrix = vec![
            vec!["Kecamatan / District".into(), "Luas / Area".into()],
            vec!["Karawang Timur".into(), "32.5".into()],
            vec!["Jumlah / Total".into(), "32.5".into()],
            vec!["theme".into(), "#9E7EB5 - #C4A4DA".into()],
        ];

        let st = PubgenState {
            card2_matrix: matrix,
            card2_first_row_header: true,
            card2_col_numbers: true,
            card2_table_footer: true,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };

        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "Table insertion failed: {:?}", res);

        let active = app.session.active().unwrap();
        let spread = active.doc.spreads.first().unwrap();
        let table_item =
            spread.items.iter().find(|it| it.text_frame().and_then(|tf| active.doc.story(tf.story)).is_some_and(|s| !s.tables.is_empty())).unwrap();
        let tf = table_item.text_frame().unwrap();
        let story = active.doc.story(tf.story).unwrap();
        let table = story.tables.values().next().unwrap();

        // 4 rows in table (metadata theme row was stripped):
        // Row 0: Header (fill = #9E7EB5)
        // Row 1: Col numbers (fill = #C4A4DA)
        // Row 2: Data row (fill = NONE)
        // Row 3: Footer row (fill = #9E7EB5)
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.cell(0, 0).unwrap().fill, "#9E7EB5");
        assert_eq!(table.cell(1, 0).unwrap().fill, "#C4A4DA");
        assert_eq!(table.cell(2, 0).unwrap().fill, designcraft_color::swatch::NONE);
        assert_eq!(table.cell(3, 0).unwrap().fill, "#9E7EB5");
    }

    #[test]
    fn test_execute_insert_table_automatically_threads_long_table() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();

        let mut matrix = vec![vec!["Kecamatan".into(), "Luas".into()]];
        for i in 0..300 {
            matrix.push(vec![format!("Kecamatan {i}"), format!("{}", i * 7)]);
        }
        let st = PubgenState {
            card2_matrix: matrix,
            card2_first_row_header: true,
            card2_col_numbers: false,
            card2_table_footer: false,
            card2_placement: 1,
            card2_style: 3,
            ..Default::default()
        };
        let res = execute_insert_table(&mut app, &st);
        assert!(res.is_ok(), "insert failed: {:?}", res.err());

        let sid = {
            let a = app.session.active().unwrap();
            let mut found = None;
            'outer: for sp in &a.doc.spreads {
                for it in &sp.items {
                    if let Some(tf) = it.text_frame()
                        && a.doc.story(tf.story).is_some_and(|s| !s.tables.is_empty())
                    {
                        found = Some(tf.story);
                        break 'outer;
                    }
                }
            }
            found.unwrap()
        };

        assert!(!story_is_overset(&app, sid), "Table must fit after automatically threading onto continuation pages");
        let pages_after = app.session.active().unwrap().doc.page_count();
        assert!(pages_after > 1, "Continuation pages must be added: {pages_after}");
    }
}
