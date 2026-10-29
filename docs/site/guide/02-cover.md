## 3. Bagian 1: Publication Cover (Pembuat Halaman Sampul Otomatis)

Menu **Publication Cover** memungkinkan Anda membuat halaman sampul depan buku/laporan berstandar eksekutif dengan satu kali klik. Buka dari **Publication ▸ Publication Cover**, atau gunakan kolom yang sama di alur Master Excel dan compiler (§7, §10).

Selain sampul depan, **Add Inside cover and Back Cover** menyisipkan sampul dalam abu-abu di halaman 2 dan sampul belakang di akhir. **Map Overlay** menempatkan peta administrasi dekoratif di belakang teks dan logo sampul, dari peta bawaan maupun berkas `.geojson` Anda sendiri.

![Fiture Pubgen:Publication Cover](../../images/thumb02.png)

### Ikhtisar & Posisi Halaman
- **Selalu Menjadi Halaman Depan**: Saat tombol **Generate Cover Page** diklik, Pubgen secara otomatis menyisipkan halaman baru persis sebelum Halaman 1 dan mengarahkan tampilan kanvas ke sampul tersebut. Halaman isi yang sudah ada sebelumnya tidak akan rusak atau bergeser keliru.
- **Skala Otomatis yang Presisi**: Baik dokumen Anda berukuran **A4**, **A5**, maupun **B5**, seluruh elemen vektor, bentuk poligon, jarak bingkai, dan ukuran huruf akan disesuaikan secara proporsional.

---

### 13 Gaya Sampul Eksekutif

Pubgen menyediakan 13 pilihan desain sampul siap pakai:

#### 1. Wave Curves (`🌊 Wave`)
- **Karakter Visual**: Mengalir dinamis, modern, dan luwes.
- **Elemen Desain**:
  - Dua lengkungan gelombang besar asimetris (gelombang bawah biru tua elegan dan gelombang atas putih bersih).
  - Garis kurva konsentris berwarna emas (*amber gold*).
  - Bingkai medali bulat di tengah untuk menempatkan foto pemandangan kota/daerah.
  - Penempatan blok judul dan nomor katalog yang tegas dan kontras.
- **Paling Cocok Untuk**: Laporan statistik sosial, profil daerah, lingkungan hidup, dan publikasi umum.

#### 2. Geometric Medallion (`❖ Medallion`)
- **Karakter Visual**: Kokoh, arsitektural, dan terstruktur.
- **Elemen Desain**:
  - Bidang pembagi diagonal bersudut tegas.
  - Potongan foto berbentuk lingkaran dengan cincin ganda emas dan biru gelap serta bayangan lembut (*ambient drop shadow*).
  - Pita aksen diagonal di bagian atas dan bawah.
- **Paling Cocok Untuk**: Sensus ekonomi, statistik industri, tata kota, dan laporan tahunan institusi.

#### 3. Folded Origami (`◈ Origami`)
- **Karakter Visual**: Futuristik, berdimensi faset lipatan kertas (*low-poly*), dan dinamis.
- **Elemen Desain**:
  - Poligon geometris bersudut dengan pencahayaan faset lipatan origami.
  - Foto panorama lanskap lebar yang terpasang di separuh atas halaman sampul.
  - Potongan faset emas dan biru tua di bagian kiri bawah yang menciptakan kesan arah maju.
  - Blok tipografi judul yang berani dan kontras.
- **Paling Cocok Untuk**: Publikasi perdagangan, data transportasi, inovasi daerah, dan direktori bisnis.

#### 4. Chevron & Medallion (`◆ Chevron`)
- **Karakter Visual**: Berwibawa, klasik, dan mencerminkan kelembagaan resmi.
- **Elemen Desain**:
  - Latar belakang kanvas putih bersih yang sangat elegan.
  - Pita panah (*chevron*) biru safir besar mengarah ke bawah, membingkai judul utama publikasi.
  - Medali bulat besar berisi foto lanskap kota yang dilingkari cincin emas.
  - Angka tahun raksasa (`2026`) menggunakan font **Arial Black** tepat di titik fokus medali.
  - Bilah branding instansi di bagian bawah dengan lambang resmi.
