## 4. Bagian 2: Page from CSV / Excel (Otomasi Pembuatan Halaman & Bab)

Jika Anda memiliki daftar isi atau matriks katalog publikasi di Excel atau CSV, alat ini akan membuatkan puluhan hingga ratusan halaman isi lengkap dengan judulnya secara otomatis.

![Fiture Pubgen:Page from CSV](../../images/thumb03.png)

### Cara Kerja
Daripada menambah halaman baru secara manual satu demi satu lalu mengetik "Tabel 1.1.1 Luas Wilayah Menurut Desa...", Pubgen membaca data tabel Anda dan menghasilkan halaman-halaman tersebut secara instan.

### Format Berkas Input (CSV & Excel)
- Klik **Browse...** untuk memilih berkas `.csv`, `.tsv`, atau `.xlsx` (Excel).
- **Deteksi Pemisah Otomatis**: Pubgen secara cerdas mengenali apakah berkas Anda menggunakan koma (`,`), titik koma (`;`, format umum pada sistem operasi Indonesia), atau tab.

### Pemetaan Kolom (Type, Number, Title)
Pubgen membutuhkan tiga informasi pokok dari setiap baris data:
1. **Type (Tipe/Kategori)**: Tingkatan atau jenis judul (contoh: `Tabel`, `BAB`, `Grafik`, `Subbab`).
2. **Number (Nomor)**: Penomoran atau kode judul (contoh: `1.1.1`, `BAB I`, `Grafik 2.1`).
3. **Title (Judul)**: Teks deskripsi isi halaman (contoh: `Luas Wilayah Menurut Desa/Kelurahan`).

Jika berkas Anda memiliki nama kolom seperti `type`, `number`, `title` (atau `kategori`, `nomor`, `judul`), Pubgen akan memilihnya secara otomatis. Anda juga dapat mengubah dropdown pemetaan secara manual.

### Preset Tata Letak (Layout Presets)
Tentukan bagaimana bagian kepala (*header*) halaman disusun:
- **1 Row · 3 Columns (Standar Publikasi BPS - Disarankan)**:
  - Menyusun judul menjadi 1 baris horizontal rapi:
    `Tabel` (kiri) | `1.1.1` (tengah) | `Luas Wilayah Menurut Desa...` (kanan).
  - Lebar kolom disesuaikan secara otomatis dengan panjang teks sehingga tidak ada rongga kosong yang janggal.
- **Editorial Chapter (Stacked)**:
  - Menyusun kategori di atas, angka bab besar di tengah, dan judul panjang di bawah. Sangat cocok untuk halaman pembatas bab (contoh: *BAB 1 GEOGRAFI*).
- **Technical Spec Card & Minimal Zine**:
  - Tata letak alternatif bergaya kartu teknis atau minimalis modern.

### Cakupan Data & Penempatan Halaman
- **All records**: Memproses seluruh baris data dari awal hingga akhir.
- **Range (contoh: 1-10)**: Hanya memproses baris tertentu saja.
- **Append to document**: Menambahkan halaman-halaman baru ke bagian paling belakang buku. Hapus centang jika ingin menyisipkan halaman persis setelah halaman yang sedang aktif.

### Menyisipkan Tabel Otomatis dari Kolom 'file'
Jika berkas katalog CSV Anda memiliki kolom ke-4 bernama `file` yang berisi nama berkas tabel (contoh: `01_01.csv`, `01_02.csv`), centang opsi:
`[x] Auto-insert table from 'file' col`
Maka Pubgen akan **sekaligus membuat halamannya DAN memasang tabel data statistik yang terformat rapi persis di bawah judul halaman** dalam satu proses!

### Halaman Catatan Publikasi (`Note`)
Baris katalog ber-type `Note` memakai tata letak khusus, bukan preset: kotak judul Arial di tengah yang sepenuhnya
transparan (`Keterangan Publikasi`, atau judul baris tersebut bila ada) dan berkas RTF dari kolom `file` ditempatkan
di bawahnya. Konverter Excel → CSV Data membuat baris ini secara otomatis untuk sheet bernama tepat `A02`
(baris 1: `title;Keterangan Publikasi`, baris 2: `file;notes/A02.rtf`), sehingga sheet tersebut tidak menghasilkan
CSV tabel — hanya baris katalognya. Nama sheet `A##` lainnya tetap diperlakukan sebagai sheet biasa.


