import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: 'Pubgen',
  description: 'Generator Publikasi Otomatis — Panduan Lengkap Pengguna',
  base: '/pubgen/',
  lang: 'id',

  srcDir: 'site',

  // Links to repo files outside docs/ (contributors.md, ROADMAP.md, etc.)
  ignoreDeadLinks: [/^\/pubgen\/..*\//, /\.\.\/(ROADMAP|contributors)/],

  // Serve docs/images (referenced by ASSETS.md / cargo xtask assets) without moving them.
  vite: { publicDir: '../images' },

  head: [['link', { rel: 'icon', type: 'image/png', href: '/pubgen/thumb01.png' }]],

  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Panduan', link: '/guide/01-pengenalan', activeMatch: '/guide/' },
      { text: 'DesignCraft', link: 'https://github.com/diorz38/pubgen' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Pengenalan',
          items: [
            { text: 'Apa itu Pubgen?', link: '/guide/01-pengenalan' },
            { text: 'Publication Cover', link: '/guide/02-cover' },
          ],
        },
        {
          text: 'Otomasi Excel',
          items: [
            { text: 'Page from CSV / Excel', link: '/guide/03-page-excel' },
            { text: 'Table from CSV / Excel', link: '/guide/04-table-excel' },
            { text: 'Master Excel', link: '/guide/06-master-excel' },
          ],
        },
        {
          text: 'Grafik & Tata Letak',
          items: [
            { text: 'Grafik Vektor & Infografis', link: '/guide/07-grafik' },
            { text: 'Utilitas Halaman & Tata Letak', link: '/guide/05-utilitas' },
            { text: 'Header & Footer Setup', link: '/guide/08-header-footer' },
          ],
        },
        {
          text: 'Produksi & Mutu',
          items: [
            { text: 'Publication Compiler', link: '/guide/09-compiler' },
            { text: 'Audit Publication', link: '/guide/10-audit' },
            { text: 'Publication Profiles', link: '/guide/11-profil' },
            { text: 'Terjemahan AI & Terminologi', link: '/guide/12-ai-translate' },
          ],
        },
        {
          text: 'Praktis',
          items: [
            { text: 'Tutorial Langkah demi Langkah', link: '/guide/13-tutorial' },
            { text: 'FAQ & Mengatasi Kendala', link: '/guide/14-faq' },
          ],
        },
      ],
    },

    socialLinks: [{ icon: 'github', link: 'https://github.com/diorz38/pubgen' }],

    search: {
      provider: 'local',
      options: {
        translations: {
          button: { buttonText: 'Cari', buttonAriaLabel: 'Cari di panduan' },
          modal: {
            noResultsText: 'Tidak ada hasil',
            resetButtonTitle: 'Hapus kriteria',
            displayDetails: 'Lihat detail',
            footer: { selectText: 'Pilih', navigateText: 'Navigasi', closeText: 'Tutup' },
          },
        },
      },
    },

    outline: { level: [2, 3], label: 'Di halaman ini' },

    docFooter: { prev: 'Sebelumnya', next: 'Berikutnya' },

    returnToTopLabel: 'Kembali ke atas',
    sidebarMenuLabel: 'Menu',
    darkModeSwitchLabel: 'Mode gelap',
    lightModeSwitchTitle: 'Mode terang',
    darkModeSwitchTitle: 'Mode gelap',
  },
})
