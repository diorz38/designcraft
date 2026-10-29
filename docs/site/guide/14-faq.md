## 15. Tanya Jawab (FAQ) & Panduan Mengatasi Kendala

#### T: Berkas CSV saya menggunakan pemisah titik koma (;), bukan koma (,). Apakah bisa dibaca?
**J:** Tentu saja bisa! Pubgen dilengkapi dengan sensor pendeteksi otomatis yang mengenali titik koma (`;`), koma (`,`), maupun tab (`\t`). Anda tidak perlu mengubah pengaturan komputer Anda.

#### T: Mengapa Daftar Isi saya memunculkan tanda pagar (#) alih-alih nomor halaman?
**J:** Jika nomor halaman menampilkan tanda `#`, itu artinya kotak teks Daftar Isi ditaruh di Halaman Induk (*Master/Parent Page*). Pastikan Anda membuat Daftar Isi pada halaman dokumen biasa.

#### T: Apakah teks dan tabel yang dibuat oleh Pubgen masih bisa diedit secara manual?
**J:** Sangat bisa! Semua elemen yang dibuat oleh Pubgen—kotak teks, huruf, sel tabel, hingga garis vektor—adalah **objek asli DesignCraft**. Anda dapat mengklik dua kali pada teks untuk mengoreksi ketikan, mengubah lebar kolom dengan alat tabel (*Table Tool*), atau mengganti warna melalui panel *Swatches*.

#### T: Di mana berkas pengaturan dan riwayat kerja saya tersimpan jika menggunakan versi Portable?
**J:** Versi *Portable* menyertakan berkas penanda bernama `portable.txt`. Berkas ini memastikan seluruh konfigurasi, riwayat, dan pemulihan sistem tersimpan secara terisolasi di dalam folder lokal `DesignCraftData/` persis di samping berkas program `designcraft.exe`. Tidak ada data yang ditulis ke Windows Registry maupun folder `%APPDATA%`, sehingga komputer Anda tetap bersih.

---

#### T: Halaman grafik muncul sebagai gambar biasa, bukan grafik vektor. Mengapa?
**J:** Sheet `CXX_XX` digambar sebagai grafik vektor hanya bila memuat baris `chart_type` (§8). Bila baris itu tidak ada atau nilainya bukan salah satu dari sembilan arketipe, Pubgen mempertahankan perilaku lama dan menempatkan gambar dari sheet. Tambahkan `chart_type`, lalu generate ulang atau pakai **Publication ▸ Vector Chart** untuk membangun grafik dari tabel di halaman tersebut.

#### T: Bolehkah saya menyembunyikan peringatan audit yang menurut saya aman?
**J:** Boleh. Klik tombol **FP / IG** pada baris itu untuk menandainya *false positive* atau *ignored*. Baris bertag meredup, tidak dihitung lagi di ringkasan, dan bisa disembunyikan dengan **hide tagged**. Tag dikunci berdasarkan aturan dan pesan, sehingga bertahan setelah audit ulang bahkan regenerasi.

#### T: Saya memilih profil publikasi, tetapi setelah restart kembali ke BPS DDA. Apakah itu bug?
**J:** Bukan. Profil aktif dan perubahan kustomisasi hanya bertahan selama sesi; yang tersimpan bersama publikasi hanyalah `"profile"` di manifest compiler. Profil juga belum bisa diimpor atau diekspor sebagai berkas (§12).

#### T: Mengapa prosa bab saya tidak diterjemahkan atau disusun AI?
**J:** Fitur AI menerjemahkan teks halaman dan tabel serta merawat glosarium terminologi; peringkas narasi **Draft Review** yang lama sudah dihapus pada Oktober 2026. Tempatkan ulasan Anda sebagai RTF dan terjemahkan halaman itu bila perlu versi Inggrisnya (§13).

#### T: Compile tidak melakukan apa pun di build web. Mengapa?
**J:** Kompilasi membaca dan menulis berkas sehingga memerlukan aplikasi desktop; build web menyatakannya, bukan berpura-pura. Di desktop, manifest yang dokumen keluarannya sedang terbuka di tab akan dilewati agar build tidak menimpa pekerjaan Anda.

#### T: Bagaimana cara menghasilkan berkas cetak beresolusi tinggi?
**J:** Gunakan **File ▸ Export PDF…**. Anda dapat memilih PDF/X-4, PDF/X-1a:2003, atau PDF/A-2b, mengatur CMYK dan tinta spot, bleed dengan tanda crop dan bleed, kompresi gambar, serta keluaran 300 ppi untuk percetakan offset. Langkah PDF di compiler masih ekspor biasa; preset Print dan Web khusus Pubgen belum tersedia, jadi gunakan dialog ini untuk berkas PDF/X-4.

---

*DesignCraft Publication Generator (Pubgen) — Dirancang untuk Kecepatan, Keindahan, dan Ketepatan Tata Letak Statistik.*