- **Paling Cocok Untuk**: Publikasi resmi pemerintah, buku rujukan statistik utama, dan edisi khusus.

#### 5. Honeycomb Hexagon (`⬡ Hexagon`)
- **Karakter Visual**: Premium, korporat, berkelas tinggi, dan mutakhir.
- **Elemen Desain**:
  - Latar belakang biru malam pekat (*Midnight Slate / Navy* `#111827`) yang memberikan kedalaman visual yang mewah.
  - Tiga garis vertikal arsitektural di sisi kanan dengan warna emas bertingkat (*opacity 85%*, *60%*, *35%*).
  - Pita panah biru safir horizontal di sisi kiri yang diberi garis aksen emas di bawahnya.
  - **Klaster Bingkai Honeycomb 12-Sudut**: Poligon tiga segi enam (*tri-hexagon*) yang menyatu, diberi bingkai tepi putih tebal 10pt, bayangan lembut, memotong foto lanskap secara presisi, serta dihiasi garis kontur emas di dalamnya.
  - Angka tahun 60pt tebal (**Arial Black**) di kiri bawah dengan garis tebal emas.
- **Paling Cocok Untuk**: Publikasi unggulan (*flagship*), rilis tahunan bergengsi, dan ringkasan eksekutif.

#### 6. Corporate Headline (`🏢 Corporate`)
- **Karakter Visual**: Modern korporat, arsitektural, bersih, dan berwibawa.
- **Elemen Desain**:
  - Kanvas putih bersih dengan garis bilah horizontal atas warna ungu (*purple* `#6c5ce7`).
  - **Position Lock**: Metadata publikasi di kanan atas (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan font **Arial Bold** rapi rata kanan).
  - Kotak aksen ungu di kanan atas dengan bayangan halus elegan.
  - Bingkai foto gedung/arsitektur vertikal di sisi tengah-kanan dengan siku aksen ungu (*L-bracket*) di sudut kiri atas foto.
  - Tampilan angka tahun dwi-warna di tengah-kiri (`20` berwarna abu gelap `#1e293b`, `26` berwarna ungu `#6c5ce7`) dengan **Volume** (`Volume 23 2026`) tepat di bawahnya dan garis bawah tebal.
  - Blok judul utama warna ungu semi-transparan (opasitas 85%) di kiri bawah dengan sudut kanan-atas melengkung halus (*rounded corner*), memperlihatkan siluet foto gedung perkotaan di baliknya sambil membingkai teks judul putih tebal (**Arial Black**).
  - **Position Lock**: Logo instansi resmi di kiri bawah bersanding dengan teks nama instansi resmi (**Arial Bold Italic**).
  - Blok aksen sudut kanan bawah dan tab sisi kanan.
- **Paling Cocok Untuk**: Laporan bisnis tahunan, profil wilayah, publikasi statistik ekonomi kota/daerah modern.

