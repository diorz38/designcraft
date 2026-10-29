## 6. Bagian 4: Utilitas Halaman & Tata Letak

Dialog **Publication Tools ▸ Page & Layout Utilities…** menyediakan berbagai tombol pintas untuk mempercepat perapian naskah buku:


### Sakelar Halaman Tunggal vs Halaman Ganda (Facing Pages)
- **Halaman Ganda (Double-sided / Facing Pages)**: Format wajib untuk buku cetak, di mana halaman kiri (*verso*) dan kanan (*recto*) berhadapan dengan margin cermin (ada ruang jilid di bagian dalam).
- **Halaman Tunggal (Single-sided)**: Untuk dokumen digital PDF yang dibaca di layar monitor atau tablet.
- Klik tombol sakelar untuk berganti mode kapan saja secara aman tanpa merusak posisi teks.

---

### Menambah Halaman Kosong (Add Blank Page)
Dalam aturan tata letak buku formal, setiap bab baru sebaiknya selalu dimulai di halaman sebelah kanan (halaman ganjil / *recto*).
- Jika bab sebelumnya berakhir di halaman kanan, klik **Add Blank Page** untuk menyisipkan satu halaman kosong sebagai penyeimbang sebelum bab baru dimulai.

---

### Daftar Isi Otomatis (Add TOC / Rebuild TOC)
Pubgen dapat memindai seluruh judul halaman di dokumen Anda dan menyusun Daftar Isi lengkap dengan titik-titik penuntun (*leader dots*) secara otomatis.

#### Jenis Daftar Isi (TOC style)
| Jenis | Isinya | Judul otomatis |
|---|---|---|
| **TOC Entry** | semua halaman yang judulnya memakai gaya penanda TOC, ditambah halaman naratif (perilaku awal) | *sesuai yang Anda ketik* |
| **TOC Table** | hanya halaman yang **kotak teks pertamanya** diawali `Tabel` atau `Table` (tidak peka huruf besar/kecil) | `Daftar Table` |
| **TOC Chart** | hanya halaman yang **kotak teks pertamanya** diawali `Grafik` atau `Chart` (tidak peka huruf besar/kecil) | `Daftar Grafik` |
| **TOC Paragraph** | halaman naratif (lembar `GXX` dan `RXX`) beserta judul-judul di bawahnya, bertingkat | `Daftar Isi` |

Memilih **TOC Table** atau **TOC Chart** akan mengisi kolom Judul secara otomatis dan menengahkan baris judulnya;
Anda tetap bisa mengubahnya. Penyaring ini tersimpan di dalam dokumen, sehingga **Rebuild TOC** dan menu
*Layout ▸ Update Table of Contents* tetap mempertahankannya.

**TOC Paragraph** adalah daftar bertingkat khas sebuah publikasi: halaman naratif (prakata dari lembar `GXX` dan
ulasan bab dari lembar `RXX`) di tingkat pertama, lalu paragraf `Heading 1` dan `Heading 2` dari isi setiap bab di
bawahnya. Tidak ada halaman lain — tabel, grafik, pembatas, maupun keterangan publikasi — yang ikut terdaftar.
Baris halaman naratif memuat baris bab dan judulnya, sehingga barisnya berbunyi `BAB 01 Geografi ......... 2`;
baris bab hanya dihitung kalau kotaknya sendiri berada di atas judul, jadi halaman yang dibuka baris lain di kotak
judulnya tetap memakai teksnya sendiri. Lembar `GXX` yang tidak punya baris judul (hanya tautan RTF) menaruh RTF di
bagian atas halaman, sehingga judul halaman adalah baris **paling besar yang ditengahkan** di antara paragraf-paragraf
awal RTF — yaitu judulnya, yang ditulis sebagai heading Word di sebagian lembar dan sebagai format langsung di
lembar lain. Dengan begitu baris ISSN di atasnya (rata kanan) dan judul publikasi di bawahnya (huruf lebih kecil)
tidak ikut terpakai, dan paragraf kosong di awal RTF pun tidak mengganggu.

Setiap daftar memakai gaya paragrafnya sendiri: `TOC Level 1…n` untuk daftar isi, `TOC Table Level 1…n` untuk
daftar tabel, dan `TOC Chart Level 1…n` untuk daftar grafik. Gaya itu membawa posisi tab dan indentasi daftar
tersebut, sehingga membuat atau membangun ulang satu daftar tidak lagi menggeser baris daftar yang lain.

