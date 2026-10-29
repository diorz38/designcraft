# PubGen Menu Accessibility Audit

Date: 2026-10-30
Scope: all PubGen functions vs main menu reachability
Files reviewed: `crates/ui-egui/src/menus/pubgen_bar.rs`, `crates/ui-egui/src/panels/pubgen/mod.rs`

## Main Menu Structure

PubGen replaces DesignCraft menubar with 4 top-level menus: **File**, **Publication**, **PubGen Tools**, **Help**. Toggle off → all PubGen actions collapse under single **Pubgen** menu in DesignCraft bar.

## Functions Available from Main Menu

| Menu | Item | State flag set |
|---|---|---|
| Publication | Master Excel Import & Generate… | `dialog_excel_open = true` |
| Publication | Publication Compile Wizard… | `dialog_excel_open = true` (non-wasm) |
| Publication | Statistical Vector Chart… | `chart_dialog = Some(...)` |
| Publication | Table of Contents… | `dialog_toc_open = true` |
| Publication | Publication Cover… | `dialog_cover_open = true` |
| Publication | Header & Footer Setup… | `dialog_hf_open = true` |
| Publication | Audit Publication… | `audit_open = true` |
| PubGen Tools ▸ Manual Ingest & Tables | Page from CSV… | `dialog_manual_open = true, tab = 0` |
| PubGen Tools ▸ Manual Ingest & Tables | Table from CSV… | `dialog_manual_open = true, tab = 1` |
| PubGen Tools ▸ Page & Layout Utilities | Set Standard BPS Margins | direct execute |
| PubGen Tools ▸ Page & Layout Utilities | Page Margins & Renumbering Options… | `dialog_utils_open = true` |
| PubGen Tools | Dual Language Mode (toggle) | `bilingual_output = !bilingual_output` |
| PubGen Tools ▸ AI Translate | Translate Table/Review on this Page | `ai::execute_translate_selected_page` |
| PubGen Tools ▸ AI Translate | Translate Table Pages… | `show_table_range_popup = true` |
| PubGen Tools ▸ AI Translate | Fix Terms with Glossary | `ai::execute_fix_terms` |
| PubGen Tools ▸ AI Translate | Setup… | `ai_dialog = Some(...)` |
| PubGen (DC mode) | About PubGen… | `show_about_dialog = true` |
| PubGen (DC mode) | Show DesignCraft Menus / Show PubGen Menus | toggle `pubgen_menus_on` |

## Panel-Only Cards (No Separate Menu Entry)

These render inline in panel `show()` (`mod.rs:211-398`). No dedicated menu item opens them directly; menu items set state flags that the panel reads to render dialogs.

| Card | Notes |
|---|---|
| Compile Card (`compile_card_ui`) | Always visible in panel (even no-doc). Menu "Compile Wizard" sets `dialog_excel_open`, not open compile card directly. |
| Master Excel UI (`master_excel_ui`) | File picker, style/theme selector, generate button. Menu only sets `dialog_excel_open = true`. |
| TOC Card (`card_toc_ui`) | Dedicated section in panel. Menu only sets `dialog_toc_open = true`. |
| Manual Import Accordion (`manual_import_accordion_ui`) | Collapsible in panel. Menu only sets `dialog_manual_open`. |
| Page Utilities (`page_utilities_ui`) | Bottom card in panel. |
| Cover Card (`card_cover_body_ui`) | Rendered as panel section, not standalone dialog from menu. |

## Access Pattern

All menu items use state-flag pattern: menu click → set `PubgenState` flag → panel `show()` reads flag → renders dialog/card. Menus never execute logic directly (except `Set Standard BPS Margins` and AI translate actions).

Panel must be open for menu-triggered dialogs to appear. `pubgen_dialogs()` called from `dock_extended.rs:197` and `app.rs:305`.

## Conclusion

No accessibility gap. All PubGen functions reachable from main menu via state-flag → panel-render mechanism. No hidden or panel-exclusive functionality exists without a menu trigger path.

---

# Part 2 — Effektifitas Main Menu vs UX Guidelines