#### 7. Diagonal Arrow (`↗ Arrow`)
- **Karakter Visual**: Dinamis, berarah maju (*forward motion*), dan korporat modern.
- **Elemen Desain**:
  - Kanvas putih bersih dengan panel foto gedung perkotaan setinggi penuh di sisi kanan hingga tepi halaman (`[286, 0, 595, 842]`).
  - Pita diagonal biru navy pekat yang melintang miring di bagian atas (`[113, 0, 400, 0, 287, 256, 0, 256]`), tepi bawahnya memotong foto.
  - Segitiga aksen jingga di sudut kiri atas (`[0, 0, 87, 0, 0, 196]`), terpisah dari pita diagonal oleh saluran tipis berwarna putih.
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan font **Arial Bold**) berada di kanan atas, di atas foto.
  - Pita panah biru navy yang melintas dari area putih menembus masuk ke dalam foto (`[0, 519, 352, 591, 0, 663]`), dilengkapi bayangan halus.
  - Tahun publikasi dicetak 44pt **Arial Black** dan dipusatkan di dalam pita panah tersebut, dengan baris **Volume** (`Volume 23 2026`) tepat di bawahnya.
  - Tumpukan judul di tengah-kiri dengan **Arial Black**: jenjang wilayah, judul publikasi berwarna aksen dalam satu baris selebar halaman, dan `DALAM ANGKA` di bawahnya.
  - **Position Lock**: logo instansi resmi di kiri bawah bersanding dengan teks nama instansi resmi (**Arial Bold Italic**).
- **Paling Cocok Untuk**: Laporan tahunan korporat, tinjauan bisnis, profil kota, dan publikasi industri/investasi.

#### 8. Modern Horizon (`🌇 Horizon`)
- **Karakter Visual**: Editorial, institusional, dan khas sampul laporan tahunan modern.
- **Elemen Desain**:
  - **Foto pemandangan kota penuh (*full-bleed*)** di bagian atas, terpotong oleh lengkung lebar yang membentang dari separuh bawah tepi kiri ke tengah tepi kanan (geometri jiplakan `design-5.designcraft`).
  - **Bidang aksen jingga** yang mengikuti sisi bawah lengkung tersebut: tebal di tepi kiri lalu menipis dan menyatu dengan blok aksen di sisi kanan, sehingga keduanya terbaca sebagai satu bidang.
  - **Blok institusional gelap** memenuhi bagian bawah halaman dengan sudut kiri atas membulat besar; sambungan putih tipis memisahkan foto, bidang aksen, dan blok gelap.
  - **Blok aksen kanan** memuat angka tahun publikasi besar dengan **Arial Black**, dan baris **Volume** tepat di bawahnya.
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan font **Arial Bold**) berada di kanan atas, di atas foto.
  - Tumpukan judul di tengah-kiri dengan **Arial Black** di dalam blok gelap: jenjang wilayah, judul publikasi berwarna aksen (satu kata per baris), dan `DALAM ANGKA` di bawahnya.
  - **Position Lock**: logo instansi resmi di kiri bawah bersanding dengan teks nama instansi resmi (**Arial Bold Italic**).
  - Palet bawaannya (**Report Navy & Orange**, `#082641` + `#f5871f`) mengikuti warna sampul referensi; memilih gaya ini otomatis memilih tema tersebut, dan tema lain tetap dapat dipakai.
- **Paling Cocok Untuk**: Buku tahunan statistik, profil daerah dalam angka, laporan institusional, dan publikasi pemerintah.

#### 9. Business Report (`📊 Report`)
- **Karakter Visual**: Tegas, korporat, dan kontras tinggi bergaya bisnis-ke-bisnis.
- **Elemen Desain**:
  - Kanvas putih bersih; **foto pencakar langit berbentuk panah** (ditelusuri dari `design-10.designcraft`) dengan bayangan lembut — ditambah **serpihan chevron**-nya, dipisahkan celah putih tipis namun membawa gambar yang sama, sehingga bangunan berlanjut melintasi celah.
  - **Pita jingga di sepanjang tepi atas** dan **sudut jingga** di kanan bawah membingkai komposisi.
  - **Bidang navy memotong di kiri bawah, terselip di belakang foto**, dengan **medali lingkaran putih** berisi tahun dwi-warna (setengah pertama gelap, setengah kedua aksen) dalam **Arial Black** raksasa dan baris **Volume** tepat di bawahnya.
  - Tumpukan judul kiri-atas dengan **Arial Black**: jenjang wilayah (navy), judul publikasi (jingga) dalam satu baris penuh, dan `DALAM ANGKA` (navy).
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan font **Arial Bold**, putih) berada di kanan atas, di atas pita jingga.
  - Angka tahun dwi-warna dalam **Arial Black** di dalam medali, dengan baris **Volume** tepat di bawahnya.
  - **Position Lock**: logo instansi resmi bersanding dengan teks nama instansi resmi (**Arial Bold Italic**, putih) pada bidang navy di kiri bawah.
  - Palet bawaannya (**Report Navy & Orange**, `#082641` + `#f5871f`); memilih gaya ini otomatis memilih tema tersebut, dan tema lain tetap dapat dipakai.
