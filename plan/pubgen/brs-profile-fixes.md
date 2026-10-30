# BPS BRS — Perbaikan Profile (2026-10-30)

Profil: `assets/pubgen/profiles/bps_brs.json` (BPS Berita Resmi Statistik).
Empat masalah dilaporkan. Diagnosis sudah dilakukan; belum ada kode yang diubah.

---

## 1. Satu font (Lato) untuk tabel + grafik

**Gejala.** Tabel dan grafik memakai lebih dari satu font (Arial/Raleway/Lato campur).

**Root cause.**
- `typography` di profil adalah **dead config**: didefinisikan di
  `crates/ui-egui/src/panels/pubgen/profiles.rs:239-260` (`body_font`,
  `heading_font`, `title_font` + default fn) tetapi **tidak dibaca** di mana pun.
- Font hardcoded:
  - `panels/pubgen/charts/emitter.rs:316` — `font_family.unwrap_or("Arial")`
  - `panels/pubgen/charts/emitter.rs:390,769` — `Some("Lato")` hanya untuk line chart
  - `panels/pubgen/charts/emitter.rs:832` — `Some("Lato")` untuk tabel data grafik
  - `panels/pubgen/charts/emitter.rs:875` — label `"Raleway"`
  - `panels/pubgen/charts/dialog.rs:630,643,658` — `Some("Arial")`
  - `panels/pubgen/pages.rs:546,568,590` — style paragraph `"Raleway"`
  - `panels/pubgen/pages.rs:618` — `"Lato"`
  - `panels/pubgen/tables.rs` — sel tabel tidak pernah di-set font; ikut
    `[Basic Paragraph]` dokumen.

**Rencana fix.**
1. Helper tunggal:
   ```rust
   fn body_font(st: &PubgenState) -> &str {
       let p = st.active_profile();
       if p.typography.body_font.trim().is_empty() { "Lato" } else { &p.typography.body_font }
   }
   ```
   Sesuaikan dengan API `active_profile()` yang sudah ada.
2. Pakai helper di `charts/emitter.rs`: ganti semua `unwrap_or("Arial")` dan
   `Some("Lato")`/`Some("Raleway")` dengan font profil. `emit_text_with_font`
   terima `Option<&str>` — kirim `Some(body_font(st))` dari call site yang punya `st`;
   kalau call site tidak punya `st`, teruskan lewat parameter.
3. `charts/dialog.rs` — ganti ketiga `"Arial"` dengan font profil.
4. `tables.rs` — setelah `table.convertFromText` (`:412` sudah pegang `&mut table`),
   set `font_family` semua sel + `table.options` bila ada field font teori.
5. Set `bps_brs.json` `typography`:
   ```json
   "body_font": "Lato", "heading_font": "Lato", "title_font": "Lato"
   ```
   (Saat ini `heading_font`/`title_font` = `Raleway`.)
6. `pages.rs:546-590` style paragraph H1/H2 — ganti `"Raleway"` dengan font profil.

**Test.** Regresi: `covers.rs:3202` sudah assert `Lato` untuk body paragraph.
Tambahkan assert serupa untuk sel tabel dan chart emitter.

---

## 2. Tabel lebih dari satu halaman (tidak muat 1 halaman)

**Gejala.** Tabel panjang overset; tidak berpindah ke halaman berikutnya.

**Engine sudah mendukung.** `crates/compose/src/table.rs:331-335`: baris yang tidak
muat lanjut ke kolom/frame berikutnya di thread; header diulang otomatis
(`repeat_header` default `true`, juga `repeat_footer`, di
`crates/doc/src/table/cell.rs:340-341`).

**Root cause.** Pubgen membuat **satu frame tanpa thread** →
`execute_insert_table_matrix` (`tables.rs:232-332`) → overset.

**Rencana fix.** Setelah insert tabel sukses di `pages.rs:1196` dan `pages.rs:2727`,
tambahkan loop overset seperti pola RTF yang sudah ada di `pages.rs:1584-1667`:
1. `story_is_overset(app, sid)` → cek overset.
2. `layout.pages.insert {count: 1}`.
3. Set margin halaman baru = `doc_margins`.
4. `text.thread {from: last_fid, rect: [m.x0, m.y0, m.x1, m.y1], spread}`.
5. Ulangi sampai tidak overset atau batas 50 halaman.

Header tabel otomatis terulang di fragmen lanjutan. Sumber/catatan tetap di
halaman pertama. Batas fisik: satu baris lebih tinggi dari satu halaman tidak
bisa dipindah.

**Catatan.** `execute_insert_table_matrix` mengembalikan `Result<String, String>`;
perlu juga mengembalikan `(story_id, frame_id)` atau helper terpisah supaya loop
overset tahu frame terakhir. Cek dulu apakah info itu bisa diambil dari
`app.session.active()` + `story.tables`.

---

## 3. Judul tabel kehilangan "Tabel X"

**Gejala.** Judul tampil "1.1 Luas Wilayah..." tanpa kicker "Tabel".

**Root cause.** Dua cabang `row_template_slots` di
`crates/ui-egui/src/panels/pubgen/pages.rs`:
- Bilingual (`:515-522`): `table_num = "Tabel 1.1"`, `table_kind_en = "Table 1.1"` ✅
- Non-bilingual (`:523-534`): `table_num = "1.1"`, `table_kind_en = ""` ❌

