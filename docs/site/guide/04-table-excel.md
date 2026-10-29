## 5. Bagian 3: Table from CSV / Excel (Pembuat Tabel Statistik Siap Cetak)

Tabel statistik kelembagaan menuntut ketelitian tinggi: batas garis yang rapi, penomoran kolom resmi, serta penegasan pada baris total. Pubgen mengubah data angka mentah menjadi tabel vektor asli yang siap cetak.

![Fiture Pubgen:Table from CSV](../../images/thumb04.png)

### Cara Kerja
Pilih berkas CSV atau lembar kerja Excel, tentukan gaya garis yang diinginkan, lalu klik **Insert Table**. Tabel akan langsung terpasang di dokumen dan dapat diedit secara penuh layaknya tabel buatan tangan.

---

### Opsi Header & Header Bertingkat (Stacked Headers)

#### Baris pertama sebagai header (First row is header)
Centang opsi ini jika baris pertama data Anda berisi nama-nama kolom. Teks akan otomatis ditebalkan (*bold*) dan baris dikunci sebagai kepala tabel.

#### Header Bertingkat Otomatis (`Induk--Anak`)
Pada publikasi statistik, kolom sering kali dikelompokkan di bawah satu payung kategori (contoh: kolom *Laki-Laki* dan *Perempuan* yang berada di bawah kelompok *Jenis Kelamin*).
- Di berkas Excel atau CSV Anda, cukup tuliskan nama kolom dengan tanda strip ganda (`--`):
  `Jenis Kelamin--Laki-Laki`
  `Jenis Kelamin--Perempuan`
  `Jumlah`
- Pubgen secara otomatis membaca tanda strip ganda tersebut dan membuat **kepala tabel bertingkat** (hingga 3 tingkat):
  - Kolom induk (*Jenis Kelamin*) otomatis **menggabungkan sel secara horizontal** (*merge horizontal*) menaungi anak-anaknya.
  - Kolom tunggal (*Jumlah*) otomatis **menggabungkan sel ke bawah** (*merge vertical*) sehingga tampilan kepala tabel tetap simetris dan rapi.

---

### Baris Nomor Kolom: (1), (2), (3)...
Standar baku publikasi BPS mewajibkan adanya baris penomoran kolom di bawah kepala tabel.
- Centang opsi **`[x] Add column numbers (1), (2)...`**.
- Pubgen akan menyisipkan satu baris khusus tepat di bawah judul kolom dengan nomor `(1)`, `(2)`, `(3)`... di tengah-tengah setiap kolom, dibatasi garis tipis 0,5pt.

---

### Baris Footer Tabel & Catatan Sumber Otomatis

#### Table footer (baris terakhir sebagai footer)
Centang opsi ini jika baris terakhir tabel Anda adalah baris rekapitulasi atau total. Baris ini akan diberi garis pembatas atas 0,5pt dan garis penutup bawah 1,0pt.

#### Deteksi Otomatis Catatan Kaki / Sumber Data
Jika baris terakhir dari data CSV Anda berisi keterangan sumber (contoh: `Sumber: Dinas Kependudukan dan Pencatatan Sipil`):
- Pubgen secara cerdas mengetahui bahwa hanya kolom pertama yang berisi teks sementara kolom lainnya kosong.
- Teks sumber tersebut otomatis dikeluarkan dari kisi tabel agar lebar kolom angka tidak rusak.
- Pubgen membuatkan kotak teks terpisah khusus keterangan sumber di bawah tabel dengan format cetak miring (*italic*).

---

### Aturan Kondisi (Condition Rules: Penyorotan Otomatis Baris Jumlah/Total)
Dalam lembar data angka, baris ringkasan wajib terlihat menonjol.
- Centang opsi **`[x] Condition rules (Jumlah, Tahun, Kab, Kec...)`**.
- Pubgen akan memindai kolom pertama dari setiap baris data tabel.
- Setiap kali menemukan kata kunci seperti `Jumlah`, `Total`, `Tahun`, `Kabupaten`, atau `Kecamatan`:
  - Baris tersebut otomatis diberi garis pembatas atas dan bawah sebesar 0,5pt.
  - Teks pada baris tersebut otomatis ditebalkan (*bold*).
  - Subtotal per wilayah maupun total akhir otomatis tampil rapi dan tegas.

---

### Gaya Garis Tabel
- **Horizontal Rules (Standar Booktabs - Sangat Disarankan)**:
  Format baku ilmiah dan publikasi statistik internasional:
  - Garis penutup atas tebal 1,0pt.
  - Garis pembatas header tipis 0,5pt.
  - Garis pembatas nomor kolom 0,5pt.
  - **Tanpa garis vertikal** (tampilan data bersih, mudah dibaca, dan tidak melelahkan mata).
  - Garis penutup bawah tebal 1,0pt.
- **Corporate Navy**:
  - Baris header berwarna biru gelap dengan teks putih.
  - Baris data selang-seling abu-abu muda (*zebra-striping*) untuk memudahkan pembacaan tabel yang sangat lebar.
- **Basic Grid**:
  - Garis kotak penuh di setiap batas sel tabel.

---

### Mode Penempatan Tabel
- **At page (Disarankan)**: Secara otomatis menargetkan halaman yang sedang Anda pilih di panel **Pages**, mendeteksi batas bawah judul halaman, lalu menempatkan tabel tepat di bawah judul dengan jarak aman 20pt.
- **New frame**: Menempatkan tabel di tengah halaman mulai dari batas margin atas.
- **At cursor**: Menyisipkan tabel langsung di dalam kotak teks yang sedang aktif kursornya.

---

### Berkas Excel Multi-Sheet & Ekspor Arsip ZIP
Jika Anda membuka berkas Excel yang memiliki banyak lembar (*multi-sheet* `.xlsx`):
- Pubgen membaca seluruh lembar kerja di dalam berkas tersebut.
- Muncul menu dropdown **Sheet table**: Anda dapat berpindah antar-lembar tabel (*Tabel 1.1*, *Tabel 1.2*, dst.) cukup dengan satu klik.
- **Download CSVs as ZIP...**: Tombol ini memungkinkan Anda mengunduh seluruh lembar tabel hasil konversi dalam bentuk arsip `.zip` yang tersimpan rapi di komputer Anda.