- **Paling Cocok Untuk**: Laporan bisnis, tinjauan tahunan korporat, publikasi investasi dan industri, serta profil kota.

#### 10. Urban Angle (`🌆 Urban`)
- **Karakter Visual**: Bersudut, perkotaan, dan editorial — susunan bidang warna datar yang diagonal.
- **Elemen Desain**:
  - **Baji jingga** mengisi sudut kiri atas dan memotong secara diagonal ke tepi kiri.
  - **Pita diagonal navy** dari tepi atas ke margin kiri, membawa angka tahun.
  - **Foto panorama kota** mengisi panel-panel kanan, terpotong oleh pita dan baji bawah.
  - **Baji navy** naik dari sudut kanan bawah.
  - **Tahun raksasa Arial Black** dwi-warna (putih + jingga) yang **diputar** mengikuti arah pita, dengan baris **Volume** di sampingnya.
  - Tumpukan judul pada panel putih: tingkat wilayah, judul publikasi dengan warna aksen (satu kata per baris), dan `DALAM ANGKA`.
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>`) berada di kanan atas di atas foto.
  - **Position Lock**: logo instansi resmi di kiri atas bersanding dengan teks nama instansi resmi (**Arial Bold Italic**) di atas baji jingga.
  - Palet bawaannya (navy petrol + marigold, `#034460` + `#fcb91e`); memilih gaya ini otomatis memilih tema tersebut, dan tema lain tetap dapat dipakai.
- **Paling Cocok Untuk**: Profil kota dan daerah, laporan pembangunan perkotaan, sampul korporat modern.

#### 11. Tri-Color Bands (`🎨 Tri-Color`)
- **Karakter Visual**: Eksekutif, tertata, kelembagaan — sistem warna datar alih-alih ilustrasi.
- **Elemen Desain**:
  - **Garis masthead tiga warna** di tepi atas: biru (`#0088D2`), hijau (`#74BF45`), dan jingga (`#F38B1E`), diulang sebagai garis di atas pita instansi.
  - **Panel panorama kota** di sisi kanan yang melewati tepi kanan-atas, digarisbawahi batang biru/hijau/jingga.
  - Batang aksen biru di atas judul; judul dalam **Arial Black** warna tinta, baris judul kedua berwarna jingga.
  - **Tahun dwi-warna** (biru + tinta) dalam **Arial Black** raksasa, dengan baris **Volume** di bawahnya dan garis hijau.
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan **Arial Bold**) berada di kanan atas pada area masthead putih.
  - **Position Lock**: pita instansi biru solid di bagian bawah, berisi logo di atas pelat putih dan teks instansi (**Arial Bold Italic**, putih).
- **Paling Cocok Untuk**: Publikasi statistik pemerintah, laporan tahunan, profil korporat, dan sampul yang harus memakai palet kelembagaan yang ketat.

