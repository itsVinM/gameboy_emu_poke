<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Game Boy Emulator in Rust — Deep Dive</title>
<link href="https://fonts.googleapis.com/css2?family=Share+Tech+Mono&family=VT323&family=Rajdhani:wght@400;600;700&display=swap" rel="stylesheet">
<style>
  :root {
    --gb-green:    #9bbc0f;
    --gb-dark:     #0f380f;
    --gb-mid:      #306230;
    --gb-light:    #8bac0f;
    --gb-screen:   #0d1f0d;
    --amber:       #ffb347;
    --red-accent:  #ff4444;
    --bg:          #0a0a0a;
    --surface:     #111a11;
    --surface2:    #162316;
    --border:      #1e3a1e;
    --text:        #c8d8a8;
    --text-dim:    #6a8a5a;
    --text-bright: #d8f098;
    --code-bg:     #0d1a0d;
    --scanline-opacity: 0.03;
  }

  * { margin: 0; padding: 0; box-sizing: border-box; }

  html { scroll-behavior: smooth; }

  body {
    background: var(--bg);
    color: var(--text);
    font-family: 'Rajdhani', sans-serif;
    font-size: 17px;
    line-height: 1.75;
    min-height: 100vh;
    position: relative;
    overflow-x: hidden;
  }

  /* Scanline overlay */
  body::before {
    content: '';
    position: fixed;
    inset: 0;
    background: repeating-linear-gradient(
      0deg,
      transparent,
      transparent 2px,
      rgba(0,0,0,var(--scanline-opacity)) 2px,
      rgba(0,0,0,var(--scanline-opacity)) 4px
    );
    pointer-events: none;
    z-index: 9999;
  }

  /* Background grid */
  body::after {
    content: '';
    position: fixed;
    inset: 0;
    background-image:
      linear-gradient(rgba(155,188,15,0.03) 1px, transparent 1px),
      linear-gradient(90deg, rgba(155,188,15,0.03) 1px, transparent 1px);
    background-size: 32px 32px;
    pointer-events: none;
    z-index: 0;
  }

  /* ── Layout ── */
  .wrapper {
    display: grid;
    grid-template-columns: 280px 1fr;
    min-height: 100vh;
    position: relative;
    z-index: 1;
  }

  /* ── Sidebar ── */
  nav {
    position: sticky;
    top: 0;
    height: 100vh;
    overflow-y: auto;
    background: var(--surface);
    border-right: 1px solid var(--border);
    padding: 2rem 0;
    scrollbar-width: thin;
    scrollbar-color: var(--gb-mid) transparent;
  }

  .nav-logo {
    padding: 0 1.5rem 2rem;
    border-bottom: 1px solid var(--border);
    margin-bottom: 1.5rem;
  }

  .nav-logo .pixel-icon {
    display: block;
    font-family: 'VT323', monospace;
    font-size: 2.8rem;
    color: var(--gb-green);
    line-height: 1;
    text-shadow: 0 0 20px rgba(155,188,15,0.5);
    letter-spacing: 2px;
  }

  .nav-logo small {
    display: block;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.65rem;
    color: var(--text-dim);
    margin-top: 0.25rem;
    letter-spacing: 3px;
    text-transform: uppercase;
  }

  nav ul { list-style: none; padding: 0 1rem; }
  nav ul li { margin: 2px 0; }

  nav ul li a {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    color: var(--text-dim);
    text-decoration: none;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 0.5px;
    border-radius: 4px;
    border-left: 2px solid transparent;
    transition: all 0.15s;
  }

  nav ul li a:hover {
    color: var(--gb-green);
    background: rgba(155,188,15,0.07);
    border-left-color: var(--gb-green);
  }

  nav ul li a .num {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.7rem;
    color: var(--gb-mid);
    min-width: 1.5rem;
  }

  /* ── Main content ── */
  main {
    padding: 4rem 5rem 6rem;
    max-width: 900px;
  }

  /* ── Header ── */
  .hero {
    margin-bottom: 5rem;
    padding-bottom: 3rem;
    border-bottom: 1px solid var(--border);
    position: relative;
  }

  .hero::before {
    content: 'DOCUMENTATION';
    position: absolute;
    top: -1rem;
    right: 0;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.65rem;
    letter-spacing: 5px;
    color: var(--gb-mid);
  }

  .hero h1 {
    font-family: 'VT323', monospace;
    font-size: 4.5rem;
    color: var(--gb-green);
    line-height: 1;
    text-shadow: 0 0 40px rgba(155,188,15,0.3);
    margin-bottom: 0.5rem;
  }

  .hero .subtitle {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.9rem;
    color: var(--amber);
    letter-spacing: 2px;
    margin-bottom: 1.5rem;
  }

  .hero p {
    color: var(--text-dim);
    font-size: 1rem;
    max-width: 600px;
  }

  /* ── Sections ── */
  section {
    margin-bottom: 5rem;
    scroll-margin-top: 2rem;
  }

  h2 {
    font-family: 'VT323', monospace;
    font-size: 2.4rem;
    color: var(--gb-green);
    margin-bottom: 0.25rem;
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    text-shadow: 0 0 20px rgba(155,188,15,0.2);
  }

  h2 .section-num {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.7rem;
    color: var(--gb-mid);
    letter-spacing: 2px;
    border: 1px solid var(--gb-mid);
    padding: 2px 6px;
    border-radius: 3px;
    position: relative;
    top: -4px;
  }

  h3 {
    font-family: 'Rajdhani', sans-serif;
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--amber);
    margin: 2rem 0 0.75rem;
    letter-spacing: 1px;
    text-transform: uppercase;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  h3::before {
    content: '▶';
    font-size: 0.7rem;
    color: var(--gb-mid);
  }

  h4 {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.9rem;
    color: var(--gb-light);
    margin: 1.5rem 0 0.5rem;
    letter-spacing: 1px;
  }

  p { margin-bottom: 1rem; color: var(--text); }

  /* ── Code blocks ── */
  pre {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-left: 3px solid var(--gb-mid);
    border-radius: 0 6px 6px 0;
    padding: 1.5rem;
    overflow-x: auto;
    margin: 1.25rem 0;
    position: relative;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.82rem;
    line-height: 1.7;
    color: var(--gb-green);
    scrollbar-width: thin;
    scrollbar-color: var(--gb-mid) transparent;
  }

  pre::before {
    content: 'RUST';
    position: absolute;
    top: 0.5rem;
    right: 0.75rem;
    font-size: 0.6rem;
    letter-spacing: 3px;
    color: var(--text-dim);
    opacity: 0.5;
  }

  code {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.85em;
    color: var(--amber);
    background: rgba(255,179,71,0.08);
    padding: 1px 6px;
    border-radius: 3px;
    border: 1px solid rgba(255,179,71,0.15);
  }

  pre code {
    background: none;
    border: none;
    padding: 0;
    color: inherit;
    font-size: inherit;
  }

  /* ── Comment highlighting in code ── */
  pre .comment { color: var(--text-dim); font-style: italic; }
  pre .keyword { color: #ff8c69; }
  pre .string  { color: #98d873; }
  pre .number  { color: #d4a8ff; }
  pre .func    { color: var(--amber); }

  /* ── Memory map block ── */
  .memory-map {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1.5rem;
    margin: 1.25rem 0;
    overflow-x: auto;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.8rem;
    line-height: 2;
  }

  .memory-map .addr  { color: var(--amber); }
  .memory-map .size  { color: var(--text-dim); }
  .memory-map .name  { color: var(--gb-green); font-weight: bold; }
  .memory-map .desc  { color: var(--text-dim); }
  .memory-map .sep   { color: var(--gb-mid); }

  /* ── Tables ── */
  .table-wrap { overflow-x: auto; margin: 1.25rem 0; }

  table {
    width: 100%;
    border-collapse: collapse;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.82rem;
  }

  thead tr {
    background: var(--surface2);
    border-bottom: 2px solid var(--gb-mid);
  }

  th {
    padding: 0.6rem 1rem;
    text-align: left;
    color: var(--amber);
    font-weight: 400;
    letter-spacing: 1px;
    text-transform: uppercase;
    font-size: 0.75rem;
  }

  td {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--border);
    color: var(--text);
    vertical-align: top;
  }

  td:first-child { color: var(--amber); }
  td:nth-child(2) { color: var(--text-dim); }

  tr:hover td { background: rgba(155,188,15,0.03); }

  /* ── Callout / tip boxes ── */
  .callout {
    border: 1px solid var(--gb-mid);
    border-left: 4px solid var(--gb-green);
    background: rgba(155,188,15,0.04);
    border-radius: 0 6px 6px 0;
    padding: 1rem 1.25rem;
    margin: 1.5rem 0;
    font-size: 0.95rem;
  }

  .callout .callout-title {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.7rem;
    letter-spacing: 3px;
    color: var(--gb-green);
    margin-bottom: 0.4rem;
    text-transform: uppercase;
  }

  .callout.amber {
    border-color: rgba(255,179,71,0.4);
    border-left-color: var(--amber);
    background: rgba(255,179,71,0.04);
  }

  .callout.amber .callout-title { color: var(--amber); }

  /* ── Bit diagram ── */
  .bit-diagram {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1.5rem;
    margin: 1.25rem 0;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.82rem;
    line-height: 1.8;
    overflow-x: auto;
  }

  .bit-diagram .bit-row { display: flex; gap: 0; margin-bottom: 0.25rem; }
  .bit-diagram .bit-header {
    color: var(--text-dim);
    font-size: 0.7rem;
    margin-bottom: 0.5rem;
    letter-spacing: 1px;
  }

  .bit-box {
    width: 2.8rem;
    height: 2.2rem;
    border: 1px solid var(--gb-mid);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--gb-green);
    font-size: 0.75rem;
    margin: 1px;
    border-radius: 3px;
  }

  .bit-box.set { background: rgba(155,188,15,0.15); }
  .bit-box.label { color: var(--amber); border-color: rgba(255,179,71,0.3); background: rgba(255,179,71,0.05); }

  /* ── Register visual ── */
  .register-visual {
    display: flex;
    gap: 0;
    margin: 1rem 0;
    flex-wrap: wrap;
  }

  .reg-field {
    border: 1px solid var(--gb-mid);
    padding: 0.35rem 0.6rem;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.75rem;
    text-align: center;
    position: relative;
  }

  .reg-field .reg-name { color: var(--amber); display: block; }
  .reg-field .reg-bits { color: var(--text-dim); font-size: 0.6rem; }

  /* ── Golden rule box ── */
  .golden-rule {
    border: 2px solid var(--gb-green);
    border-radius: 8px;
    padding: 2rem;
    margin: 3rem 0;
    text-align: center;
    position: relative;
    background: rgba(155,188,15,0.03);
  }

  .golden-rule::before {
    content: '◆ THE GOLDEN RULE ◆';
    position: absolute;
    top: -0.7rem;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg);
    padding: 0 1rem;
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.65rem;
    letter-spacing: 4px;
    color: var(--gb-green);
  }

  .golden-rule p {
    font-family: 'VT323', monospace;
    font-size: 1.8rem;
    color: var(--gb-green);
    text-shadow: 0 0 20px rgba(155,188,15,0.3);
    line-height: 1.3;
    margin: 0;
  }

  /* ── Quick ref table ── */
  .qref td:first-child  { color: var(--gb-light); }
  .qref td:nth-child(2) { color: var(--text-dim); }
  .qref td:nth-child(3) { color: var(--amber); font-family: 'Share Tech Mono', monospace; }
  .qref td:nth-child(4) { color: var(--text); }

  /* ── Scrollbar ── */
  ::-webkit-scrollbar { width: 6px; height: 6px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--gb-mid); border-radius: 3px; }

  /* ── TOC divider ── */
  .toc-group {
    font-family: 'Share Tech Mono', monospace;
    font-size: 0.6rem;
    letter-spacing: 4px;
    color: var(--gb-mid);
    text-transform: uppercase;
    padding: 1rem 1.5rem 0.25rem;
  }

  /* ── Responsive ── */
  @media (max-width: 900px) {
    .wrapper { grid-template-columns: 1fr; }
    nav { position: static; height: auto; }
    main { padding: 2rem 1.5rem; }
    .hero h1 { font-size: 3rem; }
  }

  /* ── Animations ── */
  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .cursor::after {
    content: '█';
    animation: blink 1s step-end infinite;
    color: var(--gb-green);
    font-size: 0.8em;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(12px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  section { animation: fadeIn 0.4s ease both; }
  section:nth-child(2)  { animation-delay: 0.05s; }
  section:nth-child(3)  { animation-delay: 0.10s; }
  section:nth-child(4)  { animation-delay: 0.15s; }
  section:nth-child(5)  { animation-delay: 0.20s; }

  ul, ol {
    padding-left: 1.5rem;
    margin-bottom: 1rem;
  }
  li { margin-bottom: 0.3rem; color: var(--text); }
  li code { font-size: 0.8em; }

  hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 3rem 0;
  }

  strong { color: var(--text-bright); font-weight: 700; }
</style>
</head>
<body>
<div class="wrapper">

<!-- ── Sidebar Navigation ── -->
<nav>
  <div class="nav-logo">
    <span class="pixel-icon">GB.RS</span>
    <small>Emulator Deep Dive</small>
  </div>

  <div class="toc-group">Overview</div>
  <ul>
    <li><a href="#s1"><span class="num">01</span> The Big Picture</a></li>
    <li><a href="#s2"><span class="num">02</span> Memory Map</a></li>
    <li><a href="#s3"><span class="num">03</span> The MMU</a></li>
    <li><a href="#s4"><span class="num">04</span> Hardware Registers</a></li>
    <li><a href="#s5"><span class="num">05</span> MBC3 Banking</a></li>
  </ul>

  <div class="toc-group">Core Systems</div>
  <ul>
    <li><a href="#s6"><span class="num">06</span> CPU Registers</a></li>
    <li><a href="#s7"><span class="num">07</span> Instruction Decoding</a></li>
    <li><a href="#s8"><span class="num">08</span> The PPU</a></li>
    <li><a href="#s9"><span class="num">09</span> Tile Data</a></li>
    <li><a href="#s10"><span class="num">10</span> Sprites & OAM</a></li>
  </ul>

  <div class="toc-group">Timing & I/O</div>
  <ul>
    <li><a href="#s11"><span class="num">11</span> Main Loop</a></li>
    <li><a href="#s12"><span class="num">12</span> Joypad</a></li>
    <li><a href="#qref"><span class="num">QR</span> Quick Reference</a></li>
  </ul>
</nav>

<!-- ── Main Content ── -->
<main>

  <div class="hero">
    <h1>Game Boy<br>Emulator in Rust<span class="cursor"></span></h1>
    <div class="subtitle">// FOCUS: MEMORY MAPPING · BIT OPERATIONS · HARDWARE REGISTERS</div>
    <p>A complete technical breakdown of every system in the emulator — from how a 16-bit address maps to the right byte of RAM, to how two bitplane bytes combine into a single pixel's color.</p>
  </div>

  <!-- ── Section 1 ── -->
  <section id="s1">
    <h2><span class="section-num">01</span> The Big Picture</h2>
    <p>The Game Boy has four major hardware components that run in parallel and communicate through <strong>shared memory</strong>:</p>
<pre>┌─────────────────────────────────────────────────────┐
│              ADDRESS BUS (16-bit)                    │
│               $0000 — $FFFF                          │
└──────┬──────────┬──────────┬──────────┬─────────────┘
       │          │          │          │
     [CPU]      [PPU]     [MMU]    [Joypad]
   Executes   Renders    Routes    Reports
  opcodes     pixels     reads/    button
              to VRAM    writes    state</pre>
    <p>Everything is glued together through the <strong>MMU (Memory Management Unit)</strong>, which owns the address space. The CPU and PPU never talk directly — they communicate through memory-mapped registers like <code>$FF40</code> (LCDC) or <code>$FF0F</code> (IF).</p>
  </section>

  <!-- ── Section 2 ── -->
  <section id="s2">
    <h2><span class="section-num">02</span> Memory Map — The Address Space</h2>
    <p>The Game Boy has a <strong>16-bit address bus</strong>, meaning addresses go from <code>$0000</code> to <code>$FFFF</code> — 65,536 possible locations, each holding one byte.</p>

    <div class="memory-map">
<span class="addr">$0000 — $3FFF</span>   <span class="size">16 KiB</span>   <span class="name">ROM Bank 0</span>          <span class="desc">Always the first 16 KiB of the cartridge</span>
<span class="addr">$4000 — $7FFF</span>   <span class="size">16 KiB</span>   <span class="name">ROM Bank N</span>          <span class="desc">Bank-switched via MBC</span>
<span class="addr">$8000 — $9FFF</span>   <span class="size"> 8 KiB</span>   <span class="name">VRAM</span>                <span class="desc">Tile data + tile maps</span>
<span class="addr">$A000 — $BFFF</span>   <span class="size"> 8 KiB</span>   <span class="name">External RAM</span>        <span class="desc">Battery-backed save RAM (switchable)</span>
<span class="addr">$C000 — $DFFF</span>   <span class="size"> 8 KiB</span>   <span class="name">Work RAM (WRAM)</span>     <span class="desc">General purpose RAM</span>
<span class="addr">$E000 — $FDFF</span>   <span class="size"> 7 KiB</span>   <span class="name">Echo RAM</span>            <span class="desc">Mirror of $C000–$DDFF</span>
<span class="addr">$FE00 — $FE9F</span>   <span class="size">160  B</span>   <span class="name">OAM</span>                 <span class="desc">40 sprite descriptors × 4 bytes</span>
<span class="addr">$FEA0 — $FEFF</span>   <span class="size"> 96  B</span>   <span class="name">Unused</span>              <span class="desc">Returns $FF on read</span>
<span class="addr">$FF00 — $FF7F</span>   <span class="size">128  B</span>   <span class="name">I/O Registers</span>       <span class="desc">Hardware control (LCDC, JOYP, etc.)</span>
<span class="addr">$FF80 — $FFFE</span>   <span class="size">127  B</span>   <span class="name">HRAM</span>                <span class="desc">Fast RAM (usable during DMA)</span>
<span class="addr">$FFFF</span>            <span class="size">  1  B</span>   <span class="name">IE Register</span>         <span class="desc">Interrupt Enable</span>
    </div>

    <h3>Why the Top Nibble Matters</h3>
    <p>The <strong>top nibble</strong> of the address (bits 15–12) instantly tells you which region you're in:</p>
<pre>Address = 0x8000
Binary  = 1000 0000 0000 0000
           ^^^^
           top nibble = 8 → VRAM region

Address = 0xFF40  (LCDC register)
Binary  = 1111 1111 0100 0000
           ^^^^
           top nibble = F → I/O or HRAM region</pre>

    <p>Every read or write goes through one match function, which picks the right storage based purely on the address value. This is <strong>memory mapping</strong> — the hardware behavior depends on <em>where</em> you read, not just <em>what</em> you read.</p>

    <h3>Address Subtraction — Converting to Array Index</h3>
    <p>Every region is stored starting at index 0 in its own array. To convert an absolute address to an array index, subtract the region's base:</p>
<pre>VRAM address: $8340
Base address: $8000
Array index:  $8340 - $8000 = $340 = 832

// In Rust:
0x8000..=0x9FFF => self.vram[addr as usize - 0x8000],
//                                           ^^^^^^^
//                           strip the $8000 base → array index</pre>
  </section>

  <!-- ── Section 3 ── -->
  <section id="s3">
    <h2><span class="section-num">03</span> The MMU — Reading and Writing Memory</h2>

    <h3>The Struct Layout</h3>
<pre>pub struct Mmu {
    rom:          Vec&lt;u8&gt;,       // entire cartridge ROM (up to 1 MiB)
    rom_bank:     usize,         // which 16 KiB chunk is at $4000–$7FFF
    pub vram:     [u8; 0x2000],  // 8 KiB video RAM
    pub extram:   Vec&lt;u8&gt;,       // 32 KiB save RAM
    extram_bank:  usize,         // which 8 KiB chunk is at $A000–$BFFF
    wram:         [u8; 0x4000],  // 8 KiB work RAM
    pub oam:      [u8; 0xA0],    // 160 bytes = 40 sprites × 4 bytes
    pub io:       [u8; 0x80],    // 128 bytes of I/O registers
    hram:         [u8; 0x7F],    // 127 bytes of high RAM
    pub ie:       u8,            // $FFFF — Interrupt Enable
}</pre>

    <div class="callout">
      <div class="callout-title">◆ Why 0xA0 for OAM?</div>
      40 sprites × 4 bytes each = 160 bytes = 0xA0. OAM lives at $FE00–$FE9F: <code>$FE9F - $FE00 + 1 = 0xA0 = 160 ✓</code>
    </div>

    <h3>DMA Transfer — Hardware-Accelerated memcpy</h3>
    <p>Writing to <code>$FF46</code> copies 160 bytes into OAM instantly. The byte written becomes the <strong>high byte</strong> of the source address:</p>
<pre>fn io_write(&amp;mut self, addr: u16, val: u8) {
    match addr {
        0xFF46 =&gt; {
            let src = (val as u16) &lt;&lt; 8;   // val becomes high byte of address
            // e.g., write $C1 → copy from $C100–$C19F into OAM
            for j in 0..0xA0u16 {
                self.oam[j as usize] = self.read(src + j);
            }
        }
        0xFF04 =&gt; self.io[i] = 0,  // DIV: any write resets to 0
        _      =&gt; self.io[i] = val,
    }
}</pre>
  </section>

  <!-- ── Section 4 ── -->
  <section id="s4">
    <h2><span class="section-num">04</span> Hardware Registers — Bit by Bit</h2>

    <h3>$FF40 — LCDC (LCD Control)</h3>
    <p>The master control register for all graphics. Every single bit controls something:</p>

    <div class="bit-diagram">
      <div class="bit-header">Bit  7    6    5    4    3    2    1    0</div>
      <div style="display:flex; gap:4px; margin-bottom:1rem;">
        <div class="bit-box label">LCD<br>on</div>
        <div class="bit-box label">Win<br>map</div>
        <div class="bit-box label">Win<br>en</div>
        <div class="bit-box label">Tile<br>data</div>
        <div class="bit-box label">BG<br>map</div>
        <div class="bit-box label">OBJ<br>size</div>
        <div class="bit-box label">OBJ<br>en</div>
        <div class="bit-box label">BG<br>en</div>
      </div>
    </div>

<pre>let lcdc = mmu.io[0x40];  // io[] index = $FF40 - $FF00 = 0x40

// Is the LCD on? (bit 7)
if lcdc &amp; 0x80 == 0 { return; }
//        ^^^^  0x80 = 0b10000000

// Background tile map (bit 3)
let map_base = if lcdc &amp; 0x08 != 0 { 0x9C00 } else { 0x9800 };
//                       ^^^^  0x08 = 0b00001000

// Tile data region (bit 4)
let tile_addr = if lcdc &amp; 0x10 != 0 { ... };
//                        ^^^^  0x10 = 0b00010000

// Sprites enabled? (bit 1)
if lcdc &amp; 0x02 != 0 { self.render_sprites(mmu); }
//        ^^^^  0x02 = 0b00000010</pre>

    <h3>$FF0F — IF and $FFFF — IE (Interrupts)</h3>
<pre>Bit 7  6  5  4  3  2  1  0
    │  │  │  │  │  │  │  └─ VBlank  interrupt
    │  │  │  │  │  │  └──── LCD STAT interrupt
    │  │  │  │  │  └─────── Timer interrupt
    │  │  │  │  └────────── Serial interrupt
    │  │  │  └───────────── Joypad interrupt
    └──┴──┴──────────────── Unused (always 1)</pre>

    <p>An interrupt fires when the same bit is set in <strong>both</strong> IF and IE, and the IME flag is on:</p>
<pre>let triggered = mmu.io[0x0F] &amp; mmu.ie &amp; 0x1F;
//              ─────────────  ───────  ─────
//              IF register    IE reg   mask unused bits

// PPU sets VBlank by ORing bit 0 into IF:
mmu.io[0x0F] |= 0x01;  // SET bit 0 without touching others

// CPU clears it when handled:
mmu.io[0x0F] &amp;= !(1 &lt;&lt; bit);  // CLEAR just that bit
// bit=0 → !(0b00000001) = 0b11111110</pre>

    <h3>$FF47 — BGP (Background Palette)</h3>
    <p>Maps 2-bit color indices (0–3) to actual shades. Two bits per color, four colors = 8 bits = 1 byte:</p>
<pre>Bit 7  6  5  4  3  2  1  0
    └──┘  └──┘  └──┘  └──┘
    Color3  Color2  Color1  Color0

// Extract shade for a pixel with color_id = 2:
let shade = (bgp &gt;&gt; (color_id * 2)) &amp; 0x03;

// Example: bgp = 0b11_10_01_00
// color_id = 2  →  bgp &gt;&gt; 4 = 0b00001110
//                            &amp; 0x03 = 0b10 = 2 = dark gray ✓</pre>
  </section>

  <!-- ── Section 5 ── -->
  <section id="s5">
    <h2><span class="section-num">05</span> MBC3 — Bank Switching</h2>
    <p>Pokemon Red is 1 MiB but the address space only has 64 KiB. The MBC3 chip swaps in different 16 KiB chunks of ROM at runtime.</p>

<pre>// Game writes bank number to $2000–$3FFF → MBC intercepts it:
0x2000..=0x3FFF =&gt; {
    self.rom_bank = if val == 0 { 1 } else { (val &amp; 0x7F) as usize };
    //                 ^^^^^^^^         ^^^^^^^^^^^^^^^^^^^
    //                 bank 0 illegal   mask to 7 bits (0–127)
    //                 → use bank 1
}

// Reading from $4000–$7FFF uses the active bank:
0x4000..=0x7FFF =&gt; {
    let offset = self.rom_bank * 0x4000 + (addr as usize - 0x4000);
    *self.rom.get(offset).unwrap_or(&amp;0xFF)
}

// Example:
// rom_bank = 5, addr = $4200
// offset = 5 × 16384 + 512 = 82432  ← actual byte in rom Vec</pre>
  </section>

  <!-- ── Section 6 ── -->
  <section id="s6">
    <h2><span class="section-num">06</span> The CPU — Registers and Flags</h2>

    <h3>Register Layout</h3>
    <div class="register-visual">
      <div class="reg-field"><span class="reg-name">A</span><span class="reg-bits">7:0</span></div>
      <div class="reg-field"><span class="reg-name">F</span><span class="reg-bits">Z N H C</span></div>
      <div style="width:8px"></div>
      <div class="reg-field"><span class="reg-name">B</span><span class="reg-bits">15:8</span></div>
      <div class="reg-field"><span class="reg-name">C</span><span class="reg-bits">7:0</span></div>
      <div style="width:8px"></div>
      <div class="reg-field"><span class="reg-name">D</span><span class="reg-bits">15:8</span></div>
      <div class="reg-field"><span class="reg-name">E</span><span class="reg-bits">7:0</span></div>
      <div style="width:8px"></div>
      <div class="reg-field"><span class="reg-name">H</span><span class="reg-bits">15:8</span></div>
      <div class="reg-field"><span class="reg-name">L</span><span class="reg-bits">7:0</span></div>
    </div>

<pre>// Pair construction:
pub fn bc(&amp;self) -&gt; u16 { (self.b as u16) &lt;&lt; 8 | self.c as u16 }
//                          ^^^^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^^^
//                          B in high byte    C in low byte

// Example: B=0x12, C=0x34 → BC = 0x1200 | 0x34 = 0x1234

// Splitting back:
pub fn set_bc(&amp;mut self, v: u16) {
    self.b = (v &gt;&gt; 8) as u8;  // high byte
    self.c = v as u8;           // low byte (truncates)
}</pre>

    <h3>The Flags Register (F)</h3>
<pre>Bit 7  6  5  4  3  2  1  0
    Z  N  H  C  0  0  0  0
    │  │  │  │  └──┴──┴──┴── always 0
    │  │  │  └───────────── Carry flag
    │  │  └──────────────── Half-carry flag
    │  └─────────────────── Subtract flag
    └────────────────────── Zero flag

// Setting flags packs 4 booleans into 1 byte:
pub fn set_flags(&amp;mut self, z: bool, n: bool, h: bool, c: bool) {
    self.f = (z as u8) &lt;&lt; 7
           | (n as u8) &lt;&lt; 6
           | (h as u8) &lt;&lt; 5
           | (c as u8) &lt;&lt; 4;
}

// Reading: isolate with mask
pub fn flag_z(&amp;self) -&gt; bool { self.f &amp; 0x80 != 0 }
//                                      ^^^^  isolate bit 7</pre>

    <h3>Half-Carry — What Is It?</h3>
    <p>Half-carry (H) is set when there's a carry from bit 3 into bit 4. Used for BCD arithmetic:</p>
<pre>// For ADD — did the low nibbles overflow 4 bits?
let h = (self.a &amp; 0xF) + (operand &amp; 0xF) &gt; 0xF;

// Example: A=0x38 (nibble=8), operand=0x0A (nibble=A)
// 8 + A = 18 = 0x12 → overflows 4 bits → H = true

// For SUB — borrow instead:
let h = (self.a &amp; 0xF) &lt; (operand &amp; 0xF);</pre>
  </section>

  <!-- ── Section 7 ── -->
  <section id="s7">
    <h2><span class="section-num">07</span> Instruction Decoding — Bit Patterns</h2>

    <h3>The r8 Register Encoding (3 bits)</h3>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Value</th><th>Register</th><th>Note</th></tr></thead>
        <tbody>
          <tr><td>0b000</td><td>B</td><td></td></tr>
          <tr><td>0b001</td><td>C</td><td></td></tr>
          <tr><td>0b010</td><td>D</td><td></td></tr>
          <tr><td>0b011</td><td>E</td><td></td></tr>
          <tr><td>0b100</td><td>H</td><td></td></tr>
          <tr><td>0b101</td><td>L</td><td></td></tr>
          <tr><td>0b110</td><td>(HL)</td><td>Memory access — costs extra cycles</td></tr>
          <tr><td>0b111</td><td>A</td><td></td></tr>
        </tbody>
      </table>
    </div>

    <h3>Decoding ALU Instructions ($80–$BF)</h3>
<pre>Opcode layout:
Bit 7  6  5  4  3  2  1  0
    1  0  └──┴──┘  └──┴──┘
           operation   source (r8)
           000=ADD  001=ADC  010=SUB  011=SBC
           100=AND  101=XOR  110=OR   111=CP

// Decode 0xA8:
// 0xA8 = 1010 1000
//        10      = confirms $80–$BF block
//          101   = XOR (bits 5–3)
//             000 = B register
// → XOR A, B ✓

// In Rust:
0x80..=0xBF =&gt; {
    let src = op &amp; 0x07;          // bits 2–0 = source register
    let operand = self.read_r8(src, mmu);
    let cycles = if src == 6 { 8 } else { 4 };
    self.alu(op, operand);
    cycles
}

// Inside alu():
let kind = (op &gt;&gt; 3) &amp; 0x07;  // shift right 3, mask 3 bits
//           align bits 5–3 into bits 2–0</pre>

    <h3>CB-Prefix Instructions</h3>
<pre>CB opcode layout:
Bit 7  6  5  4  3  2  1  0
    └──┘  └──┴──┘  └──┴──┘
   group   bit/op   register

group 00 = shift/rotate
group 01 = BIT (test a bit)
group 10 = RES (clear a bit)
group 11 = SET (set a bit)

// SET bit 3 of register C:
// opcode = 0xD9 = 1101 1001
//          11   = SET group
//            011 = bit 3
//               001 = C register

// RES clears with an inverted mask:
v &amp; !(1 &lt;&lt; bit)  // 1&lt;&lt;3 = 0b00001000 → !(...) = 0b11110111

// SET with OR:
v | (1 &lt;&lt; bit)   // sets only that bit, leaves others alone</pre>
  </section>

  <!-- ── Section 8 ── -->
  <section id="s8">
    <h2><span class="section-num">08</span> The PPU — How Pixels Are Built</h2>
    <p>The PPU renders 160×144 pixels, one scanline at a time. Each scanline takes exactly <strong>456 T-cycles</strong>.</p>

<pre>pub fn tick(&amp;mut self, cycles: u32, mmu: &amp;mut Mmu) {
    self.dot += cycles;

    while self.dot &gt;= 456 {
        self.dot -= 456;

        if self.ly &lt; 144 {
            self.render_scanline(mmu);   // draw visible line
        }

        self.ly += 1;

        if self.ly == 144 {
            mmu.io[0x0F] |= 0x01;  // VBlank → set IF bit 0
        }
        if self.ly &gt; 153 {
            self.ly = 0;  // 154 total lines, then reset
        }
        mmu.io[0x44] = self.ly;  // $FF44 = LY, game reads this
    }
}</pre>

    <div class="callout amber">
      <div class="callout-title">◆ Frame Math</div>
      144 visible lines × 456 + 10 VBlank lines × 456 = <strong>70,224 T-cycles per frame</strong>.<br>
      4,194,304 Hz ÷ 70,224 = <strong>~59.7 fps</strong> — the Game Boy's refresh rate.
    </div>
  </section>

  <!-- ── Section 9 ── -->
  <section id="s9">
    <h2><span class="section-num">09</span> Tile Data — The Bit Plane System</h2>
    <p>Each tile is 8×8 pixels. Each pixel needs 2 bits (4 colors), but those 2 bits are stored in <strong>separate byte planes</strong>, interleaved row by row.</p>

<pre>Each tile = 16 bytes (2 bytes per row × 8 rows):
Byte  0: Low bitplane  row 0  ← bit 0 of each pixel's color
Byte  1: High bitplane row 0  ← bit 1 of each pixel's color
Byte  2: Low bitplane  row 1
Byte  3: High bitplane row 1
...and so on</pre>

    <h3>Reconstructing a Pixel's Color</h3>
<pre>let lo = mmu.read(row_addr);      // low bitplane byte
let hi = mmu.read(row_addr + 1);  // high bitplane byte

// Pixel 0 is bit 7 (leftmost), pixel 7 is bit 0
let bit = 7 - (px % 8) as u8;

let color_id = ((hi &gt;&gt; bit) &amp; 1) &lt;&lt; 1   // high bit → bit 1 of color
             | ((lo &gt;&gt; bit) &amp; 1);         // low bit  → bit 0 of color

// Example — pixel 5 of a row:
// bit = 7 - 5 = 2
// lo = 0b0011_0110 → lo&gt;&gt;2 &amp; 1 = 1
// hi = 0b0101_1010 → hi&gt;&gt;2 &amp; 1 = 0
// color_id = (0 &lt;&lt; 1) | 1 = 1 = light gray ✓</pre>

    <h3>Tile Addressing — Signed vs Unsigned Mode</h3>
<pre>// LCDC bit 4 = 1: unsigned mode — tile 0 at $8000
0x8000u16 + tile_idx as u16 * 16

// LCDC bit 4 = 0: signed mode — tile 0 at $9000
// CRITICAL cast chain: u8 → i8 → i32 → u16
(0x9000i32 + tile_idx as i8 as i32 * 16) as u16
//           ^^^^^^^^^^
//           reinterpret byte as signed BEFORE multiplying

// tile_idx = 0xFF = 255 (u8)
// as i8   = -1
// as i32  = -1
// 0x9000 + (-1 × 16) = 0x8FF0  ← tile -1 is at $8FF0 ✓</pre>
  </section>

  <!-- ── Section 10 ── -->
  <section id="s10">
    <h2><span class="section-num">10</span> Sprites — OAM and Attribute Bits</h2>
    <p>OAM at <code>$FE00–$FE9F</code> holds 40 sprite descriptors, 4 bytes each:</p>

    <div class="table-wrap">
      <table>
        <thead><tr><th>Byte</th><th>Content</th><th>Note</th></tr></thead>
        <tbody>
          <tr><td>0</td><td>Y position</td><td>Sprite top = byte0 − 16</td></tr>
          <tr><td>1</td><td>X position</td><td>Sprite left = byte1 − 8</td></tr>
          <tr><td>2</td><td>Tile index</td><td>Always unsigned from $8000</td></tr>
          <tr><td>3</td><td>Attributes</td><td>Flags — see below</td></tr>
        </tbody>
      </table>
    </div>

    <h3>Sprite Attribute Byte (Byte 3)</h3>
<pre>Bit 7  6  5  4  3  2  1  0
    │  │  │  │  │
    │  │  │  │  └─────────── Unused
    │  │  │  └────────────── Palette: 0=OBP0, 1=OBP1
    │  │  └───────────────── X-flip
    │  └──────────────────── Y-flip
    └─────────────────────── Priority: 0=above BG, 1=behind BG

let palette  = if attrs &amp; 0x10 != 0 { obp1 } else { obp0 }; // bit 4
let x_flip   = attrs &amp; 0x20 != 0;  // bit 5
let y_flip   = attrs &amp; 0x40 != 0;  // bit 6
let priority = attrs &amp; 0x80 != 0;  // bit 7</pre>

    <h3>Flipping Pixels</h3>
<pre>// X-flip: reverse which bit we read
let bit = if x_flip { px } else { 7 - px } as u8;

// Y-flip: reverse which row of the tile
let mut row = (ly - sprite_y) as u16;
if y_flip { row = 7 - row; }

// Priority: sprite only shows through transparent BG (color 0)
if priority {
    let existing = self.framebuffer[(y * 160 + x) * 4];
    if existing != 0xFF { continue; }  // BG not white → sprite hidden
}

if color_id == 0 { continue; }  // sprite color 0 always transparent</pre>
  </section>

  <!-- ── Section 11 ── -->
  <section id="s11">
    <h2><span class="section-num">11</span> The Main Loop — Timing Everything</h2>
    <p>The CPU and PPU must stay in sync, measured in <strong>T-cycles</strong>. Run one instruction, advance the PPU by the same number of cycles:</p>

<pre>let mut frame_cycles = 0u32;
while frame_cycles &lt; 70224 {            // 70,224 T-cycles per frame
    let cycles = cpu.step(&amp;mut mmu);    // run one instruction
    ppu.tick(cycles, &amp;mut mmu);         // PPU catches up
    frame_cycles += cycles;
}</pre>

    <h3>Instruction Cycle Counts</h3>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Instruction</th><th>T-Cycles</th><th>M-Cycles</th><th>Why</th></tr></thead>
        <tbody>
          <tr><td>NOP</td><td>4</td><td>1</td><td>Just fetch the opcode</td></tr>
          <tr><td>LD BC, u16</td><td>12</td><td>3</td><td>Fetch opcode + 2 data bytes</td></tr>
          <tr><td>CALL u16</td><td>24</td><td>6</td><td>Fetch + 2 data + 2 stack writes + 1 internal</td></tr>
          <tr><td>LD r, (HL)</td><td>8</td><td>2</td><td>Opcode fetch + memory read</td></tr>
        </tbody>
      </table>
    </div>
  </section>

  <!-- ── Section 12 ── -->
  <section id="s12">
    <h2><span class="section-num">12</span> Joypad — Active-Low Input</h2>
    <p>The joypad register at <code>$FF00</code> is <strong>active-low</strong> (0 = pressed, 1 = not pressed) and multiplexed — the game selects which group to read.</p>

<pre>Bit 7  6  5  4  3  2  1  0
    1  1  │  │  │  │  │  │
           │  │  │  │  │  └─ Right  / A      (0=pressed)
           │  │  │  │  └──── Left   / B
           │  │  │  └─────── Up     / Select
           │  │  └────────── Down   / Start
           │  └───────────── Select D-pad (write 0 to select)
           └──────────────── Select buttons (write 0 to select)</pre>

<pre>fn update_joypad(window: &amp;Window, mmu: &amp;mut Mmu) {
    let joyp = mmu.io[0x00];

    // Build nibble, then INVERT: pressed(1) → 0, not-pressed(0) → 1
    let d: u8 = !(
        (window.is_key_down(Key::Down)  as u8) &lt;&lt; 3 |
        (window.is_key_down(Key::Up)    as u8) &lt;&lt; 2 |
        (window.is_key_down(Key::Left)  as u8) &lt;&lt; 1 |
        (window.is_key_down(Key::Right) as u8)
    ) &amp; 0x0F;

    // Return correct nibble based on what the game selected
    let low = if joyp &amp; 0x10 == 0 { d_pad }      // bit 4 low → D-pad
              else if joyp &amp; 0x20 == 0 { buttons } // bit 5 low → buttons
              else { 0x0F };                        // neither → nothing

    mmu.io[0x00] = (joyp &amp; 0x30) | low | 0xC0;
}</pre>

    <div class="callout">
      <div class="callout-title">◆ Active-Low Example</div>
      Right arrow held → <code>is_key_down = 1</code> → before invert: <code>0b0000_0001</code> → after <code>!</code>: <code>0b1111_1110</code> → after <code>&amp; 0x0F</code>: <code>0b0000_1110</code>. Game reads bit 0 = 0 → Right is pressed ✓
    </div>
  </section>

  <!-- ── Quick Reference ── -->
  <section id="qref">
    <h2><span class="section-num">QR</span> Quick Reference — Key Bit Masks</h2>
    <div class="table-wrap">
      <table class="qref">
        <thead><tr><th>Register</th><th>Address</th><th>Mask</th><th>Meaning</th></tr></thead>
        <tbody>
          <tr><td>LCDC</td><td>$FF40</td><td>0x80</td><td>LCD on/off</td></tr>
          <tr><td>LCDC</td><td>$FF40</td><td>0x40</td><td>Window tile map ($9800/$9C00)</td></tr>
          <tr><td>LCDC</td><td>$FF40</td><td>0x20</td><td>Window enable</td></tr>
          <tr><td>LCDC</td><td>$FF40</td><td>0x10</td><td>BG tile data ($8800/$8000)</td></tr>
          <tr><td>LCDC</td><td>$FF40</td><td>0x08</td><td>BG tile map ($9800/$9C00)</td></tr>
          <tr><td>LCDC</td><td>$FF40</td><td>0x02</td><td>Sprite enable</td></tr>
          <tr><td>IF / IE</td><td>$FF0F</td><td>0x01</td><td>VBlank interrupt</td></tr>
          <tr><td>JOYP</td><td>$FF00</td><td>0x10</td><td>Select D-pad group</td></tr>
          <tr><td>JOYP</td><td>$FF00</td><td>0x20</td><td>Select button group</td></tr>
          <tr><td>Sprite attr</td><td>OAM+3</td><td>0x10</td><td>Palette (OBP0/OBP1)</td></tr>
          <tr><td>Sprite attr</td><td>OAM+3</td><td>0x20</td><td>X-flip</td></tr>
          <tr><td>Sprite attr</td><td>OAM+3</td><td>0x40</td><td>Y-flip</td></tr>
          <tr><td>Sprite attr</td><td>OAM+3</td><td>0x80</td><td>Priority (above/behind BG)</td></tr>
          <tr><td>F register</td><td>—</td><td>0x80</td><td>Zero flag (Z)</td></tr>
          <tr><td>F register</td><td>—</td><td>0x40</td><td>Subtract flag (N)</td></tr>
          <tr><td>F register</td><td>—</td><td>0x20</td><td>Half-carry flag (H)</td></tr>
          <tr><td>F register</td><td>—</td><td>0x10</td><td>Carry flag (C)</td></tr>
        </tbody>
      </table>
    </div>
  </section>

  <!-- ── Golden Rule ── -->
  <div class="golden-rule">
    <p>Every hardware behavior is just bit manipulation on memory-mapped addresses.</p>
  </div>

  <p style="color:var(--text-dim); font-family:'Share Tech Mono',monospace; font-size:0.75rem; text-align:center; letter-spacing:3px;">
    GAME BOY EMULATOR IN RUST · DEEP DIVE DOCUMENTATION · 2026
  </p>

</main>
</div>
</body>
</html>