Template `assets/templates/data_tables/001_single_table_standard.designcraft`
punya slot terpisah: `#slot:text:table_num`, `#slot:text:table_kind_en`,
`#slot:text:table_title`, `#slot:text:table_title_en`, `#slot:text:source`,
`#slot:table:main`. Mode ID single-language mengisi `table_num` tanpa prefix dan
tidak mengisi kicker sama sekali.

**Rencana fix (1 baris).** Cabang non-bilingual `:528`:
```rust
text.insert("table_num".into(), json!(format!(
    "{} {}",
    locale::table_prefix(Some(st.active_profile()), st.locale),
    formatted_num
)));
```
`locale::table_prefix` sudah ada (`locale.rs:124`) dan membaca
`profile.nomenclature.table_prefix` = `"Tabel"` dari `bps_brs.json`.

**Efek ke TOC.** `tables.rs:1018` `find_type_and_number_on_page` scan prefix
`["Grafik","Chart","Tabel","Table"]` — kicker ikut terbawa, konsisten.

---

## 4. Judul tabel terlalu jauh dari tabel; wrap judul harus mendorong tabel

**Gejala.** Gap judul→tabel besar. Judul yang wrap menabrak tabel.

**Template.** `001_single_table_standard.designcraft`:
- `table_title` frame y 70→90 (tinggi tetap 20pt)
- `table_title_en` y 90→110
- tabel `#slot:table:main` y 120→520
- source y 530→555

**Root cause (dua bug).**

**(a) Title frame tidak tumbuh.** `autoSize` off. Judul wrap > 20pt tumpah ke
bawah dan menimpa tabel yang tetap di y=120.

**(b) Tabel diposisikan dari item terbawah di halaman.**
`crates/ui-egui/src/panels/pubgen/tables.rs:258-281`:
```rust
if b.x0 < page_r.x1 && b.x1 > page_r.x0 && (st.active_profile().id == "bps_brs" || b.y0 < margin.y0 + 260.0)
```
Untuk `bps_brs` **semua** item dihitung — termasuk source frame di y=555 →
`table_y0 = 555 + 12` melewati margin bawah → frame tabel jadi sisa 60pt
(`available_h.max(60.0)`). Untuk profil lain cutoff 260pt menyelamatkan.

**Rencana fix.**

**(a)** Di `post_process_template_applied_page` (`pages.rs:180-336`), untuk item
berlabel `#slot:text:table_title` / `table_title_en`:
```rust
app.run("object.textFrameOptions", json!({
    "autoSize": "heightOnly", "autoSizeReference": "topLeft", "ids": [id]
}));
app.run("object.fit", json!({"mode": "fitFrameToContent", "axis": "height", "ids": [id]}));
```
Catatan: `autoSize heightOnly` di engine sudah menangani ini
(`crates/engine/src/autosize.rs:175`) — mungkin cukup `object.textFrameOptions`
tanpa `object.fit`. Verifikasi.

**(b)** Scan `title_bottom` di `tables.rs` hanya dari item berlabel
`#slot:text:table_*`. Fallback scan lama hanya untuk halaman non-template
(tidak ada slot label). Gap 12 → 6pt.

**(c) Sumber tetap.** `note_reserve_h` (`tables.rs:273-283`) hanya menghitung
footnote/notes dari CSV. Slot `#slot:text:source` template berada di y=530 tetap.
Kalau tabel panjang sampai y=530, tabrak. Perlu: geser source frame ke bawah
tabel hasil (`table_y0 + frame_h + 6`) setelah insert, atau reserve tinggi
source. Perlu diputuskan saat implementasi.
***Keputusan***: geser source frame ke bawah tabel hasil

**Test.** Regresi: judul panjang (3 baris) → assert `title_frame.bounds().y1 <
table_frame.bounds().y0` di `tables.rs` test module.

---

## Urutan implementasi yang disarankan

1. **#3** — 1 baris, risiko nol, verifikasi cepat.
2. **#4** — terisolasi di `tables.rs` + `post_process_template_applied_page`.
3. **#1** — plumb font profil ke emitter + sel tabel + JSON profil.
4. **#2** — butuh refactor kecil return value `execute_insert_table_matrix`.
   Paling besar; kerjakan terakhir.

## File yang disentuh

- `assets/pubgen/profiles/bps_brs.json`
- `crates/ui-egui/src/panels/pubgen/pages.rs`
- `crates/ui-egui/src/panels/pubgen/tables.rs`
- `crates/ui-egui/src/panels/pubgen/charts/emitter.rs`
- `crates/ui-egui/src/panels/pubgen/charts/dialog.rs`

## Referensi kode (verifikasi sebelum edit)

- `locale::table_prefix` — `crates/ui-egui/src/panels/pubgen/locale.rs:124`
- `row_template_slots` — `pages.rs:350`
- `post_process_template_applied_page` — `pages.rs:180`
- `execute_insert_table_matrix` — `tables.rs:25`
- `title_bottom` scan — `tables.rs:258-281`
- `emit_text_with_font` — `charts/emitter.rs:288`
- `AutoSize` — `crates/doc/src/item/text_frame.rs:30`
- Table fragmentation — `crates/compose/src/table.rs:331`
- Overset-loop pattern (RTF) — `pages.rs:1584-1667`