#### 12. Prism Diamonds (`💎 Prism`)
- **Karakter Visual**: Seperti permata, modern, dramatis — sampul malam yang dibangun dari satu bentuk berulang.
- **Elemen Desain**:
  - **Bidang malam pekat** (`#0A2233`) dengan massa belah ketupat teal gelap di belakang foto.
  - **Rangkaian bentuk belah ketupat** dalam sebelas ukuran, dari serpihan 12 pt hingga foto 300 pt: biru, hijau, dan jingga solid, warna-warna pucat, serta dua garis tepi tipis, bergerak dari kiri atas ke kanan bawah dan melewati tepi kanan.
  - **Foto panorama kota dipotong sebagai belah ketupat terbesar**, sudut kiri bawahnya ditumpuk belah ketupat biru muda dan satu serpihan putih.
  - Judul dalam **Arial Black** putih es, baris judul kedua berwarna jingga dengan jarak huruf lebar, dan tanda tiga belah ketupat di sudut kiri atas.
  - **Tahun dwi-warna** (biru + putih) dalam **Arial Black** raksasa dengan baris **Volume** di bawahnya dan garis hijau.
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan **Arial Bold**) berada di kanan atas di atas bidang malam.
  - **Position Lock**: sambungan hijau di atas pita instansi biru solid di bagian bawah, berisi logo di atas pelat putih, teks instansi (**Arial Bold Italic**, putih), dan tiga belah ketupat penutup.
  - Templat membawa swatch-nya sendiri (`Prism Night`, `Prism Deep`, `Prism Blue`, `Prism Green`, `Prism Orange`, `Prism Mint`, `Prism Ice`, `Prism Sky`), sehingga menu Theme tidak mewarnai ulang sampul ini.
- **Paling Cocok Untuk**: Buku tahunan statistik unggulan, laporan tahunan, profil korporat, dan sampul yang membutuhkan tampilan kontemporer yang mencolok.

#### 13. Quarter Stack (`▢ Quarter Stack`)
- **Karakter Visual**: Hangat, taktil, editorial — seperti setumpuk kartu di atas meja kertas.
- **Elemen Desain**:
  - **Bidang kertas hangat** (`#F6F1E7`) alih-alih putih, dengan metadata dan tiga chip kecil bersudut satu di sudut kiri atas.
  - **Tumpukan empat kartu terkembang** dengan selisih 30 pt, masing-masing membulatkan tepat **satu** sudut (kanan-bawah, kiri-atas, kanan-atas, lalu kartu foto dengan kiri-bawah beradius 110 pt) dalam warna blush `#E7C7B3`, sage `#93A98C`, clay `#C75B39` dan foto, melewati tepi kanan.
  - **Gugusan chip bertingkat** di bawah tumpukan — chip teal `#2F6F6B`, marigold `#E8A33D`, blush dan clay dengan empat ukuran berbeda — menggemakan kartu dalam skala kecil.
  - Judul dalam **Arial Black** warna tinta `#1F2A44`, baris judul kedua clay dengan jarak huruf lebar, dan **tahun dwi-warna** (clay + tinta) dalam **Arial Black** raksasa dengan baris **Volume** di bawahnya serta garis sage.
  - **Position Lock**: metadata publikasi (Baris 1: `Katalog BPS: <nomor>`, Baris 2: `ISSN/ISBN <nomor>` dengan **Arial Bold**) berada di kiri atas pada bidang kertas.
  - **Position Lock**: pita instansi tinta solid di bagian bawah, berisi logo di atas pelat kertas dengan satu sudut membulat, teks instansi (**Arial Bold Italic**, kertas), dan tiga chip penutup.
  - Templat membawa swatch hangatnya sendiri (`Stack Paper`, `Stack Ink`, `Stack Clay`, `Stack Marigold`, `Stack Sage`, `Stack Blush`, `Stack Teal`), sehingga menu Theme tidak mewarnai ulang sampul ini.
- **Paling Cocok Untuk**: Buku tahunan, profil kelembagaan, laporan sosial dan budaya, serta sampul yang sebaiknya terasa seperti cetakan alih-alih korporat.

---

### Pembuatan Sampul AI Base (Online / Gaya ≥ 100)

Pubgen menghadirkan **Sampul AI Base**, yang memadukan latar belakang seni generatif kecerdasan buatan (*text-to-image AI*) dengan tipografi vektor tajam, pelat identitas instansi, dan tata letak geometris presisi milik DesignCraft.