**Include TOC page in TOC entries** mendaftarkan halaman daftar isi itu sendiri di dalam daftarnya — untuk semua
jenis daftar, termasuk daftar paragraf yang bertingkat.

Satu dokumen boleh memuat beberapa daftar sekaligus — misalnya *Daftar Table* dan *Daftar Grafik* berdampingan.
**Add TOC** membuat daftar di halaman yang sedang dibuka dan menggantikan daftar yang sudah ada di halaman itu
(satu halaman satu daftar).

#### Pilihan Tata Letak Daftar Isi:
1. **Standard Layout**:
   - Format umum buku teks:
     `Tabel 1.1 Luas Wilayah Menurut Desa ........................... 12`
2. **TablePage Layout (Standar Katalog Statistik BPS)**:
   - Diformat khusus mengikuti standar rujukan buku BPS:
     - Judul kolom kiri: `Tabel`
     - Judul kolom kanan: `Halaman`
     - Nomor tabel terpisah rapi: `1.1`
     - Teks judul tabel berinden menggantung (*hanging indent*) agar baris kedua tidak bertumpuk di bawah nomor tabel.
     - Nama bab ditulis tebal (*bold*) tanpa nomor halaman.
     - **Header Berulang Otomatis**: Jika Daftar Isi memanjang lebih dari satu halaman, tulisan `Tabel` dan `Halaman` akan otomatis diulang di bagian paling atas halaman berikutnya!

#### Memperbarui Daftar Isi (Rebuild TOC)
Jika di kemudian hari Anda menambah, menghapus, atau menukar posisi halaman, klik **Rebuild TOC**. Pubgen
memperbarui daftar **di halaman yang sedang dibuka** memakai jenis, judul, dan tata letak daftar itu sendiri,
sehingga daftar lain di dokumen tidak tersentuh. Saat penomoran halaman diubah (Renumber), seluruh daftar di
dokumen ikut diperbarui agar nomor halamannya tetap benar.

---

### Impor Cerita Teks RTF (Page from RTF)
Untuk memasukkan naskah teks panjang seperti *Kata Pengantar*, *Undang-Undang Statistik*, atau *Ringkasan Eksekutif*:
1. Simpan naskah Word Anda ke dalam format Rich Text Format (`.rtf`).
2. Klik **Browse...** lalu pilih berkas `.rtf` tersebut.
3. Klik **Insert RTF Story**.
4. Pubgen akan mengalirkan teks secara otomatis mengikuti batas margin halaman. Jika teks terlalu panjang, Pubgen **secara otomatis membuatkan halaman-halaman baru dan menyambungkan kotak teksnya** hingga seluruh naskah selesai tertampung. Huruf tebal, miring, ukuran font, dan poin balet tetap terjaga rapi.

---

### Menyisipkan Gambar (Page from Image: Objek vs Latar Belakang Penuh)
Memudahkan penempatan foto infografis atau halaman pemisah bab (*Pembatas BAB*):
- **Page margins (Object)**: Menyesuaikan ukuran foto agar pas di dalam batas margin halaman.
- **Page background**: Memperbesar foto hingga memenuhi seluruh bidang kertas (*full-bleed*), memposisikannya di tengah, lalu memindahkannya ke lapisan paling belakang. Sangat cocok untuk halaman pemisah bab yang berlatar gambar penuh.

---

### Penomoran Ulang Halaman & Seksi (Romawi vs Arab)
Buku resmi umumnya terbagi menjadi dua bagian penomoran:
1. **Bagian Awal / Halaman Pendahuluan** (Kata Pengantar, Daftar Isi): Menggunakan angka Romawi kecil (`i, ii, iii...`).
2. **Bagian Isi / Bab Utama** (Mulai dari Bab 1 hingga selesai): Menggunakan angka Arab biasa (`1, 2, 3...`).

#### Cara Penggunaan:
1. Buka halaman Kata Pengantar atau Daftar Isi Anda.
2. Di menu **Page Renumbering**, pilih **Style: Roman (i, ii, iii...)**, Start #: `1`. Klik **Restart page #**.
3. Buka Halaman 1 dari Bab pertama Anda.
4. Pilih **Style: Arabic (1, 2, 3...)**, Start #: `1`. Klik **Restart page #**.
5. Klik **Rebuild TOC**: Seluruh angka halaman di Daftar Isi otomatis diperbarui mengikuti skema penomoran Romawi untuk halaman awal dan Arab untuk halaman isi!


