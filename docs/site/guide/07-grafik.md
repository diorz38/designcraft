## 8. Grafik Vektor & Infografis Statistik

Grafik di Pubgen adalah objek dokumen asli — batang, garis, dan poligon vektor dengan teks sungguhan — bukan gambar. Tajam pada zoom berapa pun dan saat dicetak, serta masih bisa diedit setelah dibuat.

Buka pembuat grafik dari **Publication ▸ Vector Chart** (`Ctrl+Shift+V`), baris **Chart Options…** di **Object ▸ Chart Options…**, atau biarkan Master Excel mengubah sheet `CXX_XX` menjadi halaman grafik.

### Sembilan arketipe

| Arketipe | Nilai yang diterima di `chart_type` | Kegunaan umum |
|---|---|---|
| Batang berkelompok | `bar_clustered`, `bar`, `column`, `batang` | perbandingan berdampingan per kategori |
| Batang bertumpuk | `bar_stacked`, `stacked`, `stackedbar`, `stackedcolumn` | bagian dari keseluruhan per kategori |
| Batang horizontal | `horizontal_bar`, `hbar`, `bar_h`, `batanghorizontal`, `ranked` | peringkat antarwilayah |
| Garis | `line`, `garis`, `trend` | deret multi-tahun |
| Area | `area`, `filled`, `arealine` | tren dengan volume |
| Donat | `donut`, `doughnut`, `pie`, `donat`, `lingkaran` | pangsa dari total |
| Piramida penduduk | `population_pyramid`, `pyramid`, `piramidapenduduk`, `piramida` | distribusi umur/jenis kelamin |
| Gauge | `gauge`, `speedometer`, `dial`, `alatukur` | satu metrik terhadap target |
| Peta choropleth | `map`, `choropleth`, `peta`, `geomap`, `thematicmap` | nilai menurut wilayah |

### Yang ditanyakan dialog grafik

1. **Import from Table** — pilih tabel yang sudah ada di dokumen (daftar berisi `Tabel 01.01 — … (Page N)`), pilih halamannya, lalu tekan **Auto-fill** untuk membaca kategori, seri, dan label dari tabel itu.
2. **Insert Location** — *Selected page*, *After table*, atau *Current page*.
3. **Chart Type & Titles** — arketipe serta **Title (ID)**, kolom dua baris mengikuti format BPS (`Grafik 01.01` di baris 1, judul data di baris 2), dan **Title (EN)** yang dirender sebagai `Figure 01.01 …` miring pada mode Dual Lang.
4. **Data Columns & Rows** — huruf kolom kategori, rentang baris, dan hingga tiga seri, masing-masing dengan huruf kolom serta label legenda Indonesia dan Inggris.
5. **Maps** (khusus choropleth) — peta administrasi bawaan atau berkas `.geojson` sendiri, tema warna, dan contoh CSV desa untuk mencoba mesinnya.
6. **Formatting & Display Options** — *Number (1.234,5)*, *Percentage (%)*, atau *Currency IDR (Rp)*, batas nilai maksimum (kosong = batas "bulat" otomatis), serta **Show Value Labels**, **Show Legend**, **Show Gridlines**.

### Grafik dari Excel

Sheet grafik adalah blok pasangan `key` / `value`. Hanya `chart_type` yang wajib:

```
chart_type        bar_clustered
title             Pertumbuhan Ekonomi Menurut Sektor
title_en          Economic Growth by Sector
category_col      A
series_1_col      B
series_1_label    Pertanian
series_1_label_en Agriculture
series_2_col      C
series_2_label    Industri
data_start_row    5
data_end_row      12
value_format      percent
show_value_labels true
```

Kunci lain yang diterima: `data_start_row` / `data_end_row`, `max_value`, `show_legend`, `show_grid`, `geojson`. Seri 2 dan 3 memakai pola `series_N_col` / `series_N_label` / `series_N_label_en`. Tanpa `chart_type`, sheet ditempatkan sebagai gambar seperti sebelumnya.

Grafik terindeks otomatis: halaman grafik vektor muncul di **Daftar Grafik** seperti gambar lainnya. Halaman peta juga bisa menjadi lapisan peta dekoratif di belakang teks dan logo sampul.