```
+-------------------------------------------------------------------+
| Publication Cover                                                 |
| Style: [ ── AI Base (online) ──                                 ] |
|        [ Retro Waves           ▼ ]                                |
| Palette:[ Warm Terracotta ] [ Pastel Dream ] [ Mono Slate ] [ Forest Gold ]|
|                                                                   |
| Title:  [ STATISTIK DAERAH                                  ]      |
| Title 2:[ KABUPATEN KARAWANG                                ]      |
| Year:   [ 2026        ]  Volume: [ Volume 23 2026          ]      |
|                                                                   |
| [  Generate Cover Page  ]                                         |
+-------------------------------------------------------------------+
```

#### 3 Gaya Kurasi AI
1. **Retro Waves (`AI Retro Waves`, Gaya 100)**:
   - **Karakter Visual**: Mengalir, organik, bernuansa risograf 1970-an dan lengkungan editorial modern.
   - **Komposisi**: Pita melengkung retro dengan ruang negatif (*negative space*) lapang yang dirancang khusus menjaga kontras teks tetap tajam dan mudah dibaca.
   - **Paling Cocok Untuk**: Publikasi statistik umum, laporan sosial-budaya, ulasan lingkungan dan ringkasan daerah.
2. **Geometric Business (`AI Geometric Business`, Gaya 101)**:
   - **Karakter Visual**: Bersih, berwibawa, geometri bisnis modern vektor 2D datar.
   - **Komposisi**: Garis tebal melengkung diagonal melintasi bentangan sampul, heksagon mengambang bersudut membulat, bingkai aksen geometris, serta ruang kosong yang lega untuk judul teks resmi.
   - **Paling Cocok Untuk**: Buku statistik daerah (KCDA), laporan dinas/kementerian pemerintah, publikasi tahunan resmi.
3. **Minimal Sci-Fi (`AI Minimal Sci-Fi`, Gaya 102)**:
   - **Karakter Visual**: Futuristik berteknologi tinggi, estetika cetak biru (*blueprint*), gradien arsitektural terstruktur.
   - **Komposisi**: Garis struktural rapi, aksen grid isometrik halus, dan ruang bidang terbuka yang luas.
   - **Paling Cocok Untuk**: Statistik industri, ekonomi digital, sensus, direktori ilmiah dan inovasi.

#### 6 Palet Warna Serasi
Setiap gaya AI dapat dipadukan dengan 6 palet pilihan:
- **Warm Terracotta**: Terakota kaya (`#d96b43`), oker hangat (`#e8a838`), dan abu-abu arang (`#2c3539`).
- **Pastel Dream**: Lavender lembut (`#a29bfe`), koral cerah (`#fab1a0`), dan ungu malam (`#2d3436`).
- **Mono Slate**: Biru baja (`#4b6584`), abu-abu terang halus (`#a5b1c2`), dan grafit gelap (`#1e272e`).
- **Forest Gold**: Hijau zamrud (`#20bf6b`), emas anggun (`#f7b731`), dan hijau hutan pinus (`#0b3b24`).
- **Corporate Blue**: Biru dongker gelap (`#0a1931`), biru royal (`#003399`), dan biru sian elektrik (`#00a8ff`).
- **Teal Lime**: Hijau teal gelap (`#006d6f`), hijau pinus (`#0a2e28`), dan hijau limau cerah (*chartreuse*) (`#84cc16`).

