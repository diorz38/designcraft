# PubGen Standalone Evolution & Long-Term Future Plan

- **Target Horizon:** 2028-06-09
- **Detailed Specification:** [docs/plans/2028-06-09-future_plan.md](docs/plans/2028-06-09-future_plan.md)
- **Status:** Strategic Blueprint

---

## Overview

When **PubGen** reaches functional maturity, this blueprint details the separation of PubGen from DesignCraft into an independent, hyper-focused automated publishing engine: **PubGen Standalone** (or **PubCraft**).

### Key Pillars of the 2028-06-09 Plan:
1. **Strip Down Manual Design Tools:** Eliminate 70% of unused InDesign baggage (pen/pencil/scissors tools, bezier curve editors, EPUB reflowable converters, complex interactive dialogs).
2. **Retain the Publishing Core:** Retain HarfBuzz font shaping, high-speed line breaking & hyphenation, native vector PDF/X-4 generation, Excel matrix parsing, and grounded AI narrative synthesis.
3. **Ultra-Lightweight Footprint:** Binary shrinks to `< 15–20 MB`, cold startup `< 50 ms`, and minimal RAM usage.
4. **Three Operational Forms:**
   - **PubGen Studio:** Zero-clutter, 3-click desktop interface for statistical officers and non-designers.
   - **PubGen CLI:** Headless server runner in Docker for overnight batch publishing of all 514 regional publications without a GUI.
   - **PubGen Web Portal:** In-browser client-side compilation via Rust WebAssembly (WASM).
5. **Universal Multi-Domain Profiles:** First-class profiles for BPS statistical publications, corporate annual reports, academic monographs, and technical product catalogs.

---
*For the complete architectural design and phased transition roadmap, see [docs/plans/2028-06-09-future_plan.md](docs/plans/2028-06-09-future_plan.md).*
