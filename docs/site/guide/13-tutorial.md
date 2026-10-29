## 14. Tutorial Praktis Langkah demi Langkah

### Tutorial 1: Menyusun Publikasi Lengkap dalam 3 Menit

Berikut alur kerja praktis menyusun publikasi statistik dari nol hingga selesai:

1. **Membuat Dokumen Baru**:
   - Tekan tombol pintas `Ctrl+N` (Menu `File` ▸ `New Document`). Pilih ukuran `A4`, centang *Facing Pages*.
2. **Membuat Sampul Depan**:
   - Buka **Publication ▸ Publication Cover…**.
   - Pilih gaya **`⬡ Hexagon`** (atau arahkan kursor ke pil gaya untuk melihat pratinjau).
   - Isi judul: `KARAWANG TIMUR` dan tahun: `2026`.
   - Klik **Generate Cover Page**. Sampul langsung jadi di Halaman 1!
3. **Membuat Halaman Judul Bab**:
   - Buka **Publication Tools ▸ Manual Ingest & Tables ▸ Page from CSV…**.
   - Klik **Browse...** lalu pilih berkas contoh `samples/KBDA2026_chapters.csv`.
   - Pilih preset: `1 Row · 3 Columns`.
   - Klik **Generate Pages**. Sebanyak 28 halaman judul bab otomatis terbentuk!
4. **Memasukkan Tabel Data**:
   - Pindah ke Halaman 2 dokumen.
   - Buka **Publication Tools ▸ Manual Ingest & Tables ▸ Table from CSV…**.
   - Klik **Browse...** lalu pilih berkas `samples/kbda_2005.csv`.
   - Pilih gaya: `Horizontal Rules (Booktabs)`. Penempatan: `At page (below title)`.
   - Centang opsi `[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`.
   - Klik **Insert Table**. Tabel statistik langsung terpasang rapi di bawah judul!
5. **Membuat Daftar Isi**:
   - Di **Publication Tools ▸ Page & Layout Utilities…**, klik **Add Blank Page**.
   - Buka **Publication ▸ Table of Contents…**, lalu pilih TOC style: `TOC Table` (kolom Judul otomatis terisi `Daftar Table`), pilih
     Layout: `TablePage`.
   - Klik **Add TOC**. Daftar isi langsung tercipta lengkap dengan nomor halamannya! Untuk daftar grafik, pindah ke
     halaman berikutnya lalu pilih TOC style: `TOC Chart` dan klik **Add TOC** lagi.
6. **Mengekspor ke PDF**:
   - Buka menu utama `File` ▸ `Export` ▸ `PDF (Print)`. Publikasi Anda siap untuk dicetak atau disebarluaskan secara digital!

---

### Tutorial 2: Mendesain Sampul Kustom dengan Foto & Warna Sendiri

Ingin menyesuaikan sampul dengan identitas visual daerah Anda?

1. Pada dialog **Publication Cover**:
   - Pilih gaya sampul, misalnya **`❖ Medallion`** atau **`⬡ Hexagon`**.
2. **Menyesuaikan Warna**:
   - Masukkan kode warna resmi daerah Anda, misalnya:
     - Primary: `#1e3a8a` (Biru Laut)
     - Accent: `#10b981` (Hijau Zamrud)
     - Dark: `#0f172a` (Biru Gelap Arang)
3. **Memilih Foto Sendiri**:
   - Pada bagian **Cover Images**, klik tombol **Browse...** di samping *Landscape* atau *Honeycomb*.
   - Pilih foto pemandangan terbaik dari komputer Anda (format `.jpg` atau `.png`).
4. **Memasang Lambang Daerah / Logo**:
   - Klik **Browse...** pada kotak *Logo* lalu pilih berkas lambang daerah berlatar transparan.
   - Ketikkan identitas lembaga pada isian *Logo Text*.
5. Klik **Generate Cover Page**. Sampul eksklusif Anda langsung selesai!

---

### Tutorial 3: Memformat Tabel Statistik Kompleks dari Excel

Jika Anda memiliki tabel dengan kelompok kolom bersusun (misalnya: Jumlah Penduduk menurut Kelompok Umur dan Jenis Kelamin):

1. Pada lembar kerja Excel, beri nama kepala kolom menggunakan tanda hubung ganda (`--`):
   | Kecamatan | Usia 0-14--Laki-Laki | Usia 0-14--Perempuan | Usia 15-64--Laki-Laki | Usia 15-64--Perempuan | Total |
2. Simpan lembar kerja tersebut sebagai berkas `.xlsx` atau `.csv`.
3. Di DesignCraft, buka dialog **Table from CSV / Excel**:
   - Klik **Browse...** lalu pilih berkas Anda.
   - Centang `[x] First row is header`.
   - Centang `[x] Add column numbers (1), (2)...`.
   - Centang `[x] Table footer (last row)`.
   - Centang `[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`.
   - Pilih gaya `Horizontal Rules (Booktabs)`.
4. Klik **Insert Table**.
5. Hasilnya: Tabel bertingkat rapi dengan penggabungan sel otomatis, baris penomoran kolom resmi `(1), (2), (3)...`, serta garis pembatas khusus pada baris total!

---

### Tutorial 4: Mengompilasi Seluruh Publikasi dari Satu Manifest

Untuk buku tahunan dengan delapan sampai dua belas buku kerja bab, compiler menggantikan perulangan manual:

1. **Kumpulkan buku kerja** dalam satu folder, misalnya `workbooks/BAB_01_2025.xlsx` … `workbooks/BAB_10_2025.xlsx`, dengan sheet `COVER` di bab 1 atau di buku kerja tersendiri.
2. **Pilih profil** di header panel — `BPS Daerah Dalam Angka` untuk buku tahunan KDA/KCDA (§12). Nilai `"profile"` di manifest menang bila diisi.
3. **Buat manifest**: di kartu Publication Compiler tekan **New…**, isi judul, daftar bab, margin, dan apakah akan mengekspor PDF, lalu tentukan lokasi penyimpanan `pubgen_project.json`. Skema lengkapnya ada di §10.
4. **Compile**: tekan **Compile**. Bilah progres menyebut setiap langkah — dokumen baru, halaman induk, sampul depan, sampul dalam, bagian awal, placeholder daftar, tiap bab, sampul belakang, daftar isi, penomoran halaman, running header, simpan, PDF. **Cancel** berhenti dengan rapi dan tidak menyentuh hasil build sebelumnya.
5. **Periksa**: baca baris status (halaman, bab, peringatan), lalu jalankan **Audit Publication** (§11) dan klik baris yang ditandai untuk langsung melompat ke sana.
6. **Ekspor**: dengan `"exportPdf": true` PDF ditulis di samping dokumen; bila tidak, gunakan **File ▸ Export PDF…**.