#### Konfigurasi Cloudflare Workers AI & Kredensial
Sampul AI Base ditenagai oleh **Cloudflare Workers AI** menggunakan model Stable Diffusion XL (SDXL):
1. Klik tombol **AI Setup…** di dialog Cover (atau di samping tombol Translate).
2. Masukkan **Cloudflare Account ID** dan **Cloudflare API Token** Anda.
3. **Cakupan Token (Scope)**: Buat token di dasbor Cloudflare dengan hak akses khusus `Workers AI: Edit` saja. Jangan gunakan hak akses administrator menyeluruh.
4. Klik **Verify**: Pubgen melakukan uji coba kecil (1 langkah) untuk memverifikasi kredensial tanpa memboroskan kuota. Token disimpan secara lokal di preferensi pengguna Anda, tidak pernah disisipkan ke dalam dokumen, dan tidak pernah diunggah ke repositori.
5. **Catatan Privasi**: Hanya templat deskripsi gaya seni abstrak yang dikirim ke Cloudflare. Judul publikasi, nama instansi, dan data rahasia daerah tetap berada 100% di komputer lokal Anda dan ditata sebagai bingkai teks vektor asli.

#### Ekonomi Kuota & Pengecekan Awal (*Pre-Flight*)
- **Konsumsi Neuron**: Setiap sampul SDXL 18 langkah mengonsumsi sekitar **~220 Neuron**. Cloudflare memberikan alokasi gratis sebesar **10.000 Neuron per hari** (diperbarui setiap pukul 00:00 UTC).
- **Kapasitas Batch Aman**: Kuota harian gratis ini mencukupi pembuatan hingga sekitar **~45 sampul publikasi per hari**.
- **Peringatan Pre-Flight**: Ketika Anda memulai pembuatan batch lebih dari 45 sampul dari buku kerja Excel, Pubgen akan menampilkan konfirmasi: *"45 of N fit today's free quota; the rest will fail after ~45 — continue?"*.
- **Seed Judul Deterministik**: Pubgen menghasilkan *seed* acak 32-bit yang stabil dari *hash* judul publikasi (`hash(title)`). Publikasi dalam seri yang sama menghasilkan latar belakang seni yang konsisten dan serasi saat dibuat ulang.
- **Stop-on-429 & Pelaporan**: Jika kuota harian habis di tengah jalan (HTTP 429), Pubgen berhenti secara rapi tanpa *crash*, mempertahankan semua sampul yang telah selesai dibuat, dan melaporkan daftar publikasi yang belum memiliki sampul.
- **Otomatisasi Lanjutan (Skip Existing)**: Menjalankan ulang proses batch akan secara otomatis mengenali halaman sampul yang telah ada (melalui label `#ai_preset:` pada bingkai latar belakang) dan hanya memproses publikasi yang belum selesai.

#### Integrasi Excel (Lembar `COVER` / `COVERS`)
Anda dapat menentukan gaya dan palet AI langsung pada lembar Excel:
- **Kolom Style**: `AI Retro Waves`, `AI Geometric Business`, `AI Minimal Sci-Fi`, `AI Dynamic Waves Spread`, atau `AI Vibrant S-Curve Spread` (alias lama `Dark Fantasy` tetap didukung untuk kompatibilitas ke belakang).
- **Kolom Palette**: `Teal Lime`, `Corporate Blue`, `Forest Gold`, `Mono Slate`, `Pastel Dream`, atau `Warm Terracotta`.

---

### Pratinjau Thumbnail Hover
- Di bawah menu pilihan gaya, terdapat tombol pil cepat untuk seluruh 13 gaya vektor korporat ditambah 3 gaya AI Base.
- Cukup **arahkan kursor tetikus (hover)** ke salah satu pil (atau klik ikon mata `👁` di samping menu). Jendela melayang (*tooltip*) akan langsung menampilkan thumbnail foto sampul yang sesungguhnya beresolusi tinggi, sehingga Anda dapat memilih desain dengan yakin sebelum membuatnya.

---

### Tema Warna & Kustomisasi Palet
Anda dapat memilih tema baku atau memasukkan kode warna heksadesimal sendiri:
- **Primary Color** (Warna Utama, bawaan: `#0072bc` / Biru Safir): Untuk pita judul utama, panah chevron, dan elemen bidang dominan.
- **Accent Color** (Warna Aksen, bawaan: `#f5ab19` / Emas Amber): Untuk garis sorotan, cincin medali, label kategori, dan garis bawah angka tahun.
- **Dark Color** (Warna Gelap, bawaan: `#111827` / Biru Malam): Untuk latar belakang kanvas gelap dan teks berbobot kuat.