Reference: Nielsen Norman Group, *Menu-Design Checklist: 17 UX Guidelines* (Page Laubheimer, 7 Jun 2024, https://www.nngroup.com/articles/menu-design/), plus platform conventions for desktop creative apps (Adobe InDesign/Photoshop menu anatomy: File / Edit / Layout-Object / Type / View / Window / Help).

## 2.1 Ringkasan

Main menu PubGen saat ini **fungsional dan lengkap** (semua fitur terjangkau), tetapi **kurang efektif** sebagai navigasi aplikasi profesional. Masalah utamanya bukan aksesibilitas, melainkan **anatomi informasi**: 4 menu top-level dengan isi yang menimpa konvensi desktop, item setinggi 18, dan pola "menu → set flag → panel render" yang membuat menu tidak pernah mengeksekusi apa pun secara langsung.

## 2.2 Perbandingan dengan Konvensi Aplikasi Desktop

| Aspek | Konvensi (InDesign/Photoshop/Illustrator) | PubGen saat ini | Verdict |
|---|---|---|---|
| Menu top-level | File, Edit, Layout, Type, Object, View, Window, Help (7–9) | File, Publication, PubGen Tools, Help (4) | ⚠ Menyembunyikan Edit/Layout/Type/Object/View/Window — padahal ini host app |
| Pengelompokan | Berdasarkan **objek yang dioperasikan** (Object, Type, Layout) | Berdasarkan **alur kerja pubgen** (Publication = pipeline) | ⚠ Dua mental model bercampur |
| Item per menu | 8–20, dengan separator & submenu bertingkat | Publication 8, PubGen Tools 14 (sebagian di submenu) | ✓ Wajar |
| Item yang tidak tersedia | Greyed-out (disabled) tetap terlihat untuk mengajarkan fitur | `enabled = has_doc` — item gelap | ✓ Baik |
| Shortcut | Hampir semua item punya accelerator | **0 shortcut** | ✗ Tidak ada |
| Icon | Kini lazim (macOS SF Symbols, Windows) | Tidak ada | ⚠ Opsional |
| Eksekusi langsung | Menu item menjalankan aksi | Menu hanya set state flag → panel render | ⚠ Melanggar §16 (lihat 2.3) |
| Toggle state | Checkmark di item | Hanya "Dual Language" pakai "✓ " prefix | ⚠ Hanya 1 dari ≥2 toggle |
| Menu duplikat | Tidak ada | `about()` muncul di Publication **dan** PubGen | ✗ Duplikasi |
| Route ganda | Satu aksi = satu jalur | 3 dari 18 item set flag yang sama (`dialog_excel_open`) | ✗ Ambigu |
| Kecepatan akses | Via Fitts's law: item terpenting di dekat trigger | "About" & toggle di bawah; pipeline di atas | ✓ |

## 2.3 Pelanggaran NN/G §1–17

| Rule | Status | Detail |
|---|---|---|
| §7 Wording jelas & familiar | ⚠ | "PubGen Tools" = jargon internal; "Publication" vs "PubGen" membingungkan user (dua nama produk). Item "Master Excel Import & Generate…" panjang, tidak front-loaded. |
| §8 Label mudah dipindai | ✗ | "Set Standard BPS Margins (20/20/19/15 mm)" terlalu panjang, angka teknis mendominasi. Tidak ada left-aligned icon column. |
| §11 Target klik cukup besar | ⚠ | Row `egui::Button` mengikuti spacing theme; tanpa padding eksplisit tinggi item bisa <24px di skala kecil. |
| §12 Caret submenu | ⚠ | Bergantung `crate::menus::menu_button` upstream; tidak ada penanda konsisten antara item aksi vs submenu. |
| §13 Click-activated | ✓ | `menu_button` egui = click-activated, bukan hover. |
| §14 Hindari cascading >2 level | ✓ | Maksimal 2 level (Publication → item; PubGen Tools → submenu → item). |
| §16 Fitts's law — command sering di dekat trigger | ✗ | Menu = satu-satunya jalur; tidak ada toolbar/ribbon untuk aksi pamungkas (Generate, Compile). Setiap generate butuh: buka menu → klik → panel terbuka → klik lagi. 2–3 hop untuk aksi primer. |
| §17 Hindari pola gimmicky | ⚠ | Toggle "Show DesignCraft Menus" adalah escape hatch ke menubar normal — pola bagus, tapi menempatkan seluruh menubar alternatif di tempat tersembunyi (menu) melanggar §1 "jangan sembunyikan navigasi di desktop". |
| §1 Jangan sembunyikan navigasi | ✗ | Saat mode PubGen aktif, 6 menu host (Edit, Layout, Type, Object, View, Window) hilang total. User kehilangan akses ke fitur host yang bukan bagian pubgen — risiko kehilangan fitur (bukan sekedar tersembunyi). |
| §5 Indikasi lokasi user | ✗ | Tidak ada indikasi menu mana yang "aktif" (mode PubGen vs host). Tidak ada breadcrumb/status. |
| §10 Visual cue menu panjang | ✗ | Tidak ada icon / warna group pada menu 8–14 item. |

## 2.4 Temuan Spesifik di Kode

1. **`pubgen_bar.rs:98-103` — "Publication Compile Wizard…"** menyet `dialog_excel_open = true`, sama persis dengan "Master Excel Import & Generate…" di `pubgen_bar.rs:90-95`. Dua label berbeda → dialog sama. User bingung; wizard compile card sebenarnya sudah always-visible di panel (`mod.rs:365-370`).
2. **`pubgen_bar.rs:250-263` — `about()` dirangkai dua kali**: sekali di `pubgen_menu()` (baris 275) dan sekali di `publication_menu()` (baris 288). Home untuk About + toggle menu-bar seharusnya **satu** (Help / PubGen).
3. **`pubgen_bar.rs:181-189` — Dual Language toggle** manual: prefix `"✓ "` tiga spasi — bukan `egui::Checkbox` atau `Button::selected()`. Tidak konsisten dengan toggle lain (mis. `Export AI Cover` di panel yang mungkin pakai checkbox native).
4. **`pubgen_bar.rs:166-169` — "Set Standard BPS Margins (20/20/19/15 mm)"** eksekusi langsung tanpa undo hint; user bisa saja tidak sengaja. Panjang label memuat angka.
5. **Tidak ada accelerator sama sekali**: 18 item menu, 0 shortcut. Item primer seperti Generate, Compile, Export harus diberi shortcut (konvensi: Ctrl+Shift+G → generate, Ctrl+Shift+B → compile, F7 → ToC).
6. **`pubgen_bar.rs:37` — slots `[0,1,2,3]`** menempatkan File di kiri, Help di kanan. Konvensi desktop: Help tetap paling kanan ✓ — tapi tidak ada menu **Window/View** untuk panel toggles.
7. **`pubgen_bar.rs:29-31` — gate `pubgen_menus_on`** default `true` = mode PubGen = menubar host hilang. `about()` di `pubgen_bar.rs:250-256` membuka About dialog → yang memuat toggle untuk kembali. Dua tempat untuk toggle yang sama.

## 2.5 Usulan Penyempurnaan

### P0 — Wajib (risiko kehilangan fungsi & ambiguitas)

| # | Usulan | Alasan (rule / kode) |
|---|---|---|
| P0-1 | **Kembalikan menu host ke bar**: jadikan `Edit, Layout, Type, Object, View, Window` tetap ada di bar, disusulkan setelah PubGen (File · Edit · Layout · Type · Object · View · Window · Publication · PubGen Tools · Help). Atau pakai **mega-menu PubGen** di sisi kanan sebagai *satu* dropdown besar (grouping: Data Source · Layout · AI · Utilities). | §1 jangan sembunyikan navigasi; host tetap reachable |
| P0-2 | **Hapus duplikasi route**: "Publication Compile Wizard…" → buka compile card langsung (scroll/highlight), **bukan** `dialog_excel_open`. Atau hapus item ini dan ganti label "Master Excel Import & Generate…" jadi "Generate from Excel…". | `pubgen_bar.rs:98-103` → flag sama dengan `:90-95` |
| P0-3 | **Satukan `about()`**: hanya di satu menu (disarankan Help → "About PubGen…"), tidak di Publication. | `pubgen_bar.rs:275` & `:288` duplikat |
| P0-4 | **Shortcut untuk aksi primer**: `Ctrl+Shift+G` Generate, `Ctrl+Shift+B` Compile, `F7` ToC, `Ctrl+Shift+M` Set BPS Margins, `Ctrl+Shift+T` AI Translate. Daftarkan di `crate::menus::shortcuts` atau `UI_COMMANDS`. | §16; 0 shortcut saat ini |
| P0-5 | **Ganti label jargon**: "PubGen Tools" → "Publication Tools" atau "Layout Tools"; "Master Excel Import & Generate…" → "Generate from Excel…"; "Set Standard BPS Margins (20/20/19/15 mm)" → "Standard BPS Margins" (angka pindah ke tooltip). | §7–8 front-loading |

### P1 — Sangat dianjurkan (konsistensi & discoverability)

| # | Usulan | Alasan |
|---|---|---|
| P1-1 | **Tambah menu View** (host atau baru) yang berisi toggle panel: Pubgen Panel, ToC Panel, … (mirror `DOCK_TABS`). User bisa restore panel jika tertutup. | §1; sekarang panel hanya bisa dibuka lewat docking atau panel tab |
| P1-2 | **Tambahkan undo hint** di item "Standard BPS Margins": eksekusi dalam satu undo group + tooltip "Undo with Ctrl+Z". | User bisa tidak sengaja; status bar feedback |
| P1-3 | **Ganti toggle manual "✓ "** dengan `egui::Button::selected()` atau `Checkbox` di submenu, konsisten dengan toggle lain. | §7 pattern familiarity; `pubgen_bar.rs:181-189` |
| P1-4 | **Gunakan `MenuItem` dengan icon** di menu (aksi: ▶, dialog: …, toggle: ✓, submenu: ▸). | §10 visual cue; §12 caret |
| P1-5 | **Item disabled diberi tooltip** "Open a document first" agar user tahu kenapa disabled. | §7 information scent |
| P1-6 | **Status/dot indicator** untuk job aktif (compile, AI, cover) di menu Publication (mis. "Compilation running…"). | §5 indikasi status; `poll_active_jobs` ada |

### P2 — Nice to have (polish)

| # | Usulan |
|---|---|
| P2-1 | **Ribbon/toolbar kanan-atas** untuk 3–4 aksi pamungkas (Generate, Compile, Export PDF). Menu jadi jalan sekunder. |
| P2-2 | **Konsolidasi AI Translate** ke submenu dan beri shortcut kontekstual: hanya tampil bila `is_bilingual && backend_ready` aktif. |
| P2-3 | **Menu "Recent"** (Sub-menu File atau Welcome) untuk `pubgen_project.json` terakhir. |
| P2-4 | **Internationalized labels**: saat ini `crate::i18n::tr(lang, "...")` untuk label, tapi menu host (upstream) bawa string sendiri; pastikan catalog lengkap. |

## 2.6 Rekomendasi Anatomi Menu Target

Usulan struktur (prioritas P0+P1):

```
File        Edit     Layout     Type     Object     View      Publication      Publication Tools        Help
│           │        │          │        │          │         │                │                        │
New         Undo     Margins    …host…   …host…     Panel ▸   Generate from    Manual Ingest ▸          Help Topics
Open        Redo     Renumber                Zoom ▸  Excel…           Page from CSV…
Save        Cut      Standard Mesh           …               Table of Contents…   Table from CSV…
Place…      Copy     Margins                         Header & Footer…   Layout Utilities ▸
Revert      Paste                                              Audit Publication…   Standard BPS Margins
Export PDF  Select All                                         Compile ▸            Page Margins & Options…
Exchange…                                                     View Compile Card   Dual Language Mode ✓
                                                              Statistical Chart…   AI Translate ▸
                                                              Publication Cover…    Translate Page
                                                                                   Translate Table Pages…
About PubGen…  ← hanya di Help
```

Prinsip: **host tetap utuh** · **Publication = pipeline sekuensial** (urutan Generate → ToC → Header/Footer → Audit → Compile mengikuti alur kerja nyata) · **Publication Tools = operasi lepas** (manual ingest, utilitas, AI) · **Help = about/shortcut/toggle menu-bar**.

## 2.7 Metrik Keberhasilan

| Metrik | Sekarang | Target |
|---|---|---|
| Aksi primer reachable dalam 1 hop dari menubar | 0 | ≥3 (Generate, Compile, Export) |
| Shortcut terdaftar | 0 | ≥6 |
| Menu host tersembunyi | 6 | 0 |
| Duplikasi route (2 label → 1 dialog) | 1 | 0 |
| Toggle state ditampilkan konsisten | 1/2 | 2/2 |
| Item menu dengan icon/cue | 0% | ≥80% |
