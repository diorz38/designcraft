## 12. Publication Profiles (Profil Publikasi)

Profil adalah gaya rumah publikasi yang dideklarasikan sekali: sheet mana berperan sebagai apa, apa namanya, geometri halaman, struktur bagian awal, dan gaya tabel. Ganti profil, buku kerja yang sama menghasilkan publikasi berbeda.

| Profil | Untuk | Geometri | Tabel |
|---|---|---|---|
| **BPS Daerah Dalam Angka** (`bps_dda`) | Buku tahunan KDA/KCDA resmi (bawaan) | A4, halaman berhadapan, 20,2 / 20 / 19 / 15 mm | baris nomor kolom, palet tema, pembatas bab |
| **BPS Berita Resmi Statistik** (`bps_brs`) | Rilis pers dan rilis berkala | A4, halaman tunggal, 18 mm semua sisi | tanpa nomor kolom, baris zebra |
| **Laporan Tahunan Korporat** (`corporate_annual`) | Laporan tahunan dan keberlanjutan | A4, berhadapan, 25 / 25 / 22 / 18 mm, label `Section` / `Table` / `Figure` | hanya garis horizontal, corporate navy |
| **Monograf Akademik & Riset** (`academic_monograph`) | Kertas kerja, policy brief, LKIP/LAKIP | A4, berhadapan, 25 / 25 / 25 / 20 mm, `Gambar` untuk figur | tabel tiga garis gaya APA, monokrom |

Cara memakainya:

- **Header panel** — dropdown profil, lencana **Auto-detect** saat Pubgen mengenali buku kerja, ringkasan satu baris (ukuran halaman, margin, nomor kolom), dan **Sesuaikan…** untuk kustomisasi.
- **Publication Tools ▸ Publication Profiles** — daftar yang sama dengan tanda centang, **Customize Active Profile…**, dan **Apply Profile Margins**.
- **Publication Tools ▸ Page & Layout Utilities ▸ Standard BPS Margins** (`Ctrl+Shift+M`) — menerapkan 20 / 20 / 19 / 15 mm terlepas dari profil.
- Compiler mematuhi `"profile"` di manifest (§10).

**Kustomisasi** mengubah profil aktif di tempat: ukuran halaman dan halaman berhadapan, empat margin, prefiks bab/tabel/grafik, judul daftar isi dan daftar, label sumber dan catatan, penomoran bagian awal, pembatas bab, sampul dalam dan belakang, serta gaya tabel. **Apply to this session** menyimpan perubahan; **Reset to BPS DDA** kembali ke bawaan.

Dua batasan yang perlu diketahui: profil kustom hanya bertahan selama sesi (pilihan aktif tidak disimpan antar peluncuran), dan profil belum bisa diimpor atau diekspor sebagai berkas.