---

### Metadata & Isian Informasi Publikasi
Lengkapi data publikasi Anda:
- **Title**: Judul sampul (contoh: `KARAWANG TIMUR`). Diketik pada kotak multi-baris dan diletakkan ke halaman apa adanya. Pubgen akan memformat teks ini dengan font **Arial Black**.
- **Title 2**: Baris judul kedua (contoh: `DALAM ANGKA`). Berada pada bingkai milik desain sampul, di bawah judul utama dengan gaya tersendiri — lebih kecil dan berwarna lain. Isian ini diawali teks dummy `Title 2`.
- **Year**: Masukkan tahun publikasi (contoh: `2026`). Ditampilkan menonjol dengan font tebal **Arial Black**.
- **Volume**: Masukkan nomor edisi/volume (contoh: `Volume 23 2026`).
- **Catalog BPS**: Masukkan nomor katalog resmi (contoh: `1102001.3215113`). Pubgen otomatis menambahkan awalan `Katalog BPS:` jika Anda belum mengetikkannya.
- **Kode Terbitan**: Pilih **ISSN** atau **ISBN** lalu ketikkan nomor serinya (contoh: `2828-7436`).
- **Teks Instansi**: Masukkan nama institusi (contoh: `BADAN PUSAT STATISTIK\nKABUPATEN KARAWANG`). Diformat rapi menggunakan **Arial Bold Italic**. Gunakan `\n` untuk memisahkan baris nama instansi dan nama daerah.

---

### Pemilih Gambar Kustom & Logo
Pubgen secara otomatis menampilkan kotak pemilih gambar yang relevan dengan gaya yang sedang aktif:
- **Skyline Image**: Untuk sampul bergaya *Wave Curves* dan *Chevron & Medallion*.
- **Landscape Image**: Untuk sampul bergaya *Folded Origami* dan *Geometric Medallion*.
- **Honeycomb Image**: Untuk sampul bergaya *Honeycomb Hexagon*. Foto Anda akan otomatis dipotong rapi mengikuti bentuk segi enam bersarang (*tri-hexagon*).
- **Photo**: Untuk sampul bergaya *Corporate Headline*, *Diagonal Arrow*, *Modern Horizon*, dan *Tri-Color Bands*. Foto arsitektur perkotaan vertikal atau gedung landmark. Pada gaya *Diagonal Arrow* foto mengisi panel setinggi penuh di sisi kanan, di belakang pita diagonal dan panah putih. Pada gaya *Modern Horizon* foto dipotong mengikuti lengkung lebar di bagian atas halaman, jadi pilih pemandangan kota lebar dengan garis horizon yang menarik.
- **Logo**: Klik tombol **Browse...** untuk memilih berkas logo instansi (format `.png`, `.jpg`, atau `.webp`). Logo akan disematkan secara proporsional tanpa gepeng/terdistorsi.

---

### Membuat Halaman Sampul
Setelah semua kolom terisi, klik tombol biru:
👉 **[ Generate Cover Page ]**

Hanya dalam sekejap (kurang dari 0,1 detik):
1. Halaman sampul baru disisipkan di posisi paling awal buku Anda.
2. Vektor bentuk, poligon, bayangan, dan garis batas dibuat secara presisi.
3. Foto pemandangan dan logo diposisikan dan dipotong rapi.
4. Teks judul, nomor katalog, dan identitas instansi dirender menggunakan hierarki tipografi resmi (**Arial Black**, **Arial Bold**, **Arial Bold Italic**).
5. Layar kanvas otomatis berpindah ke halaman sampul yang baru selesai dibuat.


