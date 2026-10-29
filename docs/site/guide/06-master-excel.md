## 7. Master Excel: Satu Buku Kerja Menjadi Halaman Jadi

Kartu **1. Data Source & Generate (Master Excel)** di bagian atas panel PubGen mengubah satu berkas `.xlsx` menjadi satu bab utuh — halaman, tabel, grafik, dan sampul — dalam satu tindakan.

| Kontrol | Fungsinya |
|---|---|
| **Auto-generate pages, tables & cover from sheets** | Aktif secara bawaan. Saat berkas dipilih, seluruh alur berjalan: "Page from CSV", tabel setiap sheet dengan opsi bawaannya, setiap halaman grafik, dan sampul dari sheet `COVER`. Matikan untuk hanya mengonversi buku kerja menjadi CSV. |
| **Excel to CSV Data (.xlsx)…** | Pilih buku kerja. Pubgen menulis satu CSV katalog plus satu CSV tabel per sheet data, lalu (dengan auto-generate aktif) membuat halamannya. |
| **Active: …** | Menampilkan nama buku kerja atau katalog yang dimuat beserta jumlah record katalog. |
| **Download … / Download Generated CSVs as ZIP…** | Menyimpan semua CSV katalog dan tabel yang dihasilkan dalam satu `.zip`. |
| Baris status | Melaporkan hasil generate atau kesalahan pertama, plus jumlah kosakata yang dijelaskan di §13. |

### Sheet menjadi apa

| Nama sheet | Menjadi |
|---|---|
| `COVER` | Nilai Publication Cover: `style`, `title1`, `title2`, `year`, `volume`, `no_catalogue`, ISSN/ISBN, `office_name`, `image_1`, `image_lanscape`, `office_logo`, `event_theme`. |
| `DIV` | Halaman pembatas bab (art prosedural atau templat, diwarnai dari tema sheet). |
| `GXX` | Halaman narasi bagian awal (Kata Pengantar, Penjelasan Teknis…). |
| `BXX` | Halaman narasi bagian awal yang juga membawa versi bahasa Inggris. |
| `RXX` | Spread ulasan: badan bahasa Indonesia, plus kolom Inggris pada mode Dual Lang. |
| `XX_XX` | Halaman tabel statistik (`01_01`, `02_03`…), tabelnya disisipkan otomatis. |
| `CXX_XX` | Halaman grafik — vektor bila sheet memuat blok `chart_type`, jika tidak gambarnya (§8). |
| `MXX_XX` | Halaman peta choropleth GeoJSON (§8). |
| `A02` | Baris **Publication Note**: judul `Keterangan Publikasi` plus halaman catatan RTF dari sheet. Tidak menghasilkan CSV tabel. |

Perutean sheet **mengikuti profil** (§12): pola sampul, pembatas, narasi, tabel, dan grafik berasal dari profil aktif, dan Pubgen berpindah profil otomatis saat mengenali nama sheet BRS, laporan keuangan korporat, atau akademik. Pilihan profil eksplisit selalu menang.


