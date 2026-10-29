## 11. Audit Publication (Preflight QA)

**Publication ▸ Audit Publication** (`Ctrl+Shift+Alt+A`), atau tombol **Audit Publication** di header panel, menjalankan daftar periksa publikasi sebelum dikirim ke percetakan atau dipublikasikan daring.

| Aturan | Yang diperiksa | Tingkat |
|---|---|---|
| `PRF-01` | Teks terpotong di dalam sel tabel asli (kolom terlalu sempit, baris terlalu pendek). | Error |
| `PRF-02` | Story overset: narasi meluap tanpa frame lanjutan. | Error |
| `PRF-03` | Baris `Sumber:` / `Catatan:` yang kosong setelah titik dua. | Warning |
| `PRF-04` | Penomoran tabel dan grafik: nomor loncat dan ganda, dikelompokkan per jenis dan bab (pasangan dwibahasa pada satu halaman bukan duplikat). | Warning |
| `PRF-05` | Kesetaraan dwibahasa: halaman ulasan yang badan Inggrisnya kosong, placeholder, atau masih teks Indonesia. | Warning |
| `PRF-06` | Gambar raster di bawah ambang DPI profil yang dipilih. | Warning |
| `PRF-07` | Khusus cetak: grafis melewati tepi trim tanpa bleed 3 mm, dan teks dalam 5 mm dari spine (atau melewatinya). | Error |

- **Digital (online PDF)** — ambang 150 ppi, tanpa pemeriksaan spine/bleed.
- **Print (press)** — ambang 300 ppi plus pemeriksaan spine dan bleed.

Klik baris mana pun untuk memilih objek bermasalah dan melompat ke halamannya. Tombol **FP / IG** memutar tag — *false positive* atau *ignored*. Baris bertag meredup, bisa disembunyikan dengan **hide tagged**, dan tidak lagi dihitung di ringkasan; tag dikunci berdasarkan aturan dan pesan sehingga bertahan setelah audit ulang maupun regenerasi. **Min ppi** menimpa ambang profil, dan **Re-run** menyegarkan daftar. Dialog juga menjalankan ulang sendiri bila dokumen berubah sejak audit terakhir, sehingga baris tidak pernah menunjuk objek yang sudah tidak ada.


