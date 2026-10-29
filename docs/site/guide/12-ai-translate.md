## 13. Terjemahan AI & Memori Terminologi

Fitur AI opsional Pubgen berjalan melalui **Groq** (terjemahan daring) dan **Cloudflare Workers AI** (latar sampul AI, §3). Semua di bawah ini memerlukan build daring (`--features pubgen,ai-translate-online`) dan kunci yang sudah dikonfigurasi.

### Penyiapan

Buka **Publication Tools ▸ AI Translate ▸ Setup…**, atau tekan **AI Translator** di header panel. Tempel API key Groq dari [console.groq.com/keys](https://console.groq.com/keys), pilih model (**Refresh models** mengisi daftar dari akun Anda), lalu **Use AI**. Kunci disimpan di preferensi DesignCraft lokal. Dialog juga jujur soal konsekuensinya: teks katalog dikirim ke cloud Groq, dan batas laju gratis membuat pekerjaan besar lebih lambat, bukan gagal.

### Menerjemahkan halaman

Mode Dual Lang harus aktif (**Publication Tools ▸ Dual Language Mode**, `Ctrl+Shift+L`). Lalu:

- **Translate this Page** — label berubah menjadi **Translate Review on this Page** pada halaman ulasan `RXX` (judul bab dan badan kanan diganti bahasa Inggris, gaya paragraf dipertahankan) atau **Translate Table on this Page** pada halaman tabel (judul, sel header, dan kotak sumber/catatan diterjemahkan di tempat, bahasa Inggris di baris baru dengan huruf miring).
- **Translate Tables…** — terjemahan massal untuk rentang halaman, dari tombol yang sama atau **Publication Tools ▸ AI Translate ▸ Translate Table Pages…**.
- Bilah progres dan **Cancel** berjalan di latar belakang; hasil terjemahan di-cache, jadi Generate atau Translate kedua atas baris yang sama berjalan seketika.

### Memori terminologi

Glosarium resmi menjaga keluaran AI tetap memakai istilah statistik baku — *Kecamatan* → *Subdistrict*, *PDRB* → *GRDP*, *Luas Wilayah* → *Total Area*.

- Glosarium bawaan ada di `assets/pubgen/bps_glossary.json`. Urutan pencarian: jalur tersimpan di preferensi, lalu aset, lalu folder build. Berkas hilang atau rusak menjadi glosarium kosong, bukan kesalahan.
- Glosarium disuntikkan ke prompt (maksimal 200 istilah, `id = en`) dan diterapkan **secara deterministik pada sel utuh** setelah terjemahan.
- Tab **Terminology** di dialog penyiapan AI adalah editor lengkap: cari, tambah, ubah, hapus, **Import CSV…** (dua kolom — Indonesia, Inggris — dengan koma, titik koma, tab, atau pipa; header opsional), **Reset to bundled**, dan **Save…** ke `bps_glossary.json` milik Anda.
- **Collect Vocabulary** membaca halaman hasil generate (header, label baris, judul, baris catatan dan sumber, serta rangkaian frasa) dan menambahkan istilah Indonesia yang ditemukan dengan kolom Inggris kosong; setiap generate Excel melakukannya otomatis dan melaporkan `Vocabulary: N new term(s) collected`.
- **Fill AI Drafts** mengisi sel Inggris kosong itu dari model, istilah per istilah. Kotak **Auto** di sampingnya mengulang langkah ini setelah setiap generate.
- **Fix Terms** menerapkan glosarium pada terjemahan yang sudah ada di halaman terpilih; **Retranslate (Glossary)** membetulkan istilah tersebut lalu menerjemahkan ulang hanya teks yang ditandai glosarium (satu batch permintaan).

> **Tidak ada di produk:** peringkas narasi *Draft Review* yang lama sudah dihapus pada Oktober 2026. Pubgen tidak menyusun prosa bab dari angka tabel saat ini; pekerjaan AI-nya adalah terjemahan dan terminologi. Bila perlu paragraf ulasan, tulis sendiri lalu tempatkan sebagai RTF.


