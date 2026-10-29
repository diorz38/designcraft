## 1. Apa itu Pubgen?

**Pubgen** (*Publication Generator*) adalah modul otomasi tata letak yang terpasang langsung di dalam DesignCraft.

Dalam aplikasi tata letak konvensional (seperti Adobe InDesign atau QuarkXPress), pembuatan publikasi statistik setebal ratusan halaman (seperti buku *Kecamatan Dalam Angka*, *Kabupaten Dalam Angka*, atau Laporan Tahunan) menuntut ratusan langkah manual yang memakan waktu: menggambar kotak teks satu per satu, mengatur gaya judul, merapikan batas tabel, memasukkan logo, dan menyusun titik-titik daftar isi.

Pubgen mengotomatiskan seluruh alur kerja tersebut:
- **Sampul Instan**: Membangun sampul korporat eksekutif lengkap dengan geometri modern, bayangan lembut, foto terpotong rapi, dan tipografi resmi — depan, dalam, dan belakang.
- **Halaman Otomatis**: Membaca daftar bab atau tabel dari Excel/CSV dan menyusun puluhan halaman rapi dalam hitungan detik.
- **Tabel Standar Penerbitan**: Memformat tabel data mengikuti standar baku BPS (garis horizontal tipis-tebal *booktabs*, header bertingkat, penomoran kolom resmi, serta penyorotan baris Jumlah/Total secara otomatis).
- **Grafik Vektor**: Mengubah sheet `CXX_XX` (atau tabel yang sudah ada) menjadi grafik asli yang bisa diedit — sembilan arketipe, dari batang dan garis sampai piramida penduduk, gauge, dan peta GeoJSON.
- **Buku Satu Tombol**: Satu buku kerja Master Excel menghasilkan halaman, tabel, grafik, dan sampul sekaligus; Publication Compiler menyatukan banyak buku kerja bab menjadi satu dokumen dari satu manifest.
- **Audit Mutu**: Preflight statistik memeriksa sel tabel terpotong, sumber yang hilang, nomor tabel yang loncat, kesetaraan dwibahasa, DPI gambar, dan — untuk cetak — spine serta bleed, dengan baris yang bisa diklik.
- **Profil & Terminologi**: Profil publikasi mengganti perutean sheet, penamaan, geometri, dan gaya tabel; glosarium yang bisa diedit menjaga terjemahan AI memakai istilah statistik resmi.
- **Alat Editorial Praktis**: Memudahkan pengelolaan halaman bolak-balik (ganda), penomoran angka Romawi dan Arab, header dan footer, pengaliran teks dokumen RTF, serta peremajaan Daftar Isi secara dinamis.

---

## 2. Menemukan Pubgen di DesignCraft

Pubgen berada di **bilah menu**. Bila Pubgen aktif — kondisi bawaan — bilah menu mempertahankan seluruh menu bawaan DesignCraft dan menambahkan **Publication**, **Publication Tools**, serta **Shortcut** sebelum **Help**:

```
+-----------------------------------------------------------------------------------------------------------+
|  File  Edit  Layout  Type  Object  Table  View  Window  Publication  Publication Tools  Shortcut  Help |
+-----------------------------------------------------------------------------------------------------------+
```

| Menu | Isinya |
|---|---|
| **File … Window** | Menu bawaan DesignCraft, tidak berubah. |
| **Publication** | Alur pembuatan publikasi: Generate from Excel (`Ctrl+Shift+G`), Compile Publication (`Ctrl+Shift+B`), Publication Profile, Vector Chart (`Ctrl+Shift+V`), Table of Contents (`Ctrl+Alt+T`), Publication Cover, Header & Footer Setup, Audit Publication (`Ctrl+Shift+Alt+A`). |
| **Publication Tools** | **Manual Ingest & Tables** (Page from CSV…, Table from CSV…), **Publication Profiles** (pilih profil, Customize Active Profile…, Apply Profile Margins), **Page & Layout Utilities** (Standard BPS Margins `Ctrl+Shift+M`, Page Margins & Renumbering Options…), centang **Dual Language Mode** (`Ctrl+Shift+L`), dan **AI Translate** (Translate Selected Page, Translate Table Pages…, Fix Terms with Glossary, Setup…). |
| **Shortcut** | Akses cepat ke aksi asli DesignCraft: Export PDF…, Preflight Document, Package…, Place…, New Sample Document. |
| **Help** | Baris Help bawaan DesignCraft, lalu **About PubGen…** dan tombol pengalih bilah menu. |

Dengan begitu, publikasi lengkap hanya berjarak tiga kunjungan menu:

1. **Publication ▸ Generate from Excel** — membaca naskah Excel/CSV dan membangun halamannya.
2. **Publication ▸ Publication Cover** — membuat sampul.
3. **Publication ▸ Table of Contents** — menyisipkan atau memperbarui Daftar Isi.

