## 10. Publication Compiler (Pengikat Multi-Bab)

Kartu **Publication Compiler** di bagian atas panel membangun seluruh publikasi dari satu manifest. Daripada membuka sepuluh buku kerja bab satu per satu, Anda mendeskripsikannya sekali di `pubgen_project.json` lalu menekan **Compile**.

- **Manifest** — ketik jalurnya atau pakai **Open…**. Manifest terakhir yang dibuka diingat dan dipakai saat kolom kosong.
- **New…** — formulir pembuat proyek: judul publikasi, mode dwibahasa, jalur simpan, gaya dan tema sampul, daftar bab, margin, penomoran halaman, dan ekspor PDF. Hasilnya `pubgen_project.json` yang siap dijalankan.
- **Compile** — menjalankan alur dengan bilah progres dan tombol **Cancel** yang berfungsi. Tanda titik menandai baris menu Publication selama proses berjalan.
- Build yang gagal atau dibatalkan tidak menyentuh dokumen sebelumnya: berkas hasil disimpan pada langkah kedua terakhir. Dokumen yang sudah terbuka di tab tidak pernah ditimpa — build dilewati.

### Manifest

```json
{
  "version": 1,
  "publicationTitle": "Kecamatan Karawang Timur Dalam Angka 2026",
  "bilingual": true,
  "profile": "bps_dda",
  "output": {
    "document": "build/karawang-timur.designcraft",
    "pdf": "build/karawang-timur.pdf"
  },
  "document": {
    "preset": "A4",
    "facingPages": true,
    "bleedMm": 3,
    "marginsMm": { "top": 20.2, "bottom": 20, "inside": 19, "outside": 15 }
  },
  "cover": {
    "source": "workbooks/BAB_01_2025.xlsx",
    "style": 0,
    "themePreset": 0,
    "includeInside": true,
    "includeBack": true
  },
  "frontMatter": { "workbook": "" },
  "chapters": [
    { "chapter": 1, "path": "workbooks/BAB_01_2025.xlsx", "title": "Geografi" },
    { "chapter": 2, "path": "workbooks/BAB_02_2025.xlsx", "title": "Pemerintahan" }
  ],
  "pagination": { "frontMatterFormat": "lowerRoman", "bodyFormat": "arabic", "restartAtBody": true },
  "toc": {
    "placement": "afterFrontMatter",
    "lists": ["contents", "tables", "charts"],
    "titleBilingual": true,
    "hangingIndent": true
  },
  "headerFooter": { "preset": "book", "scope": "body" },
  "build": { "keepIntermediate": true, "exportPdf": true }
}
```

Semua jalur relatif — buku kerja bab, sumber sampul, berkas keluaran — diselesaikan terhadap folder manifest. Kunci yang tidak dikenal diabaikan, dan setiap bagian boleh dihilangkan untuk memakai bawaannya.

### Urutan kerja satu kompilasi

1. Dokumen baru dengan ukuran halaman, halaman berhadapan, margin, dan bleed dari manifest (margin profil mengisi bila manifest masih memakai bawaan).
2. Halaman induk untuk badan.
3. Sampul depan dan (opsional) sampul dalam, dari sheet `COVER`.
4. Bagian awal, bernomor Romawi kecil bila `restartAtBody` aktif.
5. Placeholder daftar isi untuk daftar yang diminta.
6. Setiap bab berurutan: pembatas, ulasan, grafik, tabel — nomor badan dimulai lagi dari Arab 1 dan berlanjut antar bab.
7. Sampul belakang.
8. Daftar isi/daftar tabel/daftar grafik, penomoran halaman, dan running header.
9. Simpan, lalu ekspor PDF bila `build.exportPdf` aktif.

Compiler hanya tersedia di aplikasi desktop: build web menyatakan bahwa kompilasi memerlukan aplikasi desktop. Lewat control channel atau skrip: `{"command":"pubgen.compileProject","params":{"path":"…/pubgen_project.json","pdf":"…/out.pdf"}}`; tambahkan `"sync": false` agar berjalan bertahap di panel.

PDF yang dihasilkan adalah ekspor biasa. Preset **Print (PDF/X-4)** dan **Web** khusus Pubgen belum tersedia — untuk sekarang ekspor lewat **File ▸ Export PDF…**.