Karya besar ditempuh lewat **Publication ▸ Compile Publication** (§10), sedangkan pekerjaan statistik lewat **Vector Chart** (§8), **Header & Footer Setup** (§9), **Publication Profile** (§12), dan **Audit Publication** (§11).

Tindakan editorial lainnya (sebaran satu sisi, halaman kosong, halaman RTF dan gambar, penomoran ulang halaman) dibuka dari **Publication Tools ▸ Page & Layout Utilities…**, sedangkan jalur manual dari **Publication Tools ▸ Manual Ingest & Tables**.

### Panel PubGen

Semua yang ada di menu juga tersedia sebagai panel, dibuka dari **Window ▸ PubGen Panel**. Panel adalah permukaan kerja; menu hanyalah jalan pintas.

| Area | Fungsinya |
|---|---|
| **Header panel** | Pemilih profil dengan ringkasan satu baris, **AI Translator**, **Translate this Page** (atau *Translate Review / Translate Table on this Page*), **Fix Terms**, **Retranslate (Glossary)**, **Translate Tables…**, **Audit Publication**, **Set Page Margin**, dan lencana Single/Dual Lang. |
| **Publication Compiler** | Satu manifest menjadi seluruh publikasi (§10), khusus desktop. |
| **1. Data Source & Generate (Master Excel)** | Alur buku kerja satu tombol (§7). |
| **2. Table of Contents** | Gaya, judul, tata letak ToC, dan **Add TOC / Rebuild TOC** (§6). |
| **3. Manual Import & Fine-Tuning** | Grup yang bisa dilipat berisi **Page from CSV** (§4) dan **Table from CSV** (§5) untuk kerja manual. |
| **4. Page Utilities** | Sakelar sebaran, halaman kosong, penomoran ulang halaman dan seksi (§6). |

Dialog terbuka di atasnya: Master Excel, Publication Cover, Table of Contents, Header & Footer, Vector Chart, Audit Publication, kustomisasi profil, penyiapan AI, dan About PubGen.

### Beralih ke menu DesignCraft
Seluruh tindakan Pubgen juga tersedia di kumpulan menu lengkap DesignCraft, di dalam menu **Pubgen** yang terletak tepat sebelum **Help**. Untuk mencapai bilah DesignCraft polos — tanpa tiga menu Pubgen — alihkan bilah menunya:

- **Help ▸ Show DesignCraft Menus**, atau
- tombol yang sama di **Help ▸ About PubGen…**.

Bilah menu kini menampilkan menu lengkap DesignCraft dengan Pubgen disisipkan sebelum Help. Pilih **Pubgen ▸ Show PubGen Menus** untuk kembali. Pengaturan ini berlaku per sesi: aplikasi yang baru dibuka selalu mulai dalam mode Pubgen.

### Tombol Mode Dwi Bahasa (`Single Lang` / `Dual Lang`)
Baris **Dual Language Mode** pada menu **Publication Tools** mencentang atau melepas keluaran dwi bahasa (tanpa centang = Single Lang, default):
- **Single Lang**: Pembuatan publikasi satu bahasa (monolingual) standar seperti sebelumnya.
- **Dual Lang**: Menghasilkan publikasi dwi bahasa Indonesia-Inggris sesuai standar publikasi BPS:
  - **Sampul Depan**: Menautkan judul bahasa Inggris ke bingkai `#slot:text:title_en` bila templat menyediakannya, dan bila tidak ke baris judul kedua (`title#2`).
  - **Halaman Pembatas Bab (`DIV`)**: Menampilkan judul ganda dengan subjudul bahasa Inggris menggunakan warna aksen.
  - **Bab Ulasan (`RXX`) & Bagian Depan (`BXX`)**: Membuat sebaran berurutan (halaman Indonesia diikuti halaman Inggris).
  - **Tabel Statistik (`XX_XX`)**: Menggunakan tajuk 2 baris (`Tabel` / `Table` miring) dan judul 2 baris dengan teks Inggris dicetak miring.
  - **Grafik (`CXX_XX`)**: Memformat judul dwi bahasa (baris Indonesia, lalu `Figure NN.NN …` miring) dengan tipografi Inggris yang benar.

### AI dalam satu paragraf
AI bersifat opsional dan memerlukan build daring (`cargo run --release -p designcraft --features pubgen,ai-translate-online`). Terjemahan halaman dan sel tabel memakai **Groq**, sampul memakai **Cloudflare Workers AI** (§3), dan konsistensi istilah dijaga oleh memori terminologi yang bisa diedit. Penyiapan, tombol terjemahan halaman dan massal, editor glosarium, serta batasannya ada di §13.


