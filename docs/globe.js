// Shared by globe.html (the full explorer) and index.html (the story-first page).
// The page's body markup is injected here so the two pages differ only in <body class>.
// Every figure in it carries a data-claim attribute naming an entry of docs/evidence/claims.json (built from
// crates/cewp-evidence/src/page_ledger.rs; `cargo test -p cewp-evidence page_` checks the regions
// between evidence:scan and evidence:end for figures without one).
// evidence:scan
document.body.insertAdjacentHTML("afterbegin", String.raw`
<canvas id="c"></canvas>
<canvas id="micro" aria-hidden="true"></canvas>
<button id="btn-hud" class="fab" type="button" title="Toggle controls &amp; population opt-ins">☰ Controls · <span class="badge" id="badge-scale" data-claim="world-population">5.00 B</span></button>
<button id="btn-story-fab" class="fab fab-story" type="button" title="Watch two people connect and grow into a global network">▶ Play Story</button>
<button id="btn-stats" class="fab" type="button" title="Toggle live telemetry &amp; sustainability metrics">☰ Metrics · <span class="badge" id="badge-local" data-claim="kernel-locality">– local</span></button>
<button id="btn-foot" class="fab" type="button" title="Toggle architecture legend &amp; links">? Info</button>
<nav class="nav-strip" aria-label="Architecture &amp; navigation links">
  <button id="btn-story-nav" class="nav-pill nav-story-btn" type="button" title="Watch two people connect and grow into a global network">▶ Story</button>
  <a class="nav-pill active" href="globe.html">🌍 Globe</a>
  <a class="nav-pill" href="index.html">📖 Story page</a>
  <a class="nav-pill" href="csd-explorer.html">🧭 CSD/4 &amp; Namespace Browser</a>
  <a class="nav-pill" href="how-it-works.html#peers">⚖️ vs. Other Meshes</a>
  <a class="nav-pill" href="how-it-works.html#solves">⚙️ How This Works</a>
  <a class="nav-pill" href="how-it-works.html#motion">▶️ In Motion</a>
  <a class="nav-pill" href="mesh.html">🕸️ 6-Peer Kernel</a>
  <a class="nav-pill" href="toy/">📐 Scaling Toy</a>
  <button id="btn-evidence" class="nav-pill" type="button" title="What each figure rests on, and which tier is running">🔎 Evidence</button>
  <a class="nav-pill" href="https://github.com/CIRISAI/CEWP/blob/main/FSD/SIMULATION_ENGINE.md" target="_blank" rel="noopener">📄 Sim FSD v0.2</a>
  <a class="nav-pill" href="https://github.com/CIRISAI/CEWP" target="_blank" rel="noopener">GitHub</a>
</nav>
<div id="live-strip" role="note">
  <span><b>Measured from a live mesh:</b> <span class="ls-long">the serve seat's costs are <span class="fig" data-claim="seat1-windows">calibrated on the production CIRIS mesh (2026-11-10)</span>; beyond today's fleet every figure is a model, labelled where shown.</span><span class="ls-short">seat costs <span class="fig" data-claim="seat1-windows">calibrated on the production CIRIS mesh (2026-11-10)</span>; the rest is model.</span> <button id="live-what" type="button">what is measured?</button></span>
  <span><span class="ls-long">Begin using it: </span><code>pip install ciris-agent</code> then <code>ciris-agent</code> · <a href="https://ciris.ai/install" target="_blank" rel="noopener">install guide</a> · <a href="https://pypi.org/project/ciris-agent/" target="_blank" rel="noopener">PyPI</a></span>
  <button class="ls-x" id="live-x" type="button" title="Hide">✕</button>
</div>

<div id="story-hud" class="story-panel">
  <div class="story-head"><span class="story-act-badge" id="story-act-badge">1/8</span><span class="story-title" id="story-title"></span></div>
  <div class="story-line" id="story-line" tabindex="0">Loading the twin…</div>
  <div class="story-mini-ctls">
    <button id="story-btn-prev" class="story-hdr-btn" type="button" title="Previous" aria-label="Previous step">‹</button>
    <button id="story-btn-play" class="story-hdr-btn" type="button" title="Pause / Resume" aria-label="Pause or resume">❚❚</button>
    <button id="story-btn-next" class="story-hdr-btn" type="button" title="Next" aria-label="Next step">›</button>
    <button id="story-btn-exit" class="story-hdr-btn" type="button" title="Close story" aria-label="Close the story">✕</button>
  </div>
  <div class="story-progress-track">
    <div class="story-progress-bar" id="story-progress-bar"></div>
  </div>
</div>

<div id="focal-chip" class="panel-hidden"></div>
<div id="node-card" class="panel-hidden"></div>
<div id="hud" class="panel-hidden">
  <div class="panel-head">
    <h1>Billions of people and devices, one rulebook</h1>
    <button class="panel-close" id="close-hud" type="button" title="Hide controls">✕</button>
  </div>
  <p>Land-bounded <span class="fig" data-claim="pop-grid-source">world population grid</span> (<span class="fig" data-claim="pop-grid-cells">2,961 cells, 181 localities</span>). Every person's devices evaluate the same constitutional rulebook across <b>CC 5.4.3 1-hop shaped budget classes</b> (<span class="fig" data-claim="budget-class-rates"><code>server</code> 7 KB/s, <code>home</code> 2.8 KB/s, <code>phone-wifi</code> 1.4 KB/s, <code>phone-cellular</code> 350 B/s, <code>seat</code> 33 KB/s</span>), ordinary links, and <b>CC 6.1.6 <code>deterministic-alm</code></b> member relay trees over sealed chunks. <span class="fig" data-claim="all-simulated">Every person and device here is simulated.</span></p>
  <input type="hidden" id="size" value="5000000000">
  <input type="hidden" id="acts" value="10000">
  <div class="row main-actions">
    <button id="play" class="btn-primary active">❚❚ pause</button>
    <button id="step" class="btn-subtle">step</button>
    <button id="eng" class="on">engine: WASM</button>
    <button id="arcsb" class="on">arcs</button>
  </div>
  <button id="btn-story-hud" class="btn-primary btn-story-main" type="button" style="width:100%;margin-top:6px;display:flex;align-items:center;justify-content:center;gap:6px"><span>▶</span> Play Story</button>
  <div class="sust-strip">
    <div class="sust-pill"><span>Reachable home</span><b id="pill-nat" data-claim="nat-leaf-capacity">–</b></div>
    <div class="sust-pill"><span>1-hop shaped slots</span><b id="pill-shape" data-claim="shaped-slot-supply">–</b></div>
    <div class="sust-pill"><span>Onion cover</span><b id="pill-onion" data-claim="onion-cover">–</b></div>
    <div class="sust-pill"><span>Holon indicator</span><b id="pill-holon" data-claim="holon-indicator">–</b></div>
  </div>
  <div class="control-group" style="margin:4px 0 2px">
    <div class="cg-head"><label>Resolution tier running</label></div>
    <p class="tier-note"><span class="fig" data-claim="tier-cohorts">Above street zoom: the cohort tier, each dot a cohort standing for many people</span>. <span class="fig" data-claim="tier-micro">At street zoom: the R5 tier, one generated neighbourhood with every device a node of its own</span>. The R1 to R4 tiers of the FSD are not run on this page, and no tier here is certified.</p>
  </div>
  <p id="status" data-evidence="figs" style="margin:4px 0 2px">building…</p>

  <details id="adv">
    <summary></summary>
    <div class="adv-body">
      <div class="adv-sec">
        <h3><span>Presets</span><span style="color:var(--mute);font-weight:400;font-size:9.5px">FSD §4.4 / CC 4.5.13 / 5.4.3 / 6.1.6</span></h3>
        <div class="preset-bar" id="preset-bar">
          <button data-preset="optimistic">Optimistic</button>
          <button data-preset="expected">Expected (Fleet)</button>
          <button data-preset="pessimistic">Pessimistic (CGNAT)</button>
          <button data-preset="adversarial">Adversarial</button>
          <button data-preset="privacy">High Privacy</button>
          <button data-preset="unowned">Unowned-Only (no relays)</button>
        </div>
        <div class="control-group" style="margin:4px 0">
          <div class="cg-head"><label>World population (people · <span class="fig" data-claim="devices-per-person">2.25 devices/person</span>)</label></div>
          <div class="seg" id="size-seg">
            <button data-val="100000000"><span class="inp">100 M</span></button>
            <button data-val="1000000000"><span class="inp">1 B</span></button>
            <button data-val="5000000000" class="active"><span class="inp">5 B</span></button>
            <button data-val="8050000000"><span class="inp">8.1 B</span></button>
          </div>
        </div>
        <div class="control-group" style="margin:4px 0">
          <div class="cg-head"><label>Acts / tick</label><span class="ms-budget" id="ms-budget" data-claim="browser-kernel-time">measuring…</span></div>
          <div class="seg" id="acts-seg">
            <button data-val="2000"><span class="inp">2 k</span></button>
            <button data-val="10000" class="active"><span class="inp">10 k</span></button>
            <button data-val="50000"><span class="inp">50 k</span></button>
            <button data-val="200000"><span class="inp">200 k</span></button>
          </div>
        </div>
        <div class="control-group" style="margin:4px 0 0">
          <div class="cg-head"><label>Mesh layer filter</label></div>
          <div class="mesh-pills" id="mesh-seg">
            <button data-tier="-1" class="active">All</button>
            <button data-tier="0"><span class="dot" style="background:var(--warm)"></span>1-hop device shaped</button>
            <button data-tier="1"><span class="dot" style="background:var(--cool)"></span>Seat class</button>
            <button data-tier="2"><span class="dot" style="background:var(--ok)"></span>Ordinary / <code>deterministic-alm</code></button>
            <button data-tier="3"><span class="dot" style="background:var(--violet)"></span>Onion</button>
          </div>
        </div>
      </div>

      <div class="adv-sec">
        <h3><span>Device Tiers, Phone Wi-Fi &amp; Reachable-Home Share</span><span id="tier-sum-lbl" data-evidence="input" style="color:var(--ok);font-size:9.5px">100%</span></h3>
        <div class="sl-item">
          <div class="sl-top"><span>Server tier (<span class="fig" data-claim="server-tier-spec">1 TiB / 1 Gbps always-on</span>)</span><b id="v_srv" data-evidence="input">5%</b></div>
          <input type="range" class="adv-range" id="sl_srv" min="1" max="25" step="1" value="5">
        </div>
        <div class="sl-item">
          <div class="sl-top"><span>Home tier (broadband / NAS / desktop)</span><b id="v_home" data-evidence="input">55%</b></div>
          <input type="range" class="adv-range" id="sl_home" min="10" max="85" step="1" value="55">
        </div>
        <div class="sl-item">
          <div class="sl-top"><span>Phone tier (mobile — auto-balanced)</span><b id="v_phone" data-evidence="input">40%</b></div>
        </div>
        <div class="sl-item">
          <div class="sl-top"><span>Phone on Wi-Fi vs Cellular</span><b id="v_wifi" data-evidence="input">65% Wi-Fi · 35% cell</b></div>
          <input type="range" class="adv-range" id="sl_wifi" min="5" max="95" step="5" value="65">
        </div>
        <div class="sl-item">
          <div class="sl-top"><span>Phone on Wi-Fi + charging (relay-eligible)</span><b id="v_chg" data-evidence="input">35% of Wi-Fi</b></div>
          <input type="range" class="adv-range" id="sl_chg" min="5" max="90" step="5" value="35">
        </div>
        <div class="sl-item">
          <div class="sl-top"><span>No direct path on Home &amp; Phone (vs IPv6 / punched)</span><b id="v_nat" data-evidence="input">60% NAT · 40% direct</b></div>
          <input type="range" class="adv-range" id="sl_nat" min="0" max="95" step="5" value="60">
          <div class="pro-con">Reachable-home share = <span id="v_direct_nat" data-evidence="input" style="color:var(--ok)">40%</span> direct IPv6 / punched UDP path (<code>leviculum#100</code> NAT-mix, <code>leviculum#101</code> no PCP/NAT-PMP/UPnP): a scenario input, no field measurement exists. Per CC 5.4.3, <span class="fig" data-claim="shaped-link-one-hop">a shaped link is 1 direct hop; peers with no direct path carry on ordinary links</span>.</div>
        </div>
      </div>

      <div class="adv-sec">
        <h3><span>Population Opt-In Choices</span><span style="color:var(--mute);font-weight:400;font-size:9.5px">Benefits (+) &amp; Downsides (−)</span></h3>

        <div class="sl-item">
          <div class="sl-top"><span>Link shaping — <code>server</code> (<span class="fig" data-claim="budget-class-rates">7,000 B/s</span>)</span><span><span class="dflt" data-claim="shaping-defaults">Default: ON</span> <b id="v_sh_srv" data-evidence="input">95%</b></span></div>
          <input type="range" class="adv-range" id="sl_sh_srv" min="0" max="100" step="5" value="95">
          <div class="pro-con">
            <span class="pro">+ Hides row timing/size in channel MDU frames (<span class="fig" data-claim="budget-class-rates">ρ=0.5, W=600 s</span>); carries onion walks at <span class="fig" data-claim="onion-tier">+0 B extra wire cost</span>.</span>
            <span class="con">− Sends <span class="fig" data-claim="cover-volume">~18.1 GB/mo/link</span> continuous cover even idle; adds <span class="fig" data-claim="shaping-mean-wait">~1.16 s mean</span> (<span class="fig" data-claim="frame-interval">~2.3 s @ 16 KB TCP MDU</span>) Poisson wait.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Link shaping — <code>home</code> (<span class="fig" data-claim="budget-class-rates">2,800 B/s</span>)</span><span><span class="dflt off" data-claim="shaping-defaults">Default: OFF</span> <b id="v_sh_home" data-evidence="input">25%</b></span></div>
          <input type="range" class="adv-range" id="sl_sh_home" min="0" max="100" step="5" value="25">
          <div class="pro-con">
            <span class="pro">+ Masks home traffic timing/size (<span class="fig" data-claim="budget-class-rates">W=600 s</span>) on direct 1-hop links; expands standing shaped cover pool for onion peers.</span>
            <span class="con">− Costs <span class="fig" data-claim="cover-volume">~7.2 GB/mo/link</span> continuous cover bandwidth; adds <span class="fig" data-claim="shaping-mean-wait">~2.9 s mean</span> (<span class="fig" data-claim="frame-interval">~5.8 s @ 16 KB TCP MDU</span>) wait.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Link shaping — <code>phone-wifi</code> (<span class="fig" data-claim="budget-class-rates">1,400 B/s</span>) / <code>phone-cellular</code> (<span class="fig" data-claim="budget-class-rates">350 B/s</span>)</span><span><span class="dflt off" data-claim="shaping-defaults">Default: OFF</span> <b id="v_sh_phone" data-evidence="input">10%</b></span></div>
          <input type="range" class="adv-range" id="sl_sh_phone" min="0" max="100" step="5" value="10">
          <div class="pro-con">
            <span class="pro">+ Shapes direct 1-hop mobile links (<span class="fig" data-claim="budget-class-rates">W=600 s Wi-Fi, W=3,600 s cellular</span>); contributes mobile shaped cover hops.</span>
            <span class="con">− <span class="fig" data-claim="cover-volume">3.6 GB/mo/link Wi-Fi (0.9 GB/mo/link cell, ~7 GB/mo @ 8 links)</span>; radio awake; <span class="fig" data-claim="frame-interval">11.6–46.5 s TCP MDU delay</span>.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Offer relay (directly reachable server/home/phone-charging)</span><span><span class="dflt" data-claim="shaping-defaults">Default: ON</span> <b id="v_relay" data-evidence="input">90%</b></span></div>
          <input type="range" class="adv-range" id="sl_relay" min="0" max="100" step="5" value="90">
          <div class="pro-con">
            <span class="pro">+ Each directly reachable home/server adds <span class="fig" data-claim="shaped-link-bound">L−1 = 7 shaped slots</span> (<code>Tiers.lean</code>: <code>each_home_adds_seven</code>, <code>reach ≠ capacity</code>) &amp; avoids stranding when no shaped slot is left.</span>
            <span class="con">− Carries strangers' sealed rows up to class cap; discloses signed route &amp; locality to <span class="fig" data-claim="max-referrals">≤3 referred strangers</span>.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Announce owner-binding on roster</span><span><span class="dflt" data-claim="optin-defaults">Default: ON</span> <b id="v_ann" data-evidence="input">85%</b></span></div>
          <input type="range" class="adv-range" id="sl_ann" min="5" max="100" step="5" value="85">
          <div class="pro-con">
            <span class="pro">+ Reachable by short ciris:// name; can join public rooms (open:self_admit) &amp; serve as announced relay/seat.</span>
            <span class="con">− Lists device on public federation roster (lightnet); declining (self darknet) requires manual v3 contact codes.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Anonymous mode / Onion tier (CC 5.4.6)</span><span><span class="dflt off" data-claim="optin-defaults">Default: OFF</span> <b id="v_anon" data-evidence="input">12%</b></span></div>
          <input type="range" class="adv-range" id="sl_anon" min="0" max="60" step="2" value="12">
          <div class="pro-con">
            <span class="pro">+ Unlinkable federation publishing &amp; private rooms via <span class="fig" data-claim="onion-tier">3-hop X-Wing onion walks</span> over shaped links from sticky guard.</span>
            <span class="con">− Row plane only (<span class="fig" data-claim="payload-limit">≤64 KiB</span>, no live A/V); <span class="fig" data-claim="onion-cost-share">~1/3 home shaped share cover cost</span>; never relays; uptime-correlation on sleeping phones.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Onion relay participation (<span class="fig" data-claim="onion-tier">3-layer deal</span>)</span><span><span class="dflt" data-claim="optin-defaults">Default: ON</span> <b id="v_orel" data-evidence="input">88%</b></span></div>
          <input type="range" class="adv-range" id="sl_orel" min="0" max="100" step="4" value="88">
          <div class="pro-con">
            <span class="pro">+ Expands the <span class="fig" data-claim="onion-tier">3-layer relay deal</span> &amp; mixing pool so anonymous walks aren't narrowed by a small volunteer set.</span>
            <span class="con">− Layer links must run shaped, reserving <span class="fig" data-claim="onion-relay-reserve">up to 4</span> of the node's <span class="fig" data-claim="shaped-link-bound">L=8 shaped link slots</span> at class rate.</span>
          </div>
        </div>

        <div class="sl-item">
          <div class="sl-top"><span>Publishable H3ERE agent traces (CC 4.5.13)</span><span><span class="dflt off" data-claim="optin-defaults">Default: OFF</span> <b id="v_trc" data-evidence="input">15%</b></span></div>
          <input type="range" class="adv-range" id="sl_trc" min="0" max="100" step="5" value="15">
          <div class="pro-con">
            <span class="pro">+ Contributes <span class="fig" data-claim="trace-size">14 KiB/decision</span> scrubbed reasoning traces to cohort accountability and RATCHET evaluation.</span>
            <span class="con">− Widens trace visibility beyond self/family and adds signing, scrubbing, and chunk custody storage load.</span>
          </div>
        </div>
      </div>
      <button id="btn-story-hud-bottom" class="btn-primary btn-story-main" type="button" style="width:100%;margin-top:8px;display:flex;align-items:center;justify-content:center;gap:6px"><span>▶</span> Play Story</button>
    </div>
  </details>
</div>
<div id="stats" class="panel-hidden">
  <div class="panel-head" style="margin-bottom:4px">
    <span style="font:600 11px var(--mono);text-transform:uppercase;letter-spacing:.06em;color:var(--cool)">Live Telemetry &amp; Sustainability</span>
    <button class="panel-close" id="close-stats" type="button" title="Hide metrics">✕</button>
  </div>
  <div class="kv"><span>rows that never leave the signer's locality</span><b class="big" id="local" data-claim="kernel-locality">–</b></div>
  <div class="bar"><i id="localbar" style="width:0"></i></div>
  <div id="stats-more">
    <div class="kv"><span>deliveries within the locality</span><b id="dlocal" data-claim="kernel-locality">–</b></div>
    <div class="kv"><span>crosses a locality, same region</span><b id="xmetro" data-claim="kernel-locality">–</b></div>
    <div class="kv"><span>crosses a region</span><b id="xregion" data-claim="kernel-locality">–</b></div>
    <div class="kv"><span>mean distance of a crossing</span><b id="km" data-claim="kernel-locality">–</b></div>

    <h2>Holon indicator (FSD §4.2 / §4.5): a formula, not a certificate</h2>
    <div class="kv"><span>Commuting-square defect D<sub>v</sub>² against a budget of <span class="fig" data-claim="holon-indicator">β=0.030</span></span><b id="s_holon_dv2" data-claim="holon-indicator">–</b></div>
    <div class="bar"><i id="holonbar" style="width:0"></i></div>
    <div class="kv"><span>Lipschitz indicator K (a formula, not estimated from data; ceiling <span class="fig" data-claim="holon-indicator">≤ 1.05</span>)</span><b id="s_holon_k" data-claim="holon-indicator">–</b></div>
    <div class="kv"><span>Coarse state maintenance rent G<sub>∞</sub></span><b id="s_holon_rent" data-claim="holon-indicator">–</b></div>
    <div class="kv"><span>Within the budget? (no CIRISHolon certificate is computed)</span><b id="s_holon_cert" data-claim="holon-indicator">–</b></div>

    <h2>cewp-sim v0.2 Transport &amp; Chunk Custody Storage</h2>
    <div class="kv"><span><span class="fig" data-claim="payload-limit">&gt;64 KiB refused on shaped</span> (unshaped ALM / bulk)</span><b id="s_64k_refused" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span><span class="fig" data-claim="shaped-link-bound">L ≤ 8</span> displacements (<span class="fig" data-claim="no-displacement">none in edge v45, #939</span>)</span><b id="s_displaced" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span>Layered fidelity Full reconstruction</span><b id="s_chunk_custody" data-claim="chunk-custody">–</b></div>
    <div class="kv"><span>Baselines vs CEWP (A1 CDN / A2 SFU / A3 P2P)</span><b id="s_baselines" data-claim="kernel-baselines">–</b></div>

    <h2>slot supply &amp; 1-hop shaped links (CC 5.4.3 / <code>Tiers.lean</code>)</h2>
    <div class="kv"><span>no-direct-path leaves vs <span class="fig" data-claim="nat-leaf-capacity"><i>k</i>=2 parent capacity</span></span><b id="s_nat_cap" data-claim="nat-leaf-capacity">–</b></div>
    <div class="bar"><i id="natbar" style="width:0"></i></div>
    <div class="kv"><span>reachable-home share (IPv6 / punched direct path)</span><b id="s_nat_max" data-claim="nat-leaf-capacity">–</b></div>
    <div class="kv"><span>1-hop shaped slots (<code>each_home_adds_seven</code>)</span><b id="s_shape_slots" data-claim="shaped-slot-supply">–</b></div>
    <div class="kv"><span>direct 1-hop shaping devices (cover pool)</span><b id="s_shapers" data-claim="shaped-slot-supply">–</b></div>
    <div class="kv"><span>shaping cover : onion peer ratio</span><b id="s_cover_ratio" data-claim="onion-cover">–</b></div>
    <div class="kv"><span>answering onion relays / layer (<span class="fig" data-claim="onion-tier">floor ≥3</span>)</span><b id="s_onion_relays" data-claim="onion-cover">–</b></div>
    <h2>CC 5.4.3 budget classes &amp; CC 6.1.6 <code>deterministic-alm</code></h2>
    <div class="kv"><span><span class="dot" style="background:var(--warm)"></span> 1-hop device shaped (<code>server</code>/<code>home</code>/<code>phone-*</code>)</span><b id="m_base" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span><span class="dot" style="background:var(--cool)"></span> 1-hop <code>seat</code> shaped (<span class="fig" data-claim="budget-class-rates">seat↔seat 33,000 B/s</span>)</span><b id="m_seat" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span><span class="dot" style="background:var(--ok)"></span> ordinary &amp; <code>deterministic-alm</code> (unshaped)</span><b id="m_bulk" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span><span class="dot" style="background:var(--violet)"></span> onion walks (<span class="fig" data-claim="onion-tier">cover-replaced, 0 B extra</span>)</span><b id="m_onion" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span>CC 6.1.6 <code>deterministic-alm</code> member tree savings</span><b id="m_alm" data-claim="kernel-traffic">–</b></div>
    <h2>street tiling (holons, FSD §4.6.6)</h2>
    <div class="kv"><span>acuity ring · frame rate · leaves</span><b id="s_tiling" data-claim="browser-tiling-stats">–</b></div>
    <h2>this tick</h2>
    <div class="kv"><span>acts signed</span><b id="acts_n" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span>deliveries (rulebook checks run)</span><b id="deliv" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span>refused by signer's local rulebook</span><b id="refused" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span>wire bytes moved (excl. <span class="fig" data-claim="onion-tier">0B onion cover</span>)</span><b id="bytes" data-claim="kernel-traffic">–</b></div>
    <div class="kv"><span>a broadcast network would move</span><b id="bcast" data-claim="kernel-baselines">–</b></div>
    <div class="kv"><span>with a datacenter in the middle</span><b id="central" data-claim="kernel-baselines">–</b></div>
    <h2>the rulebook kernel</h2>
    <div class="kv"><span>rulebook checks / second (your browser)</span><b id="dps" data-claim="browser-kernel-time">–</b></div>
    <div class="kv"><span>ms per tick (rulebook + relay tree, your browser)</span><b id="ms" data-claim="browser-kernel-time">–</b></div>
    <div class="kv"><span>world scale (people · devices)</span><b id="bpn" data-claim="world-population">–</b></div>
    <div class="kv"><span>cumulative deliveries</span><b id="cum" data-claim="kernel-traffic">–</b></div>
    <div class="mob-links"><a href="how-it-works.html#solves">How it works</a> · <a href="how-it-works.html#peers">vs. other meshes</a> · <a href="mesh.html">6-peer kernel</a> · <a href="toy/">scaling toy</a></div>
  </div>
</div>
<div id="foot" class="panel-hidden">
  <div class="panel-head" style="margin-bottom:4px">
    <b style="color:#fff;font-size:11.5px">How the Rulebook &amp; Transport Tiers Globe Works</b>
    <button class="panel-close" id="close-foot" type="button" title="Hide info">✕</button>
  </div>
  <div id="foot-measured" style="margin:0 0 8px;padding:6px 8px;border:1px solid var(--line);border-radius:8px">
    <b style="color:#fff">Measured from a live mesh.</b> The serve seat's cost model (open a serve seat's card at street zoom) is <span class="fig" data-claim="seat1-windows">calibrated from the live production CIRIS mesh on 2026-11-10: four 60-second windows of the serve node's own counters, one of them with CPU, plus its store composition and announced-node count</span>. <span class="fig" data-claim="edge-source-read">Device behaviour is derived from edge source: edge release/45.0.0 at <code>8e46165</code> (untagged when read 2026-11-11)</span>. <span class="fig" data-claim="pull-cost-debug">The idle Pull's cost was measured on an edge debug build; a release rerun is pending.</span> Everything beyond today's fleet size is a model.
    <br><b style="color:#fff">Measured today (seat-1, aggregates only):</b> <span class="fig" data-claim="seat1-cpu-quiet">0.65 of a core in a quiet 60 s window</span> · <span class="fig" data-claim="seat1-att-queries">~329 attestation queries/s</span> · <span class="fig" data-claim="seat1-rounds">~10–12 replication rounds per minute per plane (quiet window)</span> · <span class="fig" data-claim="seat1-store">store of ~42.5k attestations, ~18.3k transport destinations, ~8.7k keys</span> · <span class="fig" data-claim="seat1-announced">1,349 announced nodes under 1,338 owners</span> · <span class="fig" data-claim="seat1-no-revocations">0 key-revocation rows</span>. On screen, only the seat card's <i>seat-1 measured</i> line shows these directly.
    <br><b style="color:#fff">Map data:</b> coastlines, land, lakes, rivers, country borders and place names are from <a href="https://www.naturalearthdata.com/" target="_blank" rel="noopener">Natural Earth</a> (public domain), at 110m, 50m and 10m scale by zoom. The terrain shading is Natural Earth's 1:10m shaded relief (public domain), and the elevation tint and some extra lakes come from <a href="https://www.ncei.noaa.gov/products/etopo-global-relief-model" target="_blank" rel="noopener">NOAA NCEI ETOPO 2022</a> (public domain). <span class="fig" data-claim="map-rules">A lake is added where ETOPO holds a flat water surface that Natural Earth has as land, by a detection rule of this twin (MODEL), so a few lakes are missing or in pieces. The street view's ground colour is a rule of this twin (MODEL): warmth from latitude and ETOPO elevation picks one of five ground bands, high ground turns to rock, and the shaded relief lights it; no land-cover or climate data is loaded. Where the population grid holds no cell near a real city, that city's Natural Earth population stands in for it (MODEL).</span> <span class="fig" data-claim="generated-below-region">Real geography is drawn only down to region scale; it fades out before the street view fades in, and every street, home and device there is generated, never placed on a real road or building.</span> <span class="fig" data-claim="pop-grid-source">Where the population grid itself comes from is not recorded (FRONTIER).</span>
    <br><b style="color:#fff">Labels:</b> every figure on this page opens its ledger entry when you click, tap or press Enter on it: its class, what supports it, how it can be wrong, and how to reproduce it. MEASURED is read off the serve node or your own browser; DERIVED is read from source or computed by stated arithmetic from it; MODEL is a stated modelling choice or the simulation's output under stated inputs; FRONTIER has no support yet. PROVISIONAL, on the seat card, is a MODEL figure resting on an open calibration question. Nothing on this page is PROJECTED: no tier here is certified. <button type="button" class="ev-open" data-ev-open>Open the legend and the full ledger</button>
  </div>
  <span class="fig" data-claim="all-simulated">Every person and device</span> sits on populated land (<span class="fig" data-claim="pop-grid-cells">2,961 density cells</span> stitched to continent polygons) and runs the same constitutional rulebook (CC 4.5.13). Per <b>CC 5.4.3</b> (<span class="fig" data-claim="shaped-link-one-hop">"A shaped link is one hop"</span>), shaped budget classes (<span class="fig" data-claim="budget-class-rates"><code>server</code> 7,000 B/s, <code>home</code> 2,800 B/s, <code>phone-wifi</code> 1,400 B/s, <code>phone-cellular</code> 350 B/s, <code>seat</code> 33,000 B/s at ρ=0.5</span>) <span class="fig" data-claim="shaped-link-one-hop">are 1 direct hop and never forwarded; peers without a direct path carry on ordinary links</span>. Large rooms fan out via the <b>CC 6.1.6 <code>deterministic-alm</code> member relay tree</b> over sealed chunks (distinct from CC 5.3.3.2's no-SFU-relay-role; no seat decodes or selects). <span class="fig" data-claim="onion-tier">15,296 B X-Wing onion packets replace cover frames on shaped links at <b>0 extra wire bytes</b></span> (CC 5.4.6). <a href="how-it-works.html#solves">How it works</a> · <a href="how-it-works.html#peers">vs. other meshes</a> · <a href="mesh.html">6-peer kernel</a> · <a href="toy/">scaling toy</a>
</div>
<div id="layer-dock" role="group" aria-label="Globe layers"></div>
`);
// evidence:end
// ------------------------------------------------------------------ the world (population density grid)
const REG = ["N. America","Latin America","Europe","MENA","Sub-Saharan Africa","South Asia","East Asia","SE Asia & Oceania","Russia & C. Asia"];
const POP_GRID_B64="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";
let POP_GRID = null;
function initPopGrid(){
  if (POP_GRID) return POP_GRID;
  const bin=atob(POP_GRID_B64); const u8=new Uint8Array(bin.length);
  for(let i=0;i<bin.length;i++) u8[i]=bin.charCodeAt(i);
  const dv=new DataView(u8.buffer);
  const nCells=dv.getUint16(0,true), nLocs=dv.getUint16(2,true);
  const regionOfMetro=new Uint8Array(nLocs);
  for(let i=0;i<nLocs;i++) regionOfMetro[i]=u8[4+i];
  let off=4+nLocs; if(off%2===1) off++;
  const cellLat=new Float32Array(nCells), cellLon=new Float32Array(nCells), cellRad=new Float32Array(nCells);
  const cellLoc=new Uint16Array(nCells), cumW=new Float64Array(nCells);
  let wsum=0;
  for(let c=0;c<nCells;c++){
    cellLat[c]=dv.getInt16(off,true)*0.01;
    cellLon[c]=dv.getInt16(off+2,true)*0.01;
    const w=dv.getUint16(off+4,true);
    cellLoc[c]=u8[off+6];
    const meta=u8[off+7];
    cellRad[c]=(meta&0x0F)/25.0;
    wsum+=w; cumW[c]=wsum;
    off+=8;
  }
  for(let c=0;c<nCells;c++) cumW[c]/=wsum;
  POP_GRID={nCells,nLocs,regionOfMetro,cellLat,cellLon,cellRad,cellLoc,cumW};
  return POP_GRID;
}
const BYTES_PER_NODE = 4+4+2+1 + 4 + 4*4; // lat, lon, locality, activity, node_off, ~4 memberships
let W = null; // the world
function rng(seed){ let s = seed>>>0 || 1; return ()=>{ s ^= s<<13; s>>>=0; s ^= s>>>17; s ^= s<<5; s>>>=0; return s/4294967296; }; }
function gauss(r){ return Math.sqrt(-2*Math.log(r()+1e-12))*Math.cos(6.283185*r()); }
function buildWorld(n){
  const G = initPopGrid();
  const r = rng(20261105);
  const lat = new Float32Array(n), lon = new Float32Array(n), metro = new Uint16Array(n);
  const {nCells, nLocs, regionOfMetro, cellLat, cellLon, cellRad, cellLoc, cumW} = G;
  const perMetro = new Uint32Array(nLocs);
  for (let i=0;i<n;i++){
    const u=r();
    let lo=0, hi=nCells-1;
    while (lo<hi){ const mid=(lo+hi)>>>1; if (cumW[mid]<u) lo=mid+1; else hi=mid; }
    const rad=cellRad[lo];
    lat[i]=cellLat[lo]+(r()*2-1)*rad;
    lon[i]=cellLon[lo]+(r()*2-1)*rad;
    const k=cellLoc[lo];
    metro[i]=k; perMetro[k]++;
  }
  // nodes grouped by locality, for drawing rooms
  const byMetroOff = new Uint32Array(nLocs+1); for (let k=0;k<nLocs;k++) byMetroOff[k+1]=byMetroOff[k]+perMetro[k];
  const byMetro = new Uint32Array(n); const fill = new Uint32Array(nLocs);
  for (let i=0;i<n;i++){ const k=metro[i]; byMetro[byMetroOff[k]+fill[k]++]=i; }
  const byRegion=[]; for (let g=0;g<REG.length;g++){ const arr=[]; for (let k=0;k<nLocs;k++) if (regionOfMetro[k]===g) for (let j=byMetroOff[k];j<byMetroOff[k+1];j++) arr.push(byMetro[j]); byRegion.push(Uint32Array.from(arr)); }
  const pickIn = (k)=>perMetro[k]?byMetro[byMetroOff[k]+Math.floor(r()*perMetro[k])]:Math.floor(r()*n);
  const pickRegion = (g)=>{ const a=byRegion[g]; return a.length?a[Math.floor(r()*a.length)]:Math.floor(r()*n); };
  // rooms: each node's family (2–5 nodes, same locality), then community rooms; memberships as CSR both ways
  const memberships = []; // [node, room]
  let rooms = 0; const roomOff=[0]; const roomMem=[];
  const addRoom = (members)=>{ for (const m of members) { roomMem.push(m); memberships.push(m, rooms); } roomOff.push(roomMem.length); return rooms++; };
  const familyOf = new Int32Array(n).fill(-1);
  for (let i=0;i<n;i++){ if (familyOf[i]>=0) continue; const size=2+Math.floor(r()*4); const mem=[i]; for (let j=1;j<size;j++){ const c=pickIn(metro[i]); if (familyOf[c]<0 && c!==i) mem.push(c); } const id=addRoom(mem); for (const m of mem) familyOf[m]=id; }
  const nFamilies = rooms;
  // community rooms: about one room per 25 nodes, sizes lognormal 6..400; 90% locality, 9% region, 1% world
  const nCommunity = Math.max(50, Math.floor(n/25));
  for (let q=0;q<nCommunity;q++){
    const size = Math.min(400, Math.max(6, Math.round(Math.exp(2.3+gauss(r)*0.9))));
    const u=r(); const mNode=Math.floor(r()*n); const k=metro[mNode]; const mem=new Set();
    if (u<0.90 && perMetro[k]>=size) while (mem.size<size) mem.add(pickIn(k));
    else if (u<0.99) while (mem.size<size) mem.add(pickRegion(regionOfMetro[k]));
    else while (mem.size<size) mem.add(Math.floor(r()*n));
    addRoom([...mem]);
  }
  // one public room per region (large), and one global public room
  const regionRoom=new Uint32Array(REG.length); for (let g=0;g<REG.length;g++){ const mem=new Set(); const size=Math.min(byRegion[g].length, Math.max(200, Math.floor(n/400))); while (mem.size<size) mem.add(pickRegion(g)); regionRoom[g]=addRoom([...mem]); }
  const gmem=new Set(); const gsize=Math.min(n, Math.max(500, Math.floor(n/200))); while (gmem.size<gsize) gmem.add(Math.floor(r()*n)); const globalRoom = addRoom([...gmem]);
  // seats: one per ~2000 nodes per locality, at least one for populated localities
  const seatsOff=new Uint32Array(nLocs+1); const seats=[]; for (let k=0;k<nLocs;k++){ const c=perMetro[k]>0?Math.max(1,Math.round(perMetro[k]/2000)):0; for (let j=0;j<c;j++) seats.push(pickIn(k)); seatsOff[k+1]=seats.length; }
  const isSeat = new Uint8Array(n); for (const s of seats) isSeat[s]=1;
  // node → rooms CSR, family first
  const cnt=new Uint32Array(n+1); for (let i=0;i<memberships.length;i+=2) cnt[memberships[i]+1]++;
  for (let i=0;i<n;i++) cnt[i+1]+=cnt[i];
  const nodeRooms=new Uint32Array(memberships.length/2); const pos=new Uint32Array(n);
  for (let i=0;i<n;i++){ nodeRooms[cnt[i]+pos[i]++]=familyOf[i]; }
  for (let i=0;i<memberships.length;i+=2){ const nd=memberships[i], rm=memberships[i+1]; if (rm===familyOf[nd]) continue; nodeRooms[cnt[nd]+pos[nd]++]=rm; }
  return { n, nLocs, nCells, lat, lon, metro, regionOfMetro, roomOff:Uint32Array.from(roomOff), roomMem:Uint32Array.from(roomMem), nodeOff:cnt, nodeRooms, seatsOff, seats:Uint32Array.from(seats), isSeat, regionRoom, globalRoom, rooms, nFamilies, memberships: memberships.length/2 };
}
// ------------------------------------------------------------------ the engines (WASM + JS parity)
const OUT = 36; let wasm=null, mem=null, bufs=null;
let simParams = { directNatPm: 420, shapingPm: 780, onionPm: 160 };
async function loadWasm(){ try { const r = await WebAssembly.instantiateStreaming(fetch("kernel/kernel_scale.wasm?v=19"), {}); wasm = r.instance.exports; mem = wasm.memory; return true; } catch(e){ console.warn("wasm unavailable", e); return false; } }
function uploadWorld(){
  if (!wasm) return;
  const W_ = W; const A = (arr)=>{ const p = wasm.alloc(arr.byteLength); new Uint8Array(mem.buffer, p, arr.byteLength).set(new Uint8Array(arr.buffer, arr.byteOffset, arr.byteLength)); return p; };
  bufs = { lat:A(W_.lat), lon:A(W_.lon), metro:A(W_.metro), reg:A(W_.regionOfMetro), roomOff:A(W_.roomOff), roomMem:A(W_.roomMem), nodeOff:A(W_.nodeOff), nodeRooms:A(W_.nodeRooms), seatsOff:A(W_.seatsOff), seats:A(W_.seats), regionRoom:A(W_.regionRoom),
           activity: wasm.alloc(W_.n), arcs: wasm.alloc(ARC_CAP*8), out: wasm.alloc(OUT*8) };
  new Uint8Array(mem.buffer, bufs.activity, W_.n).fill(0);
}
const ARC_CAP = 420; let seed = 1;
function tickWasm(acts){
  seed = (seed*1103515245+12345)>>>0;
  if (typeof simClockForTick==="function") simClockForTick(); // the kernel's clock: who signs follows the hour (workload.rs)
  const n = wasm.tick(bufs.lat,bufs.lon,bufs.metro,bufs.reg,W.n,bufs.roomOff,bufs.roomMem,bufs.nodeOff,bufs.nodeRooms,bufs.seatsOff,bufs.seats,bufs.regionRoom,W.globalRoom,acts,seed,0x9e37,650,300,40,20,simParams.directNatPm,simParams.shapingPm,simParams.onionPm,bufs.activity,bufs.arcs,ARC_CAP,bufs.out);
  if (typeof LAYERS!=="undefined") LAYERS.onTick(); // layers read the kernel's own buffers for this tick
  return { out: new Float64Array(mem.buffer, bufs.out, OUT), arcs: new Uint32Array(mem.buffer, bufs.arcs, n*2), activity: new Uint8Array(mem.buffer, bufs.activity, W.n) };
}
let jsActivity=null, jsArcs=new Uint32Array(ARC_CAP*2);
function kmf(a1,o1,a2,o2){ const R=Math.PI/180; a1*=R;o1*=R;a2*=R;o2*=R; const d=Math.min(1,Math.sqrt(Math.sin((a2-a1)/2)**2+Math.cos(a1)*Math.cos(a2)*Math.sin((o2-o1)/2)**2)); return 2*6371*Math.asin(d); }
function pushArcJs(arcsN, from, to, tier){
  if (arcsN<ARC_CAP){ jsArcs[arcsN*2]=from>>>0; jsArcs[arcsN*2+1]=((to&0x0FFFFFFF)|((tier&0x7)<<28))>>>0; return arcsN+1; }
  return arcsN;
}
function diurnalMultiplier(tLocalSecs){
  const t = ((tLocalSecs % 86400) + 86400) % 86400;
  const hour = t / 3600;
  const a1 = (hour - 20.5) * (Math.PI / 12);
  const a2 = (hour - 20.5) * (Math.PI / 6);
  const val = 1.0 + 1.05 * Math.cos(a1) + 0.25 * Math.cos(a2);
  return val < 0.05 ? 0.05 : val;
}
function tickJs(acts){
  const {n,lat,lon,metro,regionOfMetro,roomOff,roomMem,nodeOff,nodeRooms,seatsOff,seats,regionRoom,globalRoom}=W; const o=new Float64Array(OUT); let arcsN=0;
  if (!jsActivity || jsActivity.length!==n) jsActivity=new Uint8Array(n);
  const act=jsActivity; let s=(seed=(seed*1103515245+12345)>>>0)||1; const rnd=()=>{ s^=s<<13; s>>>=0; s^=s>>>17; s^=s<<5; s>>>=0; return s; };
  const localityActs = new Uint32Array(181);
  let chunkCustodyFull = 0, chunkCustodyTotal = 0;
  let baselineA1 = 0, baselineA2 = 0, baselineA3 = 0;
  const {directNatPm, shapingPm, onionPm}=simParams;

  for (let a=0;a<acts;a++){
    o[0]++; const signer=rnd()%n; const roll=rnd()%1000; const r0=nodeOff[signer], r1=nodeOff[signer+1]; if (r1===r0) continue;
    const slat=lat[signer], slon=lon[signer], sm=metro[signer], sr=regionOfMetro[sm];
    if (sm < 181) localityActs[sm]++;

    const simTimeSec = 43200 + a * 137;
    const localSec = ((simTimeSec + (slon / 360) * 86400) % 86400 + 86400) % 86400;
    const _diurnal = diurnalMultiplier(localSec);

    let room, kind;
    if (roll<650){ room=nodeRooms[r0]; kind=0; }
    else if (roll<950){ const k=r1-r0-1; room = k===0? nodeRooms[r0] : nodeRooms[r0+1+rnd()%k]; kind=1; }
    else if (roll<990){ room=-1; kind=2; }
    else { room = (rnd()%10)===0 ? globalRoom : regionRoom[regionOfMetro[metro[signer]]]; kind=3; }
    o[12+kind]++; const stranger = (rnd()%1000)<20; o[2]++; if (stranger){ o[4]++; continue; } o[3]++;

    const sSeat0=seatsOff[sm], sSeat1=seatsOff[sm+1];
    const localSeat = sSeat1>sSeat0 ? seats[sSeat0+(signer%(sSeat1-sSeat0))] : signer;
    const isOnion = (kind===1 || kind===2) && (rnd()%1000)<onionPm;
    if (isOnion) o[22]++;
    const isLiveAlm = kind===1 && !isOnion && (a&7)===0;

    // 6-Category IMIX Payload Distribution (FSD §4.1)
    const imixRoll = rnd() % 100;
    let payloadBytes;
    if (isOnion) payloadBytes = 15296;
    else if (isLiveAlm) payloadBytes = 65536;
    else if (imixRoll < 35) payloadBytes = 512;
    else if (imixRoll < 65) payloadBytes = 16384;
    else if (imixRoll < 85) payloadBytes = 14336;
    else payloadBytes = 131072;

    const payloadShapedAllowed = payloadBytes <= 65536;
    if (!payloadShapedAllowed) o[24]++;

    let delivered=0, crossed=0;
    if (kind===3){
      // Public room / CC 6.1.6 deterministic-alm member relay tree over sealed chunks:
      // Per CC 5.4.6: "a forwarder or seat is never a fan-out node of the member relay tree;
      // the tree is the members'". Publisher is the signer (root of the member relay tree).
      const a0=roomOff[room], a1=roomOff[room+1];
      const touched=new Uint16Array(32); let touchedN=0, prevMember=signer;
      for (let i=a0;i<a1;i++){
        const m=roomMem[i]; if (m===signer) continue;
        o[2]++; o[3]++; delivered++;
        const mm=metro[m];
        if (mm===sm){ o[5]++; }
        else {
          crossed++;
          if (regionOfMetro[mm]===sr) o[6]++; else o[7]++;
          let seen=false;
          for (let t=0;t<touchedN;t++) if (touched[t]===mm){ seen=true; break; }
          if (!seen && touchedN<32){
            touched[touchedN++]=mm;
            const targetMember = m;
            o[8]+=kmf(lat[prevMember],lon[prevMember],lat[targetMember],lon[targetMember]);
            o[20]++;
            const tier = 2;
            if ((rnd()%3)===0) arcsN=pushArcJs(arcsN,prevMember,targetMember,tier);
            act[targetMember]=Math.min(255,act[targetMember]+70);
            if ((touchedN&3)===0) prevMember=targetMember;
          }
        }
        if (((i-a0+a)&31)===0) act[m]=Math.min(255,act[m]+36);
      }
      const treeHops = 1 + touchedN*2 + Math.max(1, Math.floor(delivered/12));
      const saved = Math.max(0, delivered - treeHops);
      o[21]+=saved;
      o[9]+=treeHops*payloadBytes;

      chunkCustodyTotal++;
      const chunkReach = Math.min(30, Math.floor((delivered * 28) / Math.max(1, a1 - a0)));
      if (chunkReach >= 20) chunkCustodyFull++; else o[27]++;
    } else {
      let a0, a1, members;
      if (kind===2){ members=seats; a0=sSeat0; a1=sSeat1; }
      else { members=roomMem; a0=roomOff[room]; a1=roomOff[room+1]; }
      let wireHops=0;
      for (let i=a0;i<a1;i++){
        const m=members[i]; if (m===signer) continue;
        o[2]++; o[3]++; delivered++;
        const directNat = (rnd()%1000)<directNatPm;
        if (directNat) o[23]++;
        // CC 5.4.3 & 2026-11-06 shaped-only rule: direct 1-hop only, payload <= 64 KiB
        // Chat and files ride a shaped link only when one end is shaped_only;
        // every other pair's chat crosses a direct unshaped link.
        const oneEndShapedOnly = (rnd()%1000)<shapingPm;
        const shapedHop = directNat && payloadShapedAllowed && (isOnion || (!isLiveAlm && oneEndShapedOnly));
        if (shapedHop) rnd(); // v45: no displacement (#939), slot 25 stays 0; draw kept for stream parity

        const mm=metro[m];
        if (mm===sm){
          o[5]++;
          if (shapedHop){ o[18]++; wireHops++; }
          else { o[20]++; wireHops += directNat?1:2; }
        } else {
          crossed++;
          if (regionOfMetro[mm]===sr) o[6]++; else o[7]++;
          o[8]+=kmf(slat,slon,lat[m],lon[m]);
          if (shapedHop){ o[18]++; wireHops++; }
          else { o[20]++; wireHops += directNat?1:2; }
          const arcTier = (isOnion && shapedHop) ? 3 : (shapedHop ? 0 : 2);
          if ((rnd()%48)===0) arcsN=pushArcJs(arcsN,signer,m,arcTier);
        }
        act[m]=Math.min(255,act[m]+55);
      }
      if (kind===2){
        for (let j=0;j<9;j++){
          const mRand=rnd()%n; const mm=metro[mRand]; const ms0=seatsOff[mm];
          const m = seats[ms0] ?? mRand;
          if (m===signer) continue;
          o[2]++; o[3]++; delivered++; crossed++;
          o[7]++; o[19]++; wireHops++;
          o[8]+=kmf(slat,slon,lat[m],lon[m]);
          if ((rnd()%6)===0) arcsN=pushArcJs(arcsN,localSeat,m,1);
        }
      }
      if (!isOnion) o[9]+=Math.min(wireHops,delivered)*payloadBytes;

      if (kind===1){
        chunkCustodyTotal++;
        const chunkReach = Math.min(30, Math.floor((delivered * 28) / Math.max(1, a1 - a0)));
        const isRevoked = (rnd()%250)===0;
        if (!isRevoked && chunkReach >= 20) chunkCustodyFull++; else o[27]++;
      }
    }
    act[signer]=Math.min(255,act[signer]+140);
    if (delivered>0){ o[17]++; if (crossed===0) o[16]++; }
    o[1]+=delivered; o[10]+=n*payloadBytes; o[11]+=payloadBytes+delivered*payloadBytes;

    // Baselines A1, A2, A3
    baselineA1 += payloadBytes * (2.0 + delivered);
    const sfuReps = crossed > 0 ? 9.0 : 2.0;
    baselineA2 += payloadBytes * (1.0 + sfuReps + delivered);
    const natFrac = 1.0 - Math.min(1.0, directNatPm / 1000);
    baselineA3 += payloadBytes * (1.0 + delivered * (1.25 + natFrac * 1.5));
  }

  o[26] = chunkCustodyTotal > 0 ? (chunkCustodyFull / chunkCustodyTotal) * 100 : 95.0;

  let totalFineActs = 0; for (let k=0; k<181; k++) totalFineActs += localityActs[k];
  let totalSeats = 0; for (let k=0; k<181; k++) totalSeats += (seatsOff[k+1] - seatsOff[k]);
  let normDiffSq = 0, normMotionSq = 0;
  for (let k=0; k<181; k++){
    const seatK = seatsOff[k+1] - seatsOff[k];
    const weightK = seatK / Math.max(1, totalSeats);
    const coarsePredK = totalFineActs * weightK;
    const fineActK = localityActs[k];
    const diff = fineActK - coarsePredK;
    normDiffSq += diff * diff;
    normMotionSq += fineActK * fineActK;
  }
  const rawD2 = normMotionSq > 0 ? normDiffSq / normMotionSq : 0;
  const stressMult = 1.0 + 0.6 * (1.0 - directNatPm / 1000) + 1.2 * (1.0 - shapingPm / 1000);
  const dv2 = Math.min(0.20, rawD2 * 0.12 * stressMult);
  const budgetBeta = 0.030;
  const lipschitzK = 1.012 + 0.045 * (1.0 - directNatPm / 1000) * (1.0 - Math.min(0.95, shapingPm / 1000));
  const certified = dv2 <= budgetBeta && lipschitzK <= 1.05;

  o[28] = dv2;
  o[29] = budgetBeta;
  o[30] = lipschitzK;
  o[31] = 0.15 / ((1 - 0.92) + 0.15 * 0.92);
  o[32] = certified ? 0 : Math.max(1, Math.ceil(Math.max(0, dv2 - budgetBeta) * 350 + Math.max(0, lipschitzK - 1.05) * 200));

  o[33] = baselineA1;
  o[34] = baselineA2;
  o[35] = baselineA3;

  return { out:o, arcs: jsArcs.subarray(0,arcsN*2), activity: act };
}
// ------------------------------------------------------------------ the globe (WebGL)
const cv=document.getElementById("c"); const gl=cv.getContext("webgl",{antialias:true,alpha:false,stencil:true});
function sh(t,src){ const s=gl.createShader(t); gl.shaderSource(s,src); gl.compileShader(s); if(!gl.getShaderParameter(s,gl.COMPILE_STATUS)) throw new Error(gl.isContextLost()?"WebGL context lost while compiling a shader":"shader: "+gl.getShaderInfoLog(s)); return s; }
function prog(v,f){ const p=gl.createProgram(); gl.attachShader(p,sh(gl.VERTEX_SHADER,v)); gl.attachShader(p,sh(gl.FRAGMENT_SHADER,f)); gl.linkProgram(p); if(!gl.getProgramParameter(p,gl.LINK_STATUS)) throw new Error(gl.isContextLost()?"WebGL context lost while linking a program":"program: "+gl.getProgramInfoLog(p)); return p; }
const PV=`attribute vec3 p;attribute float a;attribute float k;uniform mat3 R;uniform float S;uniform vec2 asp;uniform float dpr;uniform float uTime;uniform float uStory;uniform float uStoryCos;uniform float uStoryDensity;uniform vec3 uSeedDir;uniform vec2 uPeakDir;uniform vec3 uSunWorld;varying float va;varying float vk;varying float vz;varying float vSun;
void main(){float ph=fract(sin(dot(p,vec3(12.9898,78.233,45.164)))*43758.5453);float ph2=fract(sin(dot(p,vec3(63.7264,10.873,29.412))+floor(uTime*2.5)*0.17)*43758.5453);float shimmer=1.0;float eqLen=max(0.15,length(vec2(p.x,p.z)));float cosH=(p.x*uPeakDir.x+p.z*uPeakDir.y)/eqLen;float cos2H=2.0*cosH*cosH-1.0;float diurnal=clamp((1.0+1.05*cosH+0.25*cos2H)/2.30,0.05,1.0);float sunDotW=dot(p,uSunWorld);float isDay=smoothstep(-0.08,0.12,sunDotW);float wakeProb=mix(0.05,1.0,max(diurnal*0.85,isDay*0.75));float effA=0.0;if(uStory>0.5){if(uStoryCos>1.5){if(a<0.004){gl_Position=vec4(2.0,2.0,2.0,1.0);gl_PointSize=0.0;return;}effA=a;}else{float c=dot(p,uSeedDir);if(c<uStoryCos||ph>uStoryDensity){gl_Position=vec4(2.0,2.0,2.0,1.0);gl_PointSize=0.0;return;}effA=a>0.48?a:0.0;if(effA<0.38&&k<0.5&&ph>0.09*uStoryDensity){gl_Position=vec4(2.0,2.0,2.0,1.0);gl_PointSize=0.0;return;}}}else{effA=a>0.48?a:0.0;if(effA<0.38&&k<0.5&&ph>0.09){gl_Position=vec4(2.0,2.0,2.0,1.0);gl_PointSize=0.0;return;}}vec3 q=R*p;vz=q.z;vSun=sunDotW;va=effA*shimmer;vk=k;float zBias=(va>=0.38)?(va*0.25):0.0;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,-q.z*0.40-zBias,1.0);float base=(k>0.5)?3.4:(S>2.0?2.1:1.5);float storyBoost=(uStory>0.5&&uStoryCos>1.5)?2.2:0.0;float actBoost=(va>=0.38)?(0.5+(va-0.38)*1.2):0.0;gl_PointSize=(base+storyBoost+actBoost)*dpr;}`;
const PF=`precision mediump float;varying float va;varying float vk;varying float vz;varying float vSun;
void main(){float edge=smoothstep(-0.02,0.12,vz);if(edge<0.01)discard;vec2 pc=gl_PointCoord-vec2(0.5);float d2=dot(pc,pc);if(d2>0.24)discard;float circle=smoothstep(0.24,0.07,d2);float day=smoothstep(-0.10,0.10,vSun);vec3 dayBase=(vk>0.5)?vec3(0.56,0.88,1.0):vec3(0.36,0.66,0.94);vec3 nightBase=(vk>0.5)?vec3(0.24,0.44,0.68):vec3(0.12,0.20,0.34);vec3 base=mix(nightBase,dayBase,day);float depth=0.62+0.38*max(0.0,vz);if(va>=0.38){float t=smoothstep(0.38,0.92,va);vec3 daySpark=mix(vec3(0.95,0.38,0.24),vec3(1.0,0.64,0.24),t);vec3 nightSpark=mix(vec3(0.96,0.48,0.26),vec3(1.0,0.80,0.40),t);vec3 spark=mix(nightSpark,daySpark,day);vec3 col=mix(base*depth,spark,smoothstep(0.38,0.58,va));float al=mix(0.68,0.92,t)*edge*circle;gl_FragColor=vec4(col,al);}else{float idleAl=mix(vk>0.5?0.5:0.13,vk>0.5?0.8:0.28,day)*edge*circle;gl_FragColor=vec4(base*depth,idleAl);}}`;
const LV=`attribute vec3 p;attribute vec4 param;uniform mat3 R;uniform float S;uniform vec2 asp;uniform float uTime;varying float vt;varying float vAge;varying float vz;varying float vTier;
void main(){vec3 q=R*p;vz=q.z;gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,-q.z*0.5,1.0);vt=param.x;vAge=(uTime-param.y)/param.z;vTier=param.w;}`;
const LF=`precision mediump float;varying float vt;varying float vAge;varying float vz;varying float vTier;
void main(){if(vz<-0.06||vAge>=1.0||vAge<=0.0)discard;float env=smoothstep(0.0,0.18,vAge)*(1.0-smoothstep(0.55,1.0,vAge));float pulsePos=vAge*1.40;float dist=vt-pulsePos;float tail=0.0;if(dist<=0.0&&dist>-0.45){tail=pow(1.0+dist/0.45,1.8);}float head=exp(-dist*dist*220.0);float trace=0.14*sin(vt*3.14159);float alpha=min(1.0,env*(trace+tail*0.52+head*0.85))*smoothstep(-0.06,0.08,vz);if(alpha<0.005)discard;
vec3 colBase=vec3(1.0,0.55,0.28);vec3 colMid=vec3(1.0,0.75,0.42);vec3 colHead=vec3(1.0,0.94,0.80);
if(vTier>2.5){colBase=vec3(0.72,0.42,1.0);colMid=vec3(0.84,0.62,1.0);colHead=vec3(0.96,0.88,1.0);}
else if(vTier>1.5){colBase=vec3(0.24,0.82,0.54);colMid=vec3(0.45,0.92,0.68);colHead=vec3(0.85,1.0,0.92);}
else if(vTier>0.5){colBase=vec3(0.32,0.68,1.0);colMid=vec3(0.55,0.82,1.0);colHead=vec3(0.88,0.96,1.0);}
vec3 col=mix(colBase,colMid,tail);col=mix(col,colHead,clamp(head*1.1,0.0,1.0));gl_FragColor=vec4(col,alpha);}`;
const DV=`attribute vec2 p;uniform float S;uniform vec2 asp;varying vec2 vPos;void main(){vPos=p*1.05;gl_Position=vec4(vPos.x*S*asp.x,vPos.y*S*asp.y,0.95,1.0);}`;
const DF=`precision mediump float;uniform vec3 uSunCam;varying vec2 vPos;void main(){float r=length(vPos);if(r>1.05)discard;if(r<=1.0){float z=sqrt(max(0.0,1.0-r*r));vec3 n=vec3(vPos.x,vPos.y,z);float sunDot=dot(n,uSunCam);float day=smoothstep(-0.10,0.12,sunDot);float spec=pow(max(0.0,sunDot),2.0);vec3 nightDeep=vec3(0.008,0.012,0.024);vec3 dayDeep=vec3(0.11,0.26,0.46)+vec3(0.10,0.18,0.28)*spec;vec3 deep=mix(nightDeep,dayDeep,day);vec3 rimColor=mix(vec3(0.04,0.08,0.16),vec3(0.38,0.72,1.0),day);float rim=pow(1.0-z,2.3);vec3 col=deep+rimColor*(rim*0.54);gl_FragColor=vec4(col,1.0);}else{vec3 limbN=normalize(vec3(vPos.x,vPos.y,0.15));float limbSun=dot(limbN,uSunCam);float limbDay=smoothstep(-0.15,0.20,limbSun);float halo=smoothstep(1.05,1.0,r)*mix(0.06,0.36,limbDay);vec3 haloCol=mix(vec3(0.08,0.16,0.30),vec3(0.34,0.68,1.0),limbDay);gl_FragColor=vec4(haloCol*halo,halo);}}`;
const CV=`attribute vec3 p;uniform mat3 R;uniform float S;uniform vec2 asp;uniform vec3 uSunWorld;varying float vz;varying float vSun;void main(){vec3 q=R*p;vz=q.z;vSun=dot(p,uSunWorld);gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,-q.z*0.5,1.0);}`;
const CF=`precision mediump float;varying float vz;varying float vSun;void main(){float edge=smoothstep(-0.02,0.12,vz);if(edge<0.01)discard;float day=smoothstep(-0.10,0.12,vSun);vec3 col=mix(vec3(0.13,0.21,0.34),vec3(0.46,0.74,0.98),day);float al=mix(0.42,0.86,day)*edge;gl_FragColor=vec4(col,al);}`;
// land: the population grid's own 1.5° tiles, brighter where more people live; fades in below the planetary view
const GV=`attribute vec3 p;attribute float t;uniform mat3 R;uniform float S;uniform vec2 asp;uniform vec3 uSunWorld;varying float vz;varying float vt;varying float vSun;void main(){vec3 q=R*p;vz=q.z;vt=t;vSun=dot(p,uSunWorld);gl_Position=vec4(q.x*S*asp.x,q.y*S*asp.y,0.9,1.0);}`;
// land fill: the night side keeps 0.56 of the day's light (was 0.30) so land and relief read at region scale by night
const GF=`precision mediump float;uniform float uA;varying float vz;varying float vt;varying float vSun;void main(){if(vz<0.0)discard;float day=smoothstep(-0.10,0.12,vSun);vec3 c=mix(vec3(0.22,0.24,0.19),vec3(0.31,0.33,0.26),vt);c*=0.56+0.84*smoothstep(-0.10,0.10,vSun)*(0.45+0.55*max(vSun,0.0));gl_FragColor=vec4(c,uA*smoothstep(0.0,0.06,vz));}`;
const VF=`precision mediump float;uniform vec3 uCol;uniform float uA;varying float vz;varying float vSun;void main(){float edge=smoothstep(-0.02,0.12,vz);if(edge<0.01)discard;float day=smoothstep(-0.10,0.12,vSun);gl_FragColor=vec4(uCol*mix(0.42,1.0,day),uA*mix(0.55,1.0,day)*edge);}`;
const pP=prog(PV,PF), pL=prog(LV,LF), pD=prog(DV,DF), pC=prog(CV,CF), pG=prog(GV,GF), pV=prog(CV,VF);
let landBuf=gl.createBuffer(), landTBuf=gl.createBuffer(), landVerts=0, fanBuf=gl.createBuffer(), fanVerts=0, groundBuf=gl.createBuffer(), groundVerts=0, LAND=null;
// Land is Natural Earth's land rings plus the lakes on them, filled even-odd (geoUpload below). Every
// 110m edge is split into steps of at most COAST_STEP degrees, and the shoreline is drawn from the same
// points, so the fill's edge is the drawn coast at the planet view.
const COAST_STEP=0.5;
function densify(a,b,out){ const m=Math.max(1,Math.ceil(Math.max(Math.abs(b[0]-a[0]),Math.abs(b[1]-a[1]))/COAST_STEP)); for(let t=0;t<m;t++) out.push([a[0]+(b[0]-a[0])*t/m, a[1]+(b[1]-a[1])*t/m]); }
// The fill, drawn into the stencil buffer: per ring, a fan from one point to every ring edge, each
// fan triangle cut into strips of at most 6° so it stays near the sphere. Inverting the stencil
// per triangle leaves it odd exactly inside an odd number of rings (lakes and inland seas are holes).
// Adjacent fan triangles share their long edges point for point, so only the ring's own edges show.
function landFan(rings){
  const V=[], STEP=6;
  const zipper=(A,B)=>{ let a=0,b=0; const na=A.length-1, nb=B.length-1;
    while (a<na || b<nb){ if (a<na && (b>=nb || (a+1)/na <= (b+1)/nb)){ V.push(...A[a],...A[a+1],...B[b]); a++; } else { V.push(...A[a],...B[b],...B[b+1]); b++; } } };
  for (const r of rings){
    let ay=0, ax=0; for (const p of r){ ay+=p[0]; ax+=p[1]; } ay/=r.length; ax/=r.length;
    const spoke=(p)=>{ const K=Math.max(1,Math.ceil(Math.max(Math.abs(p[0]-ay),Math.abs(p[1]-ax))/STEP)); const o=[]; for (let k=0;k<=K;k++) o.push(xyz(ay+(p[0]-ay)*k/K, ax+(p[1]-ax)*k/K)); return o; };
    const first=spoke(r[0]); let E0=first;
    for (let i=0;i<r.length;i++){ const E1=(i+1<r.length)?spoke(r[i+1]):first; zipper(E0,E1); E0=E1; }
  }
  return new Float32Array(V);
}
// The same rings, for questions asked of a point: inside = an odd number of ring edges crossed on a
// ray due north (rings never cross the antimeridian, so a ray along a meridian sees every edge once).
function landTest(rings){
  const bins=Array.from({length:360},()=>[]);
  for (const r of rings) for (let i=0;i<r.length;i++){ const a=r[i], b=r[(i+1)%r.length]; const g=[a[0],a[1],b[0],b[1]];
    const lo=Math.floor(Math.min(a[1],b[1])+180), hi=Math.floor(Math.max(a[1],b[1])+180); for (let k=Math.max(0,lo);k<=Math.min(359,hi);k++) bins[k].push(g); }
  const bin=(lo)=>bins[Math.min(359,Math.max(0,Math.floor(lo+180)))];
  const inside=(la,lo)=>{ let n=0; for (const [a1,o1,a2,o2] of bin(lo)){ if ((o1<=lo)!==(o2<=lo)){ const y=a1+(a2-a1)*(lo-o1)/(o2-o1); if (y>la) n++; } } return (n&1)===1; };
  // a square is wholly land when its centre is inside and no ring edge enters it (Liang-Barsky clip)
  const enters=(g,y0,x0,y1,x1)=>{ let t0=0,t1=1; const dy=g[2]-g[0], dx=g[3]-g[1];
    for (const [p,q] of [[-dx,g[1]-x0],[dx,x1-g[1]],[-dy,g[0]-y0],[dy,y1-g[0]]]){ if (p===0){ if (q<0) return false; } else { const t=q/p; if (p<0){ if (t>t1) return false; if (t>t0) t0=t; } else { if (t<t0) return false; if (t<t1) t1=t; } } }
    return true; };
  const squareOnLand=(la0,lo0,d)=>{ if (!inside(la0+d/2, lo0+d/2)) return false;
    for (let k=Math.floor(lo0+180); k<=Math.floor(lo0+d+180); k++) for (const g of bins[Math.min(359,Math.max(0,k))]) if (enters(g,la0,lo0,la0+d,lo0+d)) return false;
    return true; };
  return { inside, squareOnLand };
}
function initLand(){
  const B=geoInitBase(); if (B) LAND=landTest(B.rings);
  // land-neutral ground: the whole sphere in 3° quads, shown only where the stencil says land
  const Gd=[]; for (let la=-90; la<90; la+=3) for (let lo=-180; lo<180; lo+=3){ const k=[[la,lo],[la,lo+3],[la+3,lo+3],[la+3,lo]].map(([y,x])=>xyz(y,x)); for (const i of [0,1,2,0,2,3]) Gd.push(...k[i]); }
  gl.bindBuffer(gl.ARRAY_BUFFER,groundBuf); gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(Gd),gl.STATIC_DRAW); groundVerts=Gd.length/3;
  // the population grid's tiles on top, brighter where more people live, clipped by the same stencil
  const G=initPopGrid(); const P=[], T=[]; let wmax=0;
  const dens=(c)=>{ const w=G.cumW[c]-(c?G.cumW[c-1]:0); const r=cellHalf(G,c); return w/(r*r); };
  for (let c=0;c<G.nCells;c++) wmax=Math.max(wmax,dens(c));
  for (let c=0;c<G.nCells;c++){
    const r=cellHalf(G,c), la=G.cellLat[c], lo=G.cellLon[c], t=Math.min(1,Math.log1p(9*dens(c)/wmax)/Math.log(10));
    const k=[[la-r,lo-r],[la-r,lo+r],[la+r,lo+r],[la+r,lo-r]].map(([y,x])=>xyz(y,x));
    for (const i of [0,1,2,0,2,3]){ P.push(...k[i]); T.push(t); }
  }
  gl.bindBuffer(gl.ARRAY_BUFFER,landBuf); gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(P),gl.STATIC_DRAW);
  gl.bindBuffer(gl.ARRAY_BUFFER,landTBuf); gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(T),gl.STATIC_DRAW);
  landVerts=P.length/3;
}
// ---- Natural Earth layers (public domain, naturalearthdata.com), built by tools/geo/build_ne.py.
// 110m rides in geo/ne110.js (the planet view and the land test); 50m loads once the view nears a
// continent; 10m loads per 10-degree tile at region scale. Every real layer fades out between 90
// and 30 m a pixel, before any generated street fades in (a patch's ground starts at 26 m a pixel), so real and generated never share a scale.
const GEO_MPP_50 = 6000, GEO_MPP_10 = 1200, GEO_TILE_CAP = 30, GEO_MPP_ETOPO_LAKE = 300;
const GEO = { base: null, l50: null, lakes: null, p50: false, idx10: null, pIdx: false, t10: new Map(), places: null, pPlaces: false, mpp: 1e9, used: "110m", rf: 1,
  stats(){ let b = 0; for (const t of this.t10.values()) b += t.bytes; return { used: this.used, tiles10: this.t10.size, bytes10: b, l50: !!this.l50, places: this.places ? this.places.length : 0, mpp: Math.round(this.mpp) }; } };
function geoDecode(buf){
  const u8 = new Uint8Array(buf); if (u8[0] !== 67 || u8[1] !== 71 || u8[2] !== 49) throw new Error("not a CG1 layer file");
  const q = new DataView(u8.buffer, u8.byteOffset + 4, 4).getUint32(0, true); let i = 8;
  const vi = () => { let r = 0, s = 1, b; do { b = u8[i++]; r += (b & 0x7f) * s; s *= 128; } while (b & 0x80); return r; };
  const zz = (n) => (n % 2 ? -(n + 1) / 2 : n / 2);
  const L = { 1: [], 2: [], 3: [], 4: [], 5: [] };
  while (i < u8.length){ const k = u8[i++], n = vi();
    for (let p = 0; p < n; p++){ const m = vi(), pts = new Array(m); let a = 0, o = 0;
      for (let j = 0; j < m; j++){ a += zz(vi()); o += zz(vi()); pts[j] = [a / q, o / q]; }
      if (L[k]) L[k].push(pts); } }
  return L;
}
// a layer set's GL buffers: the fill as stencil fans, and each line kind as segments lifted off the sphere
// K (a 10m tile only): ETOPO's lake rings and shores, kept apart so they draw only above
// GEO_MPP_ETOPO_LAKE (nearer in, a lake ETOPO finds on a 10m tile is a string of beads)
function geoUpload(L, step, K){
  const dens = (pl, closed) => { if (!step) return pl; const d = []; for (let j = 0; j + 1 < pl.length; j++) densify(pl[j], pl[j + 1], d); if (!closed) d.push(pl[pl.length - 1]); return d; };
  const ring = (r) => { const c = (r.length > 1 && r[0][0] === r[r.length - 1][0] && r[0][1] === r[r.length - 1][1]) ? r.slice(0, -1) : r; return dens(c.concat([c[0]]), true); };
  const rings = L[1].map(ring);
  const mk = (data) => { const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW); return { b, n: data.length / 3, bytes: data.byteLength }; };
  const seg = (pls) => { const v = []; for (const pl0 of pls){ const pl = dens(pl0, false); for (let j = 0; j + 1 < pl.length; j++){ const a = xyz(pl[j][0], pl[j][1]), b = xyz(pl[j + 1][0], pl[j + 1][1]); v.push(a[0] * 1.001, a[1] * 1.001, a[2] * 1.001, b[0] * 1.001, b[1] * 1.001, b[2] * 1.001); } } return new Float32Array(v); };
  const S = { fan: mk(landFan(rings)), lines: [mk(seg(L[2])), mk(seg(L[3])), mk(seg(L[4]))], rings };
  if (K && K.rings.length){ const kr = K.rings.map(ring); S.fanK = mk(landFan(rings.concat(kr))); S.shoreK = mk(seg(K.shores)); }
  S.bytes = S.fan.bytes + S.lines.reduce((a, l) => a + l.bytes, 0) + (S.fanK ? S.fanK.bytes + S.shoreK.bytes : 0);
  return S;
}
function geoFree(S){ gl.deleteBuffer(S.fan.b); S.lines.forEach(l => gl.deleteBuffer(l.b)); if (S.fanK){ gl.deleteBuffer(S.fanK.b); gl.deleteBuffer(S.shoreK.b); } }
function geoFetch(path){ return fetch(path).then(r => { if (!r.ok) throw new Error(path + " " + r.status); return r.arrayBuffer(); }); }
function geoInitBase(){
  if (typeof NE110_B64 === "undefined") return null;
  const bin = atob(NE110_B64), u8 = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  const L = geoDecode(u8.buffer); GEO.base = geoUpload(L, COAST_STEP); return GEO.base;
}
// the 10-degree tiles the view can see (centre = (pitch, yaw); half-extent from metres a pixel)
function geoVisibleTiles(w, h, mpp){
  const la = pitch * 180 / Math.PI, lo = ((yaw * 180 / Math.PI + 540) % 360) - 180;
  const ext = Math.min(60, 1.25 * Math.max(w, h) / 2 * mpp / 111000), lx = Math.min(180, ext / Math.max(0.15, Math.cos(la * Math.PI / 180)));
  const out = [];
  for (let a = Math.floor((la - ext) / 10) * 10; a < la + ext; a += 10){ if (a < -90 || a >= 90) continue;
    for (let o = Math.floor((lo - lx) / 10) * 10; o < lo + lx; o += 10){ const oo = ((o + 180) % 360 + 360) % 360 - 180; out.push(a + "_" + oo); } }
  return [...new Set(out)];
}
// which set draws now; it starts any fetch the view will need next
// the 50m set carries every lake the 10m set has (kind 5) and the lakes ETOPO adds (kinds 1, 2);
// with the places it is also what the street tiling stands on (tlReady), so the story asks early
function geoLoad50(){ if (GEO.p50) return; GEO.p50 = true;
  Promise.all([geoFetch("geo/ne50.bin?v=1"), geoFetch("geo/lakes.bin?v=1")]).then(([b, k]) => {
    const L = geoDecode(b), K = geoDecode(k); GEO.lakes = K;
    L[1] = L[1].concat(K[1], K[5]); L[2] = L[2].concat(K[2]); GEO.l50 = geoUpload(L, 0); }).catch(e => console.warn(e.message)); }
function geoLoadPlaces(){ if (GEO.pPlaces) return; GEO.pPlaces = true; geoFetch("geo/places.bin?v=1").then(b => { GEO.places = geoPlaces(b); }).catch(e => console.warn(e.message)); }
function geoSets(w, h, mpp){
  GEO.mpp = mpp;
  if (mpp < GEO_MPP_50 * 2) geoLoad50();
  if (mpp < GEO_MPP_50) geoLoadPlaces();
  let tiles = null;
  if (mpp < GEO_MPP_10 * 2){
    if (!GEO.pIdx){ GEO.pIdx = true; fetch("geo/ne10/index.json?v=1").then(r => r.json()).then(j => { GEO.idx10 = new Set(j.map(t => t[0] + "_" + t[1])); }).catch(e => console.warn(e.message)); }
    if (GEO.idx10 && GEO.lakes){ tiles = geoVisibleTiles(w, h, mpp).filter(k => GEO.idx10.has(k)); // after the lakes, so every tile gets its own
      for (const k of tiles){ const t = GEO.t10.get(k);
        if (t){ GEO.t10.delete(k); GEO.t10.set(k, t); continue; } // most recent last
        GEO.t10.set(k, { pending: true, bytes: 0 });
        geoFetch("geo/ne10/" + k + ".bin?v=1").then(b => { const e = GEO.t10.get(k); if (!e) return; const L = geoDecode(b), K = geoAddLakes(k); const S = geoUpload(L, 0, K); S.pending = false; GEO.t10.set(k, S); geoEvict(tiles); })
          .catch(e => { console.warn(e.message); GEO.t10.delete(k); }); }
      geoEvict(tiles); }
  }
  if (tiles && mpp < GEO_MPP_10 && tiles.length && tiles.every(k => { const t = GEO.t10.get(k); return t && !t.pending; })){ GEO.used = "10m"; return tiles.map(k => GEO.t10.get(k)); }
  if (mpp < GEO_MPP_50 && GEO.l50){ GEO.used = "50m"; return [GEO.l50]; }
  GEO.used = "110m"; return GEO.base ? [GEO.base] : [];
}
// ETOPO's lakes in a 10m tile: each ring clipped to the tile (so a lake on a tile edge is one hole,
// not two), each shore kept by the tile holding its first point. The 10m tiles load after the 50m set.
function geoAddLakes(key){
  const K = GEO.lakes, out = { rings: [], shores: [] }; if (!K) return out;
  const [la, lo] = key.split("_").map(Number), box = [la, lo, la + 10, lo + 10];
  const inT = (p) => p[0] >= box[0] && p[0] < box[2] && p[1] >= box[1] && p[1] < box[3];
  for (const r of K[1]){ let a0 = 90, a1 = -90, o0 = 180, o1 = -180; for (const p of r){ a0 = Math.min(a0, p[0]); a1 = Math.max(a1, p[0]); o0 = Math.min(o0, p[1]); o1 = Math.max(o1, p[1]); }
    if (a1 < box[0] || a0 >= box[2] || o1 < box[1] || o0 >= box[3]) continue;
    const c = (a0 >= box[0] && a1 < box[2] && o0 >= box[1] && o1 < box[3]) ? r : geoClipRing(r, box); if (c) out.rings.push(c); }
  for (const s of K[2]) if (s.length && inT(s[0])) out.shores.push(s);
  return out;
}
function geoClipRing(r, [y0, x0, y1, x1]){
  let pts = r.slice();
  for (const [ax, v, ge] of [[1, x0, true], [1, x1, false], [0, y0, true], [0, y1, false]]){
    if (!pts.length) break; const out = [];
    for (let i = 0; i < pts.length; i++){ const a = pts[(i + pts.length - 1) % pts.length], b = pts[i];
      const ia = ge ? a[ax] >= v : a[ax] <= v, ib = ge ? b[ax] >= v : b[ax] <= v;
      const cut = () => { const t = (v - a[ax]) / (b[ax] - a[ax]); const p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]; p[ax] = v; return p; };
      if (ib){ if (!ia) out.push(cut()); out.push(b); } else if (ia) out.push(cut()); }
    pts = out; }
  return pts.length >= 3 ? pts.concat([pts[0]]) : null;
}
function geoEvict(keep){
  const ks = new Set(keep || []);
  for (const [k, t] of GEO.t10){ if (GEO.t10.size <= GEO_TILE_CAP) break; if (ks.has(k) || t.pending) continue; geoFree(t); GEO.t10.delete(k); }
}
function geoPlaces(buf){
  const dv = new DataView(buf), u8 = new Uint8Array(buf); if (u8[0] !== 67 || u8[1] !== 80) throw new Error("not a CP1 file");
  const n = dv.getUint32(4, true), out = []; let i = 8; const td = new TextDecoder();
  for (let k = 0; k < n; k++){ const la = dv.getInt32(i, true) / 1e4, lo = dv.getInt32(i + 4, true) / 1e4, sr = u8[i + 8], pop = dv.getUint32(i + 9, true), nl = u8[i + 13];
    out.push({ la, lo, sr, pop, name: td.decode(u8.subarray(i + 14, i + 14 + nl)) }); i += 14 + nl; }
  return out;
}
// place names on the overlay canvas, biggest first, none overlapping; only at continent to region scale
function geoLabels(F){
  const mpp = GEO.mpp, rf = GEO.rf; if (!GEO.places || rf <= 0.01 || mpp > GEO_MPP_50) return;
  const maxSr = mpp > 3000 ? 1 : mpp > 1500 ? 3 : mpp > 700 ? 5 : mpp > 300 ? 7 : 10;
  const g = mctx, dpr = window.devicePixelRatio || 1; g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.font = "600 11px ui-monospace, SFMono-Regular, Menlo, monospace"; g.textBaseline = "middle";
  const boxes = []; let shown = 0;
  const a = rf * Math.min(1, (GEO_MPP_50 - mpp) / 1500);
  for (const p of GEO.places){ if (p.sr > maxSr) break; if (shown >= 60) break;
    const s = projLL(F, p.la, p.lo); if (s[2] < 0.05 || s[0] < 0 || s[1] < 0 || s[0] > F.w || s[1] > F.h) continue;
    const tw = g.measureText(p.name).width, bx = [s[0] - 3, s[1] - 7, s[0] + 8 + tw, s[1] + 7];
    if (boxes.some(b => !(bx[2] < b[0] || bx[0] > b[2] || bx[3] < b[1] || bx[1] > b[3]))) continue;
    boxes.push(bx); shown++;
    g.globalAlpha = a; g.fillStyle = "#e8eef6"; g.beginPath(); g.arc(s[0], s[1], 2, 0, 6.2832); g.fill();
    g.globalAlpha = a * 0.9; g.strokeStyle = "rgba(8,11,16,0.85)"; g.lineWidth = 3; g.strokeText(p.name, s[0] + 6, s[1]); g.fillText(p.name, s[0] + 6, s[1]); }
  g.globalAlpha = 1;
}
let posBuf=gl.createBuffer(), actBuf=gl.createBuffer(), kindBuf=gl.createBuffer(), discBuf=gl.createBuffer(), coastBuf=gl.createBuffer();
let linePosBuf=gl.createBuffer(), lineParamBuf=gl.createBuffer();
{ const disc=[]; for (let i=0;i<=96;i++){ const t=i/96*6.2831853; disc.push(Math.cos(t),Math.sin(t)); } const tri=[]; for (let i=0;i<96;i++){ tri.push(0,0,disc[i*2],disc[i*2+1],disc[i*2+2],disc[i*2+3]); } gl.bindBuffer(gl.ARRAY_BUFFER,discBuf); gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(tri),gl.STATIC_DRAW); }
const SEG=14, SEG_VERTS=SEG*2, MAX_ARCS=420, MAX_LINE_FLOATS=MAX_ARCS*SEG_VERTS*3, MAX_PARAM_FLOATS=MAX_ARCS*SEG_VERTS*4;
const posBuffer=new Float32Array(MAX_LINE_FLOATS), paramBuffer=new Float32Array(MAX_PARAM_FLOATS);
let activeArcs=[], arcCount=0, arcsNeedUpload=false, meshFilter=-1;
gl.bindBuffer(gl.ARRAY_BUFFER,linePosBuf); gl.bufferData(gl.ARRAY_BUFFER,posBuffer,gl.DYNAMIC_DRAW);
gl.bindBuffer(gl.ARRAY_BUFFER,lineParamBuf); gl.bufferData(gl.ARRAY_BUFFER,paramBuffer,gl.DYNAMIC_DRAW);
const startTime=performance.now(); const getTime=()=>(performance.now()-startTime)*0.001;
let yaw=8.54*Math.PI/180, pitch=47.37*Math.PI/180, zoom=2.35, targetZoom=2.35, velYaw=0, velPitch=0, drag=null, lastTouchTime=performance.now(), npts=0;
let storyCamDist=1.35, storyTargetCamDist=1.35, storyTargetYaw=8.54*Math.PI/180, storyTargetPitch=47.37*Math.PI/180, storyTargetZoom=2.35;
let storyActive=true, storyPaused=false, storyCurrentAct=0, storyActElapsed=0, storyMinimized=false;
// a pinch or wheel during the story hands the camera to the reader and pauses; play, next or previous hands it back
let storyUserCam=false;
function storyHold(){ if (!storyActive) return; storyUserCam = true; if (!storyPaused){ storyPaused = true; if (typeof updateStoryUI === "function") updateStoryUI(); } }
let uStoryCosVal=2.0, uStoryDensityVal=0.0;
const seedOriginXyz=(()=>{ const a=47.3769*Math.PI/180, o=8.5417*Math.PI/180; return new Float32Array([Math.cos(a)*Math.sin(o), Math.sin(a), Math.cos(a)*Math.cos(o)]); })();
function xyz(la,lo){ const a=la*Math.PI/180, o=lo*Math.PI/180; return [Math.cos(a)*Math.sin(o), Math.sin(a), Math.cos(a)*Math.cos(o)]; }
function uploadPoints(){ const n=W.n; const p=new Float32Array(n*3); for (let i=0;i<n;i++){ const v=xyz(W.lat[i],W.lon[i]); p[i*3]=v[0];p[i*3+1]=v[1];p[i*3+2]=v[2]; } gl.bindBuffer(gl.ARRAY_BUFFER,posBuf); gl.bufferData(gl.ARRAY_BUFFER,p,gl.STATIC_DRAW); gl.bindBuffer(gl.ARRAY_BUFFER,kindBuf); gl.bufferData(gl.ARRAY_BUFFER,W.isSeat,gl.STATIC_DRAW); gl.bindBuffer(gl.ARRAY_BUFFER,actBuf); gl.bufferData(gl.ARRAY_BUFFER,new Uint8Array(n),gl.DYNAMIC_DRAW); npts=n; }
function addArc(signer,packedTarget,nowSec){
  const target = packedTarget & 0x0FFFFFFF;
  const tier = (packedTarget >>> 28) & 0x7;
  if (!W||signer===target||signer>=W.n||target>=W.n) return;
  const a=xyz(W.lat[signer],W.lon[signer]), b=xyz(W.lat[target],W.lon[target]);
  const dot=Math.max(-1,Math.min(1,a[0]*b[0]+a[1]*b[1]+a[2]*b[2])); const th=Math.acos(dot);
  if (th<0.0005) return;
  const lift=0.03+0.22*(th/Math.PI); const duration=3.8+Math.min(1.0,th/Math.PI)*2.2;
  const verts=new Float32Array(SEG_VERTS*3), params=new Float32Array(SEG_VERTS*4);
  let prev=null, vw=0, pw=0;
  for (let s=0;s<=SEG;s++){
    const t=s/SEG;
    const w1=th<1e-4?1-t:Math.sin((1-t)*th)/Math.sin(th), w2=th<1e-4?t:Math.sin(t*th)/Math.sin(th);
    let v=[a[0]*w1+b[0]*w2, a[1]*w1+b[1]*w2, a[2]*w1+b[2]*w2];
    const L=Math.hypot(...v)||1; const h=1.0+lift*Math.sin(t*Math.PI);
    v=[v[0]/L*h, v[1]/L*h, v[2]/L*h];
    if (prev){
      verts[vw++]=prev[0]; verts[vw++]=prev[1]; verts[vw++]=prev[2];
      verts[vw++]=v[0];    verts[vw++]=v[1];    verts[vw++]=v[2];
      const tPrev=(s-1)/SEG;
      params[pw++]=tPrev; params[pw++]=nowSec; params[pw++]=duration; params[pw++]=tier;
      params[pw++]=t;     params[pw++]=nowSec; params[pw++]=duration; params[pw++]=tier;
    }
    prev=v;
  }
  activeArcs.push({birthTime:nowSec,duration,tier,verts,params,rec:(typeof gfxArcRec==="function")?gfxArcRec(signer,target,nowSec,duration,tier):null});
}
function pruneAndPackArcs(nowSec){
  let changed=false, w=0;
  for (let i=0;i<activeArcs.length;i++){
    const arc=activeArcs[i];
    if (nowSec-arc.birthTime<arc.duration) activeArcs[w++]=arc;
    else changed=true;
  }
  activeArcs.length=w;
  if (activeArcs.length>MAX_ARCS){ activeArcs.splice(0,activeArcs.length-MAX_ARCS); changed=true; }
  if (changed||arcsNeedUpload){
    let offPos=0, offParam=0, drawn=0;
    const chunkPos=SEG_VERTS*3, chunkParam=SEG_VERTS*4;
    for (let i=0;i<activeArcs.length;i++){
      const arc=activeArcs[i];
      if (meshFilter>=0 && arc.tier!==meshFilter) continue;
      posBuffer.set(arc.verts,offPos);
      paramBuffer.set(arc.params,offParam);
      offPos+=chunkPos; offParam+=chunkParam; drawn++;
    }
    arcCount=drawn;
    if (offPos>0){
      gl.bindBuffer(gl.ARRAY_BUFFER,linePosBuf); gl.bufferSubData(gl.ARRAY_BUFFER,0,posBuffer.subarray(0,offPos));
      gl.bindBuffer(gl.ARRAY_BUFFER,lineParamBuf); gl.bufferSubData(gl.ARRAY_BUFFER,0,paramBuffer.subarray(0,offParam));
    }
    arcsNeedUpload=false;
  }
}
function getSimUtcHours(nowSec){
  if (storyActive) {
    const oa = Math.max(0, storyCurrentAct - 1 - STORY_LEAD);
    const d = Math.max(1, STORY_ACTS[storyCurrentAct].duration);
    if (oa < 3) return 7.2 + oa * 0.7 + Math.min(1, storyActElapsed / d) * 0.7;
    if (oa === 3) return 9.3 + (storyActElapsed / d) * 3.2;
    return (8.5 + storyActElapsed * 0.85) % 24.0;
  }
  if (typeof SIM!=="undefined") return ((SIM.t/3600)%24+24)%24; // the sim's clock (gfx.js): it stops when the sim is paused
  return (8.5 + nowSec * 0.85) % 24.0;
}
function diurnalAtLon(lonDeg, nowSec){
  const localHour = ((getSimUtcHours(nowSec) + lonDeg / 15.0) % 24.0 + 24.0) % 24.0;
  const a1 = (localHour - 15.0) * (Math.PI / 12);
  const a2 = (localHour - 15.0) * (Math.PI / 6);
  return Math.max(0.06, Math.min(1.0, (1.0 + 1.05 * Math.cos(a1) + 0.25 * Math.cos(a2)) / 2.30));
}
function setArcs(arcs){
  const k=arcs.length/2, nowSec=getTime();
  const maxAdd=playing?10:24; const step=Math.max(1,Math.floor(k/maxAdd));
  const z0=seedOriginXyz;
  for (let i=0;i<k;i+=step){
    const s=arcs[i*2], packed=arcs[i*2+1], t=packed&0x0FFFFFFF;
    if (!W||s>=W.n||t>=W.n) continue;
    if (storyActive && uStoryCosVal>-0.98){
      const vs=xyz(W.lat[s],W.lon[s]), vt=xyz(W.lat[t],W.lon[t]);
      const cs=vs[0]*z0[0]+vs[1]*z0[1]+vs[2]*z0[2];
      const ct=vt[0]*z0[0]+vt[1]*z0[1]+vt[2]*z0[2];
      if (cs<uStoryCosVal||ct<uStoryCosVal) continue;
    }
    addArc(s,packed,nowSec);
  }
  arcsNeedUpload=true; pruneAndPackArcs(nowSec);
}
function rot(){ const cy=Math.cos(yaw),sy=Math.sin(yaw),cp=Math.cos(pitch),sp=Math.sin(pitch);
  return new Float32Array([cy,-sp*sy,cp*sy, 0,cp,sp, -sy,-sp*cy,cp*cy]); }
function draw(){
  const w=cv.clientWidth, h=cv.clientHeight; const dpr=(typeof gfxFrameBegin==="function")?gfxFrameBegin(w,h):(window.devicePixelRatio||1);
  if (typeof gfxFrameBegin!=="function" && (cv.width!==w*dpr|0||cv.height!==h*dpr|0)){ cv.width=w*dpr|0; cv.height=h*dpr|0; }
  gl.viewport(0,0,cv.width,cv.height); gl.clearColor(0.043,0.051,0.07,1); gl.disable(gl.DEPTH_TEST); gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT); gl.enable(gl.BLEND); gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);
  const asp=w>h? [h/w,1]:[1,w/h]; const S=0.86*zoom; const R=rot(); const nowSec=getTime();
  const simUtc=getSimUtcHours(nowSec);
  // the sun is the kernel's (clock_sun on the sim's clock); without gfx.js, the f73b918 sun
  const sunLon=(12.0-simUtc)*15.0*Math.PI/180, sunLat=18.0*Math.PI/180;
  const sunWorld=(typeof gfxSun==="function")?gfxSun(simUtc):[Math.cos(sunLat)*Math.sin(sunLon), Math.sin(sunLat), Math.cos(sunLat)*Math.cos(sunLon)];
  const sx=sunWorld[0], sy=sunWorld[1], sz=sunWorld[2];
  const sunCam=[R[0]*sx+R[3]*sy+R[6]*sz, R[1]*sx+R[4]*sy+R[7]*sz, R[2]*sx+R[5]*sy+R[8]*sz];
  const peakLon=(15.0-simUtc)*15.0*Math.PI/180;
  const peakDir=[Math.sin(peakLon), Math.cos(peakLon)];
  if (typeof gfxOcean==="function"&&(!window.LAYERS||LAYERS.on("illustration"))) gfxOcean(S,asp,sunCam,false); else { gl.useProgram(pD); gl.bindBuffer(gl.ARRAY_BUFFER,discBuf); const dp=gl.getAttribLocation(pD,"p"); gl.enableVertexAttribArray(dp); gl.vertexAttribPointer(dp,2,gl.FLOAT,false,0,0); gl.uniform1f(gl.getUniformLocation(pD,"S"),S); gl.uniform2fv(gl.getUniformLocation(pD,"asp"),asp); gl.uniform3fv(gl.getUniformLocation(pD,"uSunCam"),sunCam); gl.drawArrays(gl.TRIANGLES,0,96*3); }
  const landA=Math.max(0,Math.min(1,(zoom-3)/9));
  const mpp=EARTH_M/(0.86*zoom*Math.min(w,h)/2), gsets=geoSets(w,h,mpp); GEO.rf=sstep(30,90,mpp);
  if (landVerts){ gl.useProgram(pG); gl.uniformMatrix3fv(gl.getUniformLocation(pG,"R"),false,R); gl.uniform1f(gl.getUniformLocation(pG,"S"),S); gl.uniform2fv(gl.getUniformLocation(pG,"asp"),asp); gl.uniform3fv(gl.getUniformLocation(pG,"uSunWorld"),sunWorld); gl.uniform1f(gl.getUniformLocation(pG,"uA"),landA);
    const gp=gl.getAttribLocation(pG,"p"), gt=gl.getAttribLocation(pG,"t");
    // 1. the rings' parity into the stencil (no colour), 2. ground where odd, 3. population tiles where odd
    gl.enable(gl.STENCIL_TEST); gl.stencilMask(1); gl.clearStencil(0); gl.clear(gl.STENCIL_BUFFER_BIT);
    gl.colorMask(false,false,false,false); gl.stencilFunc(gl.ALWAYS,0,1); gl.stencilOp(gl.KEEP,gl.KEEP,gl.INVERT);
    gl.disableVertexAttribArray(gt); gl.vertexAttrib1f(gt,0);
    gl.enableVertexAttribArray(gp);
    if (gsets.length) for (const G of gsets){ const fan=(G.fanK && mpp>=GEO_MPP_ETOPO_LAKE)?G.fanK:G.fan; gl.bindBuffer(gl.ARRAY_BUFFER,fan.b); gl.vertexAttribPointer(gp,3,gl.FLOAT,false,0,0); gl.drawArrays(gl.TRIANGLES,0,fan.n); }
    else { gl.bindBuffer(gl.ARRAY_BUFFER,fanBuf); gl.vertexAttribPointer(gp,3,gl.FLOAT,false,0,0); gl.drawArrays(gl.TRIANGLES,0,fanVerts); }
    gl.colorMask(true,true,true,true); gl.stencilFunc(gl.EQUAL,1,1); gl.stencilOp(gl.KEEP,gl.KEEP,gl.KEEP);
    // ground reaches full cover sooner than the tiles fade in, so empty land never reads as shallow sea
    gl.uniform1f(gl.getUniformLocation(pG,"uA"),(0.93+0.07*Math.min(1,landA*3))*(1-0.6*sstep(26,8,mpp))); // near-opaque: the sea's glint and colour stay off the land // land shows at every zoom; at street scale it is a faint land/sea mask
    gl.bindBuffer(gl.ARRAY_BUFFER,groundBuf); gl.vertexAttribPointer(gp,3,gl.FLOAT,false,0,0); gl.drawArrays(gl.TRIANGLES,0,groundVerts);
    gl.uniform1f(gl.getUniformLocation(pG,"uA"),landA);
    gl.bindBuffer(gl.ARRAY_BUFFER,landBuf); gl.vertexAttribPointer(gp,3,gl.FLOAT,false,0,0);
    gl.bindBuffer(gl.ARRAY_BUFFER,landTBuf); gl.enableVertexAttribArray(gt); gl.vertexAttribPointer(gt,1,gl.FLOAT,false,0,0);
    gl.drawArrays(gl.TRIANGLES,0,landVerts); gl.disableVertexAttribArray(gt);
    // 4. terrain: shaded relief and elevation (relief.js), a multiply on the land just drawn
    if (typeof reliefDraw==="function") reliefDraw(R,S,asp,w,h,mpp,(0.6+0.4*Math.min(1,landA*3))*GEO.rf,sunWorld);
    // 5. the night side: the population grid (MODEL input) glows where the kernel's sun has set
    if (typeof gfxNightGlow==="function" && (typeof LAYERS==="undefined" || LAYERS.on("glow"))) gfxNightGlow(R,S,asp,mpp,1.0);
    gl.disable(gl.STENCIL_TEST); }
  // shorelines, country borders and rivers from the set in use; they fade out before streets fade in
  if (gsets.length && GEO.rf>0.004){ gl.useProgram(pV); gl.uniformMatrix3fv(gl.getUniformLocation(pV,"R"),false,R); gl.uniform1f(gl.getUniformLocation(pV,"S"),S); gl.uniform2fv(gl.getUniformLocation(pV,"asp"),asp); gl.uniform3fv(gl.getUniformLocation(pV,"uSunWorld"),sunWorld);
    const vp=gl.getAttribLocation(pV,"p"); gl.enableVertexAttribArray(vp);
    const near=sstep(GEO_MPP_50*2,GEO_MPP_50/2,mpp);
    const LS=[[[0.46,0.74,0.98],0.86],[[0.93,0.78,0.52],0.30+0.40*near],[[0.38,0.62,0.95],0.18+0.42*near]];
    LS.push(LS[0]); const nk=mpp>=GEO_MPP_ETOPO_LAKE?4:3; // 3: ETOPO's shores on a 10m tile, with its lake fill
    // anti-aliased quads, widths in CSS pixels (coast, border, river, ETOPO shore); gl.LINES without instancing
    if (!(typeof gfxLines==="function" && gfxLines(R,S,asp,gsets,LS,nk,[1.15,0.9,0.8,1.0])))
    for (let k=0;k<nk;k++){ gl.uniform3fv(gl.getUniformLocation(pV,"uCol"),LS[k][0]); gl.uniform1f(gl.getUniformLocation(pV,"uA"),LS[k][1]*GEO.rf);
      for (const G of gsets){ const L=(k===3)?G.shoreK:G.lines[k]; if (!L || !L.n) continue; gl.bindBuffer(gl.ARRAY_BUFFER,L.b); gl.vertexAttribPointer(vp,3,gl.FLOAT,false,0,0); gl.drawArrays(gl.LINES,0,L.n); } } }
  if (npts){ gl.enable(gl.DEPTH_TEST); gl.depthFunc(gl.LEQUAL); gl.clear(gl.DEPTH_BUFFER_BIT);
    gl.useProgram(pP); gl.uniformMatrix3fv(gl.getUniformLocation(pP,"R"),false,R); gl.uniform1f(gl.getUniformLocation(pP,"S"),S); gl.uniform2fv(gl.getUniformLocation(pP,"asp"),asp); gl.uniform1f(gl.getUniformLocation(pP,"dpr"),dpr); gl.uniform1f(gl.getUniformLocation(pP,"uTime"),nowSec);
    gl.uniform1f(gl.getUniformLocation(pP,"uStory"),storyActive?1.0:0.0);
    gl.uniform1f(gl.getUniformLocation(pP,"uStoryCos"),uStoryCosVal);
    gl.uniform1f(gl.getUniformLocation(pP,"uStoryDensity"),uStoryDensityVal);
    gl.uniform3fv(gl.getUniformLocation(pP,"uSeedDir"),seedOriginXyz);
    gl.uniform2fv(gl.getUniformLocation(pP,"uPeakDir"),peakDir);
    gl.uniform3fv(gl.getUniformLocation(pP,"uSunWorld"),sunWorld);
    const ap=gl.getAttribLocation(pP,"p"), aa=gl.getAttribLocation(pP,"a"), ak=gl.getAttribLocation(pP,"k");
    gl.bindBuffer(gl.ARRAY_BUFFER,posBuf); gl.enableVertexAttribArray(ap); gl.vertexAttribPointer(ap,3,gl.FLOAT,false,0,0);
    gl.bindBuffer(gl.ARRAY_BUFFER,actBuf); gl.enableVertexAttribArray(aa); gl.vertexAttribPointer(aa,1,gl.UNSIGNED_BYTE,true,0,0);
    gl.bindBuffer(gl.ARRAY_BUFFER,kindBuf); gl.enableVertexAttribArray(ak); gl.vertexAttribPointer(ak,1,gl.UNSIGNED_BYTE,false,0,0);
    // below the planetary view, people sit on the same land the fill shows: the stencil still holds it
    if (landVerts){ gl.enable(gl.STENCIL_TEST); gl.stencilFunc(gl.EQUAL,1,1); gl.stencilOp(gl.KEEP,gl.KEEP,gl.KEEP); }
    gl.drawArrays(gl.POINTS,0,npts); gl.disable(gl.STENCIL_TEST);
    gl.disable(gl.DEPTH_TEST); }
  // the street tiling (tiling.js): holons over the land, clipped by the same land stencil
  if (typeof tlDraw==="function") tlDraw(w,h,mpp,sunWorld);
  // arcs: the tick's sample (instanced, depth-correct) and every layer's arcs
  const arcSets=[]; if (showArcs && activeArcs.length && (typeof LAYERS==="undefined" || LAYERS.on("arcs"))) arcSets.push(activeArcs.filter(a=>a.rec && (meshFilter<0 || a.tier===meshFilter)).map(a=>a.rec));
  if (typeof LAYERS!=="undefined") LAYERS.arcSets(arcSets, nowSec);
  const arcsDone = typeof gfxArcsDraw==="function" && gfxArcsDraw(R,S,asp,nowSec,arcSets);
  if (!arcsDone && arcCount>0 && showArcs){ gl.useProgram(pL); gl.uniformMatrix3fv(gl.getUniformLocation(pL,"R"),false,R); gl.uniform1f(gl.getUniformLocation(pL,"S"),S); gl.uniform2fv(gl.getUniformLocation(pL,"asp"),asp); gl.uniform1f(gl.getUniformLocation(pL,"uTime"),nowSec);
    const lp=gl.getAttribLocation(pL,"p"), lparam=gl.getAttribLocation(pL,"param");
    gl.bindBuffer(gl.ARRAY_BUFFER,linePosBuf); gl.enableVertexAttribArray(lp); gl.vertexAttribPointer(lp,3,gl.FLOAT,false,0,0);
    gl.bindBuffer(gl.ARRAY_BUFFER,lineParamBuf); gl.enableVertexAttribArray(lparam); gl.vertexAttribPointer(lparam,4,gl.FLOAT,false,0,0);
    gl.drawArrays(gl.LINES,0,arcCount*SEG_VERTS); }
  // aerial perspective over land and sea (illustration), then each layer's GL marks
  if (typeof gfxOcean==="function"&&(!window.LAYERS||LAYERS.on("illustration"))) gfxOcean(S,asp,sunCam,true); // illustration layer off: the flat disc above, no haze
  if (typeof LAYERS!=="undefined") LAYERS.drawGL(R,S,asp,w,h,mpp,nowSec);
}
// rotation per pixel: the old feel near the planet; at depth, exactly one pixel of ground per pixel dragged
const dragK=()=>zoom>3?2/(0.86*zoom*Math.min(cv.clientWidth,cv.clientHeight)):0.0035/Math.max(1,zoom*0.6);
// ---- camera input. Every zoom gesture goes through zoomBy and every pan through panBy: mouse wheel,
// trackpad pinch (Chrome and Firefox send it as wheel with ctrlKey; Safari as gesture events), a
// two-finger touch pinch and pan, double click or double tap, the on-screen buttons and the keys.
const ZOOM_MIN=0.6, ZOOM_HOME=1.0;
let zoomAnchor=null; // [x, y, when]: the canvas point the ground stays under while the zoom eases
const cvXY=(x,y)=>{ const r=cv.getBoundingClientRect(); return [x-r.left,y-r.top]; };
function zoomBy(f,x,y){ if(!(f>0)||!isFinite(f)) return; storyHold(); targetZoom=Math.max(ZOOM_MIN,Math.min(zoomMax(),targetZoom*f)); zoomAnchor=x==null?null:[x,y,performance.now()]; lastTouchTime=performance.now(); }
function panBy(dx,dy,coast){ const k=dragK(), vy=-dx*k/Math.max(0.2,Math.cos(pitch)), vp=dy*k; yaw+=vy; pitch=Math.max(-1.4,Math.min(1.4,pitch+vp)); if(coast){ velYaw=vy; velPitch=vp; } lastTouchTime=performance.now(); }
function resetView(){ storyHold(); targetZoom=ZOOM_HOME; zoomAnchor=null; velYaw=0; velPitch=0; lastTouchTime=performance.now(); }
// called once a frame with the zoom before this frame's ease: turn the globe so the anchored point stays put
function zoomAnchorStep(z0,now){
  if(!zoomAnchor) return; if(drag||now-zoomAnchor[2]>1400){ zoomAnchor=null; return; }
  const f=zoom/z0; if(f===1) return; const F=camFrame(), ox=zoomAnchor[0]-F.w/2, oy=zoomAnchor[1]-F.h/2;
  if(Math.hypot(ox,oy)>0.92*F.k) return; // the pointer is off the planet: zoom about the centre
  yaw-=ox*(1-f)/F.k/Math.max(0.2,Math.cos(pitch)); pitch=Math.max(-1.4,Math.min(1.4,pitch+oy*(1-f)/F.k));
}
cv.addEventListener("mousedown",e=>{ if(e.button!==0) return; drag=[e.clientX,e.clientY];velYaw=0;velPitch=0;cv.style.cursor="grabbing";lastTouchTime=performance.now();}); window.addEventListener("mouseup",()=>{drag=null;cv.style.cursor="grab";});
window.addEventListener("mousemove",e=>{ if(!drag) return; panBy(e.clientX-drag[0],e.clientY-drag[1],true); drag=[e.clientX,e.clientY]; });
cv.addEventListener("wheel",e=>{ e.preventDefault();
  let d=e.deltaY*(e.deltaMode===1?33:e.deltaMode===2?400:1); d=Math.max(-240,Math.min(240,d));
  // a trackpad pinch: Chrome's own scale is exp(-deltaY/100); a wheel notch (about 100 to 120) is 1.5x
  const [x,y]=cvXY(e.clientX,e.clientY); zoomBy(Math.exp(-d*(e.ctrlKey?0.01:0.0035)),x,y); },{passive:false});
cv.addEventListener("dblclick",e=>{ e.preventDefault(); if(performance.now()-lastTouchEnd<800) return; /* a double tap already zoomed */ const [x,y]=cvXY(e.clientX,e.clientY); zoomBy(e.shiftKey?0.4:2.5,x,y); });
// Safari sends a trackpad pinch as gesture events (and on iOS beside the touches, which then win)
let gestScale=1;
cv.addEventListener("gesturestart",e=>{ e.preventDefault(); gestScale=1; });
cv.addEventListener("gesturechange",e=>{ e.preventDefault(); if(pinch||!(e.scale>0)) return; const [x,y]=cvXY(e.clientX,e.clientY); zoomBy(e.scale/gestScale,x,y); gestScale=e.scale; });
cv.addEventListener("gestureend",e=>{ e.preventDefault(); });
let pinch=null, lastTap=null, lastTouchEnd=-1e9; // pinch: the pair's spread and midpoint now, and where they started
const touchPair=(e)=>{ const a=e.touches[0], b=e.touches[1], [x,y]=cvXY((a.clientX+b.clientX)/2,(a.clientY+b.clientY)/2); return [Math.hypot(a.clientX-b.clientX,a.clientY-b.clientY),x,y]; };
cv.addEventListener("touchstart",e=>{
  if(e.touches.length===2){ drag=null; velYaw=0; velPitch=0; const [d,x,y]=touchPair(e); pinch={d,x,y,d0:d,x0:x,y0:y,t0:performance.now(),moved:false}; }
  else if(e.touches.length===1){ const t=e.touches[0]; drag=[t.clientX,t.clientY,t.clientX,t.clientY,performance.now()]; velYaw=0; velPitch=0; }
  else { pinch=null; drag=null; }
  lastTouchTime=performance.now();
},{passive:true});
cv.addEventListener("touchmove",e=>{
  if(e.cancelable) e.preventDefault(); // the canvas owns its gestures: no page scroll, no page zoom
  if(e.touches.length===2&&pinch){
    const [d,x,y]=touchPair(e);
    if(Math.abs(d-pinch.d)>0.5||Math.hypot(x-pinch.x,y-pinch.y)>0.5){
      if(Math.abs(d-pinch.d0)>8||Math.hypot(x-pinch.x0,y-pinch.y0)>8) pinch.moved=true;
      if(d>8&&pinch.d>8) zoomBy(d/pinch.d,x,y);
      panBy(x-pinch.x,y-pinch.y,false); pinch.d=d; pinch.x=x; pinch.y=y;
    }
    return;
  }
  if(!drag||e.touches.length!==1) return;
  const t=e.touches[0]; panBy(t.clientX-drag[0],t.clientY-drag[1],true); drag[0]=t.clientX; drag[1]=t.clientY;
},{passive:false});
cv.addEventListener("touchend",e=>{
  const now=performance.now();
  lastTouchEnd=now;
  if(pinch&&e.touches.length<2){
    // two fingers down and up without moving: zoom out (the map convention)
    if(!pinch.moved&&now-pinch.t0<320) zoomBy(0.5,pinch.x,pinch.y);
    pinch=null; lastTap=null;
    if(e.touches.length===1){ const t=e.touches[0]; drag=[t.clientX,t.clientY,t.clientX,t.clientY,now-1000]; } // the finger left carries on as a pan
    return;
  }
  if(e.touches.length===0){
    if(drag&&drag.length>4&&now-drag[4]<300&&Math.hypot(drag[0]-drag[2],drag[1]-drag[3])<10){
      // a tap; a second one close by and soon after zooms in
      if(lastTap&&now-lastTap[2]<340&&Math.hypot(drag[0]-lastTap[0],drag[1]-lastTap[1])<32){ const [x,y]=cvXY(drag[0],drag[1]); zoomBy(2.5,x,y); lastTap=null; }
      else lastTap=[drag[0],drag[1],now];
    }
    drag=null;
  }
});
cv.addEventListener("touchcancel",()=>{ pinch=null; drag=null; });
// keys: + and - zoom, the arrows pan, 0 shows the whole planet; never while typing in a field
addEventListener("keydown",e=>{
  if(e.defaultPrevented||e.metaKey||e.ctrlKey||e.altKey) return; const t=e.target;
  if(t&&(/^(INPUT|SELECT|TEXTAREA)$/.test(t.tagName)||t.isContentEditable)) return;
  if(e.key.startsWith("Arrow")&&t&&t.closest&&t.closest("#hud,#stats,#foot,#sf-menu,#node-card,#story-hud,#ev-panel,#claim-card")) return; // a focused panel scrolls
  const step=Math.min(cv.clientWidth,cv.clientHeight)*0.12;
  if(e.key==="+"||e.key==="=") zoomBy(1.6); else if(e.key==="-"||e.key==="_") zoomBy(1/1.6); else if(e.key==="0") resetView();
  else if(e.key==="ArrowLeft"){ storyHold(); panBy(step,0,false); } else if(e.key==="ArrowRight"){ storyHold(); panBy(-step,0,false); }
  else if(e.key==="ArrowUp"){ storyHold(); panBy(0,step,false); } else if(e.key==="ArrowDown"){ storyHold(); panBy(0,-step,false); }
  else return;
  e.preventDefault();
});
// the on-screen cluster: a press steps once, a held press repeats
(function zoomButtons(){
  const el=document.createElement("div"); el.id="zoomctl"; el.setAttribute("role","group"); el.setAttribute("aria-label","Map zoom");
  el.innerHTML='<button type="button" id="zc-in" aria-label="Zoom in" title="Zoom in (+)">+</button><button type="button" id="zc-out" aria-label="Zoom out" title="Zoom out (-)">&minus;</button><button type="button" id="zc-home" aria-label="Show the whole planet" title="Whole planet (0)"><svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><circle cx="12" cy="12" r="8.2" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M3.8 12h16.4M12 3.8c-3.4 3.2-3.4 13.2 0 16.4M12 3.8c3.4 3.2 3.4 13.2 0 16.4" fill="none" stroke="currentColor" stroke-width="1.3"/></svg></button>';
  document.body.appendChild(el);
  let rep=0; const stop=()=>{ if(rep){ clearInterval(rep); rep=0; } };
  const hold=(id,f)=>{ const b=el.querySelector(id);
    b.addEventListener("pointerdown",e=>{ if(e.button) return; e.preventDefault(); stop(); zoomBy(f); rep=setInterval(()=>zoomBy(f),190); });
    b.addEventListener("click",e=>{ if(e.detail===0) zoomBy(f); }); // the keyboard's Enter or Space
    for(const ev of ["pointerup","pointerleave","pointercancel","blur"]) b.addEventListener(ev,stop); };
  hold("#zc-in",1.7); hold("#zc-out",1/1.7);
  el.querySelector("#zc-home").addEventListener("click",resetView);
  addEventListener("pointerup",stop);
})();
// ------------------------------------------------------------------ the loop
const isMobileViewport = window.innerWidth <= 800;
const DEVICES_PER_PERSON = 2.25; // CC 3.3.6.3 self roster: ~1 phone + ~1 home/laptop/tablet + ~0.25 shared/personal server
let playing=false, useWasm=true, showArcs=true, cum=0, lastActivity=null, activeBlocks=[];
let currentPop=5000000000, currentSize=isMobileViewport?100000:1000000, currentActs=isMobileViewport?2000:10000, avgMsPerTick=0, avgMsPerAct=0;
const $=(id)=>document.getElementById(id);
const fmt=(x)=>x>=1e12?(x/1e12).toFixed(2)+" T":x>=1e9?(x/1e9).toFixed(2)+" B":x>=1e6?(x/1e6).toFixed(2)+" M":x>=1e3?(x/1e3).toFixed(1)+" k":String(Math.round(x));
const fmtBytes=(x)=>x>=1e18?(x/1e18).toFixed(2)+" EB":x>=1e15?(x/1e15).toFixed(2)+" PB":x>=1e12?(x/1e12).toFixed(2)+" TB":x>=1e9?(x/1e9).toFixed(2)+" GB":x>=1e6?(x/1e6).toFixed(2)+" MB":x>=1e3?(x/1e3).toFixed(1)+" KB":Math.round(x)+" B";
if (isMobileViewport){
  $("acts").value=String(currentActs);
  document.querySelectorAll("#acts-seg button").forEach(b=>b.classList.toggle("active",+b.getAttribute("data-val")===currentActs));
}
function togglePanel(panelId, btnId){
  const p = $(panelId), b = $(btnId);
  if (!p) return;
  const hidden = p.classList.toggle("panel-hidden");
  if (b) b.classList.toggle("open", !hidden);
  if (!hidden && window.innerWidth <= 800){
    ["hud","stats","foot"].forEach(other=>{
      if (other !== panelId && $(other)){
        $(other).classList.add("panel-hidden");
        const ob = $("btn-"+other); if (ob) ob.classList.remove("open");
      }
    });
  }
}
if ($("btn-hud")) $("btn-hud").onclick=()=>togglePanel("hud","btn-hud");
if ($("btn-stats")) $("btn-stats").onclick=()=>togglePanel("stats","btn-stats");
if ($("btn-foot")) $("btn-foot").onclick=()=>togglePanel("foot","btn-foot");
if ($("close-hud")) $("close-hud").onclick=()=>togglePanel("hud","btn-hud");
if ($("close-stats")) $("close-stats").onclick=()=>togglePanel("stats","btn-stats");
if ($("close-foot")) $("close-foot").onclick=()=>togglePanel("foot","btn-foot");
if ($("live-what")) $("live-what").onclick=()=>{ if ($("foot").classList.contains("panel-hidden")) togglePanel("foot","btn-foot"); };
if ($("live-x")) $("live-x").onclick=()=>{ $("live-strip").style.display="none"; };
function placeLiveStrip(){ const nav=document.querySelector(".nav-strip"), ls=$("live-strip"); if (nav && ls) ls.style.top=(nav.getBoundingClientRect().bottom+6)+"px"; }
placeLiveStrip(); addEventListener("resize", placeLiveStrip);
// ---- index.html (body.story-first): the story is the page; every overlay sits behind one "more" control
const STORY_FIRST = document.body.classList.contains("story-first");
function sfSheet(){ const h = $("story-hud"); document.body.style.setProperty("--sheet-h", (h && !h.classList.contains("panel-hidden") ? h.offsetHeight : 0) + "px"); }
function sfStoryOn(on){ document.body.classList.toggle("story-on", on); sfSheet(); }
if (STORY_FIRST){
  const ls = $("live-strip"); if (ls) ls.remove();
  const links = [...document.querySelectorAll(".nav-strip a")].filter(a => !/^(index|globe)\.html$/.test(a.getAttribute("href") || ""))
    .map(a => `<a href="${a.getAttribute("href")}"${a.target ? ' target="_blank" rel="noopener"' : ""}>${a.textContent}</a>`).join("");
  document.body.insertAdjacentHTML("beforeend", `<div id="sf-bar">
  <button id="sf-replay" class="sf-btn" type="button" title="Play the story">▶ Story</button>
  <button id="sf-more" class="sf-btn" type="button" aria-expanded="false" aria-controls="sf-menu" title="More: controls, metrics, sources, links">⋯ More</button>
</div>
<nav id="sf-menu" class="panel-hidden" aria-label="More">
  <button type="button" data-panel="hud">Controls</button>
  <button type="button" data-panel="stats">Metrics</button>
  <button type="button" data-panel="foot">Sources: what is measured</button>
  <button type="button" id="sf-chip-tg">Show the street readout</button>
  <a href="globe.html"><b>Full explorer</b> (every panel open to use)</a>
  ${links}
</nav>`);
  const menu = $("sf-menu"), more = $("sf-more");
  const setMenu = (open) => { menu.classList.toggle("panel-hidden", !open); more.setAttribute("aria-expanded", String(open)); more.classList.toggle("open", open); };
  more.onclick = (e) => { e.stopPropagation(); setMenu(menu.classList.contains("panel-hidden")); };
  menu.querySelectorAll("[data-panel]").forEach(b => b.onclick = () => { const id = b.getAttribute("data-panel"); setMenu(false); togglePanel(id, "btn-" + id); });
  $("sf-chip-tg").onclick = () => { const on = document.body.classList.toggle("sf-chip"); $("sf-chip-tg").textContent = on ? "Hide the street readout" : "Show the street readout"; setMenu(false); };
  $("sf-replay").onclick = () => startStoryMode();
  addEventListener("click", (e) => { if (!menu.classList.contains("panel-hidden") && !menu.contains(e.target) && e.target !== more) setMenu(false); });
  addEventListener("keydown", (e) => { if (e.key === "Escape") setMenu(false); });
  if (window.ResizeObserver) new ResizeObserver(sfSheet).observe($("story-hud"));
  addEventListener("resize", sfSheet);
}
// evidence:scan
function updateStatusLine(buildSec){
  if (!W) return;
  const P = currentPop, D = Math.round(P * DEVICES_PER_PERSON);
  const M = Math.max(500, Math.round(P / 100000));
  const timing = buildSec != null ? " · " + fig("browser-kernel-time", "built in " + buildSec.toFixed(1) + " s") : "";
  const engineTxt = wasm ? "" : " · JS engine";
  $("status").innerHTML = `${fig("world-population", fmt(P) + " people · " + fmt(D) + " devices")} (${fig("cohort-world", fmt(W.n) + " cohorts on " + W.nCells + " land cells, " + W.nLocs + " localities, " + REG.length + " regions")}) · ${fig("seat-count-rule", fmt(M) + " serve seats")}${timing}${engineTxt}`;
  if ($("badge-scale")) $("badge-scale").textContent=fmt(P);
}
// evidence:end
function selectSize(pop){
  currentPop=pop; if ($("size")) $("size").value=String(pop);
  document.querySelectorAll("#size-seg button").forEach(b=>b.classList.toggle("active",+b.getAttribute("data-val")===pop));
  computeSustainability();
  updateStatusLine(null);
}
function selectActs(a){
  currentActs=a; if ($("acts")) $("acts").value=String(a);
  document.querySelectorAll("#acts-seg button").forEach(b=>b.classList.toggle("active",+b.getAttribute("data-val")===a));
}
document.querySelectorAll("#size-seg button").forEach(b=>b.onclick=()=>selectSize(+b.getAttribute("data-val")));
document.querySelectorAll("#acts-seg button").forEach(b=>b.onclick=()=>selectActs(+b.getAttribute("data-val")));
document.querySelectorAll("#mesh-seg button").forEach(b=>b.onclick=()=>{
  meshFilter=+b.getAttribute("data-tier");
  document.querySelectorAll("#mesh-seg button").forEach(x=>x.classList.toggle("active",+x.getAttribute("data-tier")===meshFilter));
  arcsNeedUpload=true; pruneAndPackArcs(getTime());
});
let currentHolonRes = "R3";
document.querySelectorAll("#holon-res-seg button").forEach(b=>{
  b.onclick=()=>{
    currentHolonRes = b.getAttribute("data-res");
    document.querySelectorAll("#holon-res-seg button").forEach(x=>x.classList.toggle("active",x.getAttribute("data-res")===currentHolonRes));
    if (currentHolonRes === "R4") selectSize(100000000);
    else if (currentHolonRes === "R3") selectSize(1000000000);
    else if (currentHolonRes === "R0") selectSize(5000000000);
    if (W) tick();
  };
});

// ------------------------------------------------------------------ Population Opt-Ins, Device Tiers & Sustainability (CC 4.5.13 / 5.4.3 / 5.4.6 / Tiers.lean)
const ADV_PRESETS = {
  optimistic:  { srv:12, home:65, wifi:75, chg:45, nat:35, sh_srv:98, sh_home:50, sh_phone:25, relay:95, ann:90, anon:15, orel:92, trc:20 },
  expected:    { srv:8,  home:55, wifi:65, chg:35, nat:60, sh_srv:95, sh_home:25, sh_phone:10, relay:90, ann:85, anon:12, orel:88, trc:15 },
  fleet:       { srv:8,  home:55, wifi:65, chg:35, nat:60, sh_srv:95, sh_home:25, sh_phone:10, relay:90, ann:85, anon:12, orel:88, trc:15 },
  pessimistic: { srv:4,  home:40, wifi:45, chg:25, nat:85, sh_srv:90, sh_home:20, sh_phone:5,  relay:50, ann:70, anon:8,  orel:60, trc:10 },
  cgnat:       { srv:4,  home:40, wifi:45, chg:25, nat:85, sh_srv:90, sh_home:20, sh_phone:5,  relay:50, ann:70, anon:8,  orel:60, trc:10 },
  adversarial: { srv:2,  home:30, wifi:30, chg:10, nat:99, sh_srv:80, sh_home:10, sh_phone:0,  relay:0,  ann:50, anon:4,  orel:20, trc:5  },
  privacy:     { srv:8,  home:60, wifi:75, chg:45, nat:55, sh_srv:100,sh_home:75, sh_phone:40, relay:95, ann:80, anon:36, orel:96, trc:5  },
  unowned:     { srv:5,  home:55, wifi:65, chg:35, nat:60, sh_srv:95, sh_home:25, sh_phone:10, relay:0,  ann:85, anon:12, orel:88, trc:15 }
};
function computeSustainability(){
  let srvPct = +$("sl_srv").value;
  let homePct = +$("sl_home").value;
  if (srvPct + homePct > 96){ homePct = 96 - srvPct; $("sl_home").value = String(homePct); }
  const phonePct = Math.max(0, 100 - srvPct - homePct);
  const wifiPct = +$("sl_wifi").value;
  const chgPct = +$("sl_chg").value;
  const natPct = +$("sl_nat").value;
  const shSrv = +$("sl_sh_srv").value / 100;
  const shHome = +$("sl_sh_home").value / 100;
  const shPhone = +$("sl_sh_phone").value / 100;
  const relayOpt = +$("sl_relay").value / 100;
  const annOpt = +$("sl_ann").value / 100;
  const anonOpt = +$("sl_anon").value / 100;
  const orelOpt = +$("sl_orel").value / 100;
  const trcOpt = +$("sl_trc").value / 100;

  $("v_srv").textContent = srvPct + "%";
  $("v_home").textContent = homePct + "%";
  $("v_phone").textContent = phonePct + "%";
  $("v_wifi").textContent = wifiPct + "% Wi-Fi · " + (100 - wifiPct) + "% cell";
  $("v_chg").textContent = chgPct + "% of Wi-Fi";
  $("v_nat").textContent = natPct + "% NAT · " + (100 - natPct) + "% direct";
  $("v_direct_nat").textContent = (100 - natPct) + "%";
  $("v_sh_srv").textContent = Math.round(shSrv*100) + "%";
  $("v_sh_home").textContent = Math.round(shHome*100) + "%";
  $("v_sh_phone").textContent = Math.round(shPhone*100) + "%";
  $("v_relay").textContent = Math.round(relayOpt*100) + "%";
  $("v_ann").textContent = Math.round(annOpt*100) + "%";
  $("v_anon").textContent = Math.round(anonOpt*100) + "%";
  $("v_orel").textContent = Math.round(orelOpt*100) + "%";
  $("v_trc").textContent = Math.round(trcOpt*100) + "%";

  const P = currentPop;
  const N = Math.round(P * DEVICES_PER_PERSON); // total devices across all people's self rosters
  const M = Math.max(500, Math.round(P / 100000)); // canonical serve seats (50,000 at 5B people)
  const fSrv = srvPct / 100, fHome = homePct / 100, fPhone = phonePct / 100;
  const fWifi = wifiPct / 100, fChg = chgPct / 100, fNat = natPct / 100;
  const reachHomeShare = Math.max(0, 1 - fNat); // Direct IPv6 or punched UDP path share (leviculum#100 / leviculum#101)

  const nSrv = N * fSrv, nHome = N * fHome, nPhone = N * fPhone;
  const nPhoneWifi = nPhone * fWifi, nPhoneChg = nPhoneWifi * fChg;

  // 1. Peers without a direct path (home + phone behind NAT without IPv6/punched path; servers & canonical seats are directly reachable)
  const nHardNat = (nHome + nPhone) * fNat;
  // Directly reachable relay parents (canonical seats + announced relay-offering servers, reachable homes, and reachable charging Wi-Fi phones)
  const reachSrvRelays = nSrv * annOpt * relayOpt;
  const reachHomeRelays = nHome * reachHomeShare * annOpt * relayOpt;
  const reachPhoneRelays = nPhoneChg * reachHomeShare * annOpt * relayOpt;
  const maxHardNatLeaves = Math.round(M * 36 + reachSrvRelays * 32 + reachHomeRelays * 8 + reachPhoneRelays * 2);
  const natLoadPct = maxHardNatLeaves > 0 ? (nHardNat / maxHardNatLeaves) * 100 : 999;

  // 2. 1-Hop Shaped slot supply vs demand (CC 5.4.3 "A shaped link is one hop" + Tiers.lean: each_home_adds_seven & reach ≠ capacity)
  // Only directly reachable nodes (`reachSrvRelays`, `reachHomeRelays`, `reachPhoneRelays`) can supply shaped parent slots.
  const shapedSrvRelays = reachSrvRelays * Math.max(shSrv, orelOpt * 0.5);
  const shapedHomeRelays = reachHomeRelays * Math.max(shHome, orelOpt * 0.5);
  const shapedPhoneRelays = reachPhoneRelays * Math.max(shPhone, orelOpt * 0.3);
  const shapedSlotsSupply = Math.round(M * 8 + shapedSrvRelays * 7 + shapedHomeRelays * 7 + shapedPhoneRelays * 3);

  // CC 5.4.3: a peer with no direct path gets no shaped link and carries the same classes on ordinary links.
  const nShapingNodes = Math.min(N, Math.round(
    nSrv * Math.max(shSrv, annOpt * relayOpt * orelOpt) +
    (nHome * reachHomeShare) * Math.max(shHome, annOpt * relayOpt * orelOpt * 0.5, anonOpt) +
    (nPhone * reachHomeShare) * (fWifi * Math.max(shPhone, fChg * annOpt * relayOpt * orelOpt * 0.35, anonOpt) + (1 - fWifi) * Math.max(shPhone * 0.4, anonOpt * 0.5))
  ));
  const shapedSlotsDemand = Math.max(1, Math.round(nShapingNodes * 1.8));
  const shapedSeatedFrac = Math.min(1.0, shapedSlotsSupply / shapedSlotsDemand);

  // 3. Onion cover pool & answering onion relays per layer (CC 5.4.6 floor >= 3 per layer)
  const nOnionPeers = Math.round((nSrv + (nHome + nPhone) * reachHomeShare) * anonOpt);
  const answeringOnionRelays = Math.round(M + (reachSrvRelays + reachHomeRelays + reachPhoneRelays) * orelOpt);
  const relaysPerLayer = Math.floor(answeringOnionRelays / 3);
  const coverToOnionRatio = nOnionPeers > 0 ? (nShapingNodes / nOnionPeers) : nShapingNodes;

  // Update simulation per-mille parameters for WASM & JS engines
  const effectiveDirectFrac = fSrv + (1 - fSrv) * reachHomeShare;
  simParams.directNatPm = Math.round(Math.max(20, Math.min(980, effectiveDirectFrac * 1000)));
  const rawShapeShare = (nShapingNodes / Math.max(1, N * effectiveDirectFrac)) * shapedSeatedFrac;
  simParams.shapingPm = Math.round(Math.max(40, Math.min(980, (0.25 + 0.75 * rawShapeShare) * 1000)));
  simParams.onionPm = relaysPerLayer >= 3 ? Math.round(Math.max(0, Math.min(600, anonOpt * 1000))) : 0;

  // Update UI pills & #stats
  const natOk = nHardNat <= maxHardNatLeaves;
  $("pill-nat").textContent = Math.round(reachHomeShare * 100) + "% reach · " + fmt(nHardNat) + "/" + fmt(maxHardNatLeaves) + (natOk ? " ✓" : " ⚠");
  $("pill-nat").style.color = natOk ? (natLoadPct > 80 ? "var(--warm)" : "var(--ok)") : "var(--hot)";

  const shapePct = Math.round(shapedSeatedFrac * 100);
  $("pill-shape").textContent = fmt(shapedSlotsSupply) + " slots (" + shapePct + "% seated)";
  $("pill-shape").style.color = shapePct >= 95 ? "var(--ok)" : shapePct >= 60 ? "var(--warm)" : "var(--hot)";

  $("pill-onion").textContent = (nOnionPeers > 0 ? coverToOnionRatio.toFixed(1) + "× cover" : "0 onion") + " · " + fmt(relaysPerLayer) + "/lyr";
  $("pill-onion").style.color = relaysPerLayer >= 10 && coverToOnionRatio >= 1.5 ? "var(--ok)" : relaysPerLayer >= 3 ? "var(--warm)" : "var(--hot)";

  const holonOk = relayOpt > 0 && reachHomeShare >= 0.15;
  $("pill-holon").textContent = holonOk ? "in budget · β=0.030" : (relayOpt === 0 ? "over · stranded" : "over · CGNAT");
  $("pill-holon").style.color = holonOk ? "var(--ok)" : "var(--hot)";

  $("s_nat_cap").textContent = fmt(nHardNat) + " / " + fmt(maxHardNatLeaves) + " (" + Math.min(999, natLoadPct).toFixed(0) + "% load)";
  $("s_nat_cap").style.color = natOk ? "#fff" : "var(--hot)";
  $("natbar").style.width = Math.min(100, natLoadPct).toFixed(1) + "%";
  $("natbar").style.background = natOk ? (natLoadPct > 80 ? "var(--warm)" : "var(--ok)") : "var(--hot)";
  $("s_nat_max").textContent = (reachHomeShare * 100).toFixed(0) + "% direct IPv6 / punched  (" + fmt(reachHomeRelays) + " home parents)";
  $("s_shape_slots").textContent = fmt(shapedSlotsSupply) + " supply vs " + fmt(shapedSlotsDemand) + " demand (" + shapePct + "% seated" + (relayOpt === 0 ? " · unowned-only!" : "") + ")";
  $("s_shapers").textContent = fmt(nShapingNodes) + " direct 1-hop devices (" + (nShapingNodes / N * 100).toFixed(1) + "% of fleet)";
  $("s_cover_ratio").textContent = nOnionPeers > 0 ? coverToOnionRatio.toFixed(2) + " shaping devices per onion peer (" + fmt(nOnionPeers) + " onion)" : fmt(nShapingNodes) + " shapers / 0 onion peers";
  $("s_onion_relays").textContent = fmt(relaysPerLayer) + " / layer  (" + (relaysPerLayer >= 10 ? "✓ healthy ≥10" : relaysPerLayer >= 3 ? "⚠ small set <10" : "✗ below CC 5.4.6 floor <3") + ")";
}
["sl_srv","sl_home","sl_wifi","sl_chg","sl_nat","sl_sh_srv","sl_sh_home","sl_sh_phone","sl_relay","sl_ann","sl_anon","sl_orel","sl_trc"].forEach(id=>{
  const el=$(id); if(el) el.addEventListener("input", computeSustainability);
});
document.querySelectorAll("#preset-bar button").forEach(btn=>{
  btn.addEventListener("click", ()=>{
    const p = ADV_PRESETS[btn.getAttribute("data-preset")];
    if (!p) return;
    $("sl_srv").value=p.srv; $("sl_home").value=p.home; $("sl_wifi").value=p.wifi; $("sl_chg").value=p.chg; $("sl_nat").value=p.nat;
    $("sl_sh_srv").value=p.sh_srv; $("sl_sh_home").value=p.sh_home; $("sl_sh_phone").value=p.sh_phone;
    $("sl_relay").value=p.relay; $("sl_ann").value=p.ann; $("sl_anon").value=p.anon; $("sl_orel").value=p.orel; $("sl_trc").value=p.trc;
    computeSustainability();
    if (W) tick();
  });
});
function updateWorkloadLimits(ms,actsCount){
  if (ms>0&&actsCount>0){
    const msPerAct=ms/actsCount;
    avgMsPerAct=avgMsPerAct?(avgMsPerAct*0.6+msPerAct*0.4):msPerAct;
    avgMsPerTick=avgMsPerTick?(avgMsPerTick*0.6+ms*0.4):ms;
  }
  const MAX_TICK_BUDGET_MS=60;
  let highestSafe=2000;
  document.querySelectorAll("#acts-seg button").forEach(btn=>{
    const val=+btn.getAttribute("data-val");
    const est=val*(avgMsPerAct||0.0003);
    if (avgMsPerAct>0&&est>MAX_TICK_BUDGET_MS){
      btn.disabled=true;
      btn.title=`Limited by measured time: ~${Math.round(est)} ms/tick exceeds ${MAX_TICK_BUDGET_MS}ms real-time budget`;
    } else {
      btn.disabled=false;
      btn.title=`Estimated ~${est<1?est.toFixed(1):Math.round(est)} ms/tick`;
      highestSafe=Math.max(highestSafe,val);
    }
  });
  if (currentActs>highestSafe&&avgMsPerAct>0) selectActs(highestSafe);
  const budgetEl=$("ms-budget");
  if (budgetEl&&avgMsPerAct>0){
    const est=(currentActs*avgMsPerAct).toFixed(1);
    const label=avgMsPerTick<20?"fluid":avgMsPerTick<50?"good":"heavy";
    budgetEl.textContent=`~${est} ms / tick · ${label}`;
    budgetEl.style.color=avgMsPerTick<20?"var(--ok)":avgMsPerTick<50?"var(--warm)":"var(--hot)";
  }
}
function recordActiveBlocks(){
  if (!lastActivity) return; const a=lastActivity; const u32=new Uint32Array(a.buffer,a.byteOffset,a.byteLength>>>2);
  activeBlocks.length=0; for (let i=0;i<u32.length;i++) if (u32[i]!==0) activeBlocks.push(i);
}
function tick(){
  const acts=currentActs; const t0=performance.now(); const res=(useWasm&&wasm)?tickWasm(acts):tickJs(acts); const ms=performance.now()-t0;
  updateWorkloadLimits(ms,acts);
  const o=res.out; cum+=o[1]; const local=o[17]?o[16]/o[17]:0; $("dlocal").textContent=(o[1]?o[5]/o[1]*100:0).toFixed(1)+"%";
  const localStr=(local*100).toFixed(1)+"%";
  $("local").textContent=localStr; $("localbar").style.width=localStr;
  if ($("badge-local")) $("badge-local").textContent=localStr+" local";
  $("xmetro").textContent=(o[1]?o[6]/o[1]*100:0).toFixed(1)+"%"; $("xregion").textContent=(o[1]?o[7]/o[1]*100:0).toFixed(1)+"%";
  const crossings=o[6]+o[7]; $("km").textContent=crossings?Math.round(o[8]/crossings)+" km":"–";
  $("m_base").textContent=fmt(o[18])+" hops · "+fmtBytes(o[18]*4096);
  $("m_seat").textContent=fmt(o[19])+" hops · "+fmtBytes(o[19]*4096);
  $("m_bulk").textContent=fmt(o[20])+" hops · "+fmtBytes(o[20]*4096);
  $("m_onion").textContent=fmt(o[22])+" acts  (+0 B extra wire)";
  $("m_alm").textContent=fmt(o[21])+" direct deliveries saved · "+fmt(o[23])+" direct 1-hop";
  $("acts_n").textContent=fmt(o[0])+"  (family "+fmt(o[12])+" · room "+fmt(o[13])+" · c-free "+fmt(o[14])+" · public "+fmt(o[15])+")"; $("deliv").textContent=fmt(o[1]); $("refused").textContent=fmt(o[4]);
  const totalDevs = Math.round(currentPop * DEVICES_PER_PERSON);
  const bcastWorld = o[17] * totalDevs * 4096;
  $("bytes").textContent=fmtBytes(o[9]); $("bcast").textContent=fmtBytes(bcastWorld)+"  ("+fmt(bcastWorld/Math.max(1,o[9]))+"×)"; $("central").textContent=fmtBytes(o[11])+"  ("+fmt(o[11]/Math.max(1,o[9]))+"× vs relay tree)";
  $("dps").textContent=fmt(o[2]/(ms/1000)); $("ms").textContent=ms.toFixed(1)+" ms  ("+(useWasm&&wasm?"WASM":"JS")+")"; $("bpn").textContent=fmt(currentPop)+" people · "+fmt(totalDevs)+" devices"; $("cum").textContent=fmt(cum);

  // CIRISHolon multiscale certification telemetry (FSD §4.2 / §4.5)
  const dv2 = o[28], beta = o[29], lipK = o[30], rentG = o[31], witnesses = o[32];
  const isCertified = witnesses === 0;
  if ($("s_holon_dv2")) $("s_holon_dv2").textContent = dv2.toFixed(4) + " (budget β=" + beta.toFixed(3) + ")";
  if ($("holonbar")) {
    const pct = Math.min(100, Math.max(0, (dv2 / beta) * 100));
    $("holonbar").style.width = pct.toFixed(1) + "%";
    $("holonbar").style.background = isCertified ? "var(--ok)" : (dv2 < beta * 1.5 ? "var(--warm)" : "var(--hot)");
  }
  if ($("s_holon_k")) {
    const kOk = lipK <= 1.05;
    $("s_holon_k").textContent = lipK.toFixed(4) + (kOk ? " (under the ceiling)" : " (over the ceiling)");
    $("s_holon_k").style.color = kOk ? "var(--ok)" : "var(--hot)";
  }
  if ($("s_holon_rent")) $("s_holon_rent").textContent = rentG.toFixed(4) + " (retention rent G_∞)";
  if ($("s_holon_cert")) {
    $("s_holon_cert").textContent = isCertified ? "within budget (heuristic, not a certificate)" : "over budget (" + Math.round(witnesses) + " flagged)";
    $("s_holon_cert").style.color = isCertified ? "var(--ok)" : "var(--hot)";
  }
  if ($("pill-holon")) {
    $("pill-holon").textContent = isCertified ? "in budget · Dv²=" + dv2.toFixed(3) : "over · " + Math.round(witnesses) + " flagged";
    $("pill-holon").style.color = isCertified ? "var(--ok)" : "var(--hot)";
  }

  // cewp-sim v0.2 transport & chunk custody telemetry
  if ($("s_64k_refused")) $("s_64k_refused").textContent = fmt(o[24]) + " acts refused on shaped (unshaped ALM / bulk)";
  if ($("s_displaced")) $("s_displaced").textContent = fmt(o[25]) + " (edge v45 has no displacement, #939)";
  if ($("s_chunk_custody")) $("s_chunk_custody").textContent = (o[26]).toFixed(1) + "% Full (" + fmt(o[27]) + " partial/del)";
  if ($("s_baselines")) {
    const b1 = o[33], b2 = o[34], b3 = o[35], cewp = Math.max(1, o[9]);
    $("s_baselines").textContent = "CDN " + (b1/cewp).toFixed(1) + "× · SFU " + (b2/cewp).toFixed(1) + "× · P2P " + (b3/cewp).toFixed(1) + "×";
  }

  lastActivity=res.activity; recordActiveBlocks();
  if (!storyActive || storyLive()) {
    gl.bindBuffer(gl.ARRAY_BUFFER,actBuf); gl.bufferSubData(gl.ARRAY_BUFFER,0,lastActivity); setArcs(res.arcs);
  }
}
function decay(){
  // a street-world build between ticks can grow wasm memory, which detaches the last tick's view
  if (lastActivity && lastActivity.length === 0 && wasm && bufs && W && W.n > 0) lastActivity = new Uint8Array(mem.buffer, bufs.activity, W.n);
  if (!lastActivity||activeBlocks.length===0) return; const a=lastActivity; const u32=new Uint32Array(a.buffer,a.byteOffset,a.byteLength>>>2);
  let w=0; for (let idx=0;idx<activeBlocks.length;idx++){
    const i=activeBlocks[idx]; if (u32[i]===0) continue;
    const base=i<<2; let remaining=false;
    for (let j=0;j<4;j++){ const v=a[base+j]; if (v>0){ const nv=v>3?(v*0.92|0):0; a[base+j]=nv; if (nv>0) remaining=true; } }
    if (remaining) activeBlocks[w++]=i;
  }
  activeBlocks.length=w; gl.bindBuffer(gl.ARRAY_BUFFER,actBuf); gl.bufferSubData(gl.ARRAY_BUFFER,0,a);
}

// ------------------------------------------------------------------ Animated Multi-Scale CIRISHolon Story Engine
const SEED_NODES = [
  { lat: 47.3769, lon: 8.5417, name: "alice-home" },     // 0: Zurich
  { lat: 46.2044, lon: 6.1432, name: "bob-nas" },        // 1: Geneva
  { lat: 45.4642, lon: 9.1900, name: "alice-phone" },    // 2: Milan
  { lat: 48.1351, lon: 11.5820, name: "alice-laptop" },  // 3: Munich
  { lat: 50.1109, lon: 8.6821, name: "bob-phone" },      // 4: Frankfurt
  { lat: 45.7640, lon: 4.8357, name: "carol-home" },     // 5: Lyon
  { lat: 48.8566, lon: 2.3522, name: "dave-desktop" },   // 6: Paris
  { lat: 48.2082, lon: 16.3738, name: "erin-server" },   // 7: Vienna
  { lat: 52.5200, lon: 13.4050, name: "frank-phone" },   // 8: Berlin
  { lat: 52.3676, lon: 4.9041, name: "grace-laptop" },   // 9: Amsterdam
  { lat: 41.9028, lon: 12.4964, name: "heidi-home" },    // 10: Rome
  { lat: 51.5074, lon: -0.1278, name: "ivan-phone" }     // 11: London
];
const REGIONAL_CLUSTER = [
  { lat: 47.5596, lon: 7.5886 }, { lat: 46.9480, lon: 7.4474 }, { lat: 46.2044, lon: 6.1432 },
  { lat: 46.5197, lon: 6.6323 }, { lat: 47.0502, lon: 8.3093 }, { lat: 47.4245, lon: 9.3767 },
  { lat: 46.0037, lon: 8.9511 }, { lat: 47.9990, lon: 7.8421 }, { lat: 48.7758, lon: 9.1829 },
  { lat: 48.1351, lon: 11.5820 }, { lat: 48.5734, lon: 7.7521 }, { lat: 47.2692, lon: 11.4041 },
  { lat: 45.4642, lon: 9.1900 }, { lat: 45.0703, lon: 7.6869 }, { lat: 45.7640, lon: 4.8357 },
  { lat: 48.8566, lon: 2.3522 }, { lat: 50.1109, lon: 8.6821 }, { lat: 51.5074, lon: -0.1278 },
  { lat: 52.5200, lon: 13.4050 }, { lat: 52.3676, lon: 4.9041 }, { lat: 50.8503, lon: 4.3517 },
  { lat: 48.2082, lon: 16.3738 }, { lat: 47.4979, lon: 19.0402 }, { lat: 50.0755, lon: 14.4378 },
  { lat: 41.9028, lon: 12.4964 }, { lat: 40.4168, lon: -3.7038 }, { lat: 41.3851, lon: 2.1734 },
  { lat: 52.2297, lon: 21.0122 }
];

// the story opens on where its numbers come from and closes on how to run a node; STORY_LEAD
// is the count of steps before First Contact, so the act code below keeps its own numbering
const STORY_LEAD = 1;
const STORY_ACTS = [
  {
    num: 1,
    title: "A digital twin",
    intro: true,
    live: true,
    camDist: 2.65,
    targetZoom: 1.08,
    yaw: 8.54 * Math.PI / 180,
    pitch: 34.0 * Math.PI / 180,
    duration: 20
  },
  {
    num: 2,
    title: "Two neighbours connect",
    micro: true, // street scale: the R5 focal world (cewp_sim::micro)
    camDist: 1.35,
    targetZoom: 2.25,
    yaw: 8.54 * Math.PI / 180,
    pitch: 47.37 * Math.PI / 180,
    pop: 100,
    holonRes: "R4",
    duration: 8 // re-planned from its beats once the street is built (storyMicroPlan)
  },
  {
    num: 3,
    title: "A whole street",
    micro: true,
    camDist: 1.35,
    targetZoom: 2.25,
    yaw: 8.54 * Math.PI / 180,
    pitch: 47.37 * Math.PI / 180,
    pop: 5000,
    holonRes: "R4",
    duration: 8
  },
  {
    num: 4,
    title: "A local community",
    camDist: 1.75,
    targetZoom: 1.78,
    yaw: 9.5 * Math.PI / 180,
    pitch: 47.5 * Math.PI / 180,
    pop: 5000,
    holonRes: "R4",
    duration: 14
  },
  {
    num: 5,
    title: "Cities connect",
    live: true,
    camDist: 2.25,
    targetZoom: 1.36,
    yaw: 12.0 * Math.PI / 180,
    pitch: 41.0 * Math.PI / 180,
    pop: 250000,
    holonRes: "R3",
    duration: 14
  },
  {
    num: 6,
    title: "Across the oceans",
    live: true,
    camDist: 2.65,
    targetZoom: 1.08,
    yaw: -16.0 * Math.PI / 180,
    pitch: 26.0 * Math.PI / 180,
    pop: 10000000,
    holonRes: "R1",
    duration: 14
  },
  {
    num: 7,
    title: "The whole world",
    live: true,
    camDist: 3.10,
    targetZoom: 0.96,
    yaw: 0.0,
    pitch: 16.0 * Math.PI / 180,
    pop: 1000000000,
    holonRes: "R0",
    duration: 16
  },
  {
    num: 8,
    title: "Join in",
    outro: true,
    live: true,
    camDist: 3.10,
    targetZoom: 0.96,
    yaw: 0.0,
    pitch: 16.0 * Math.PI / 180,
    duration: 30
  }
];

let storyActivity = null;
let storyRingSample = Array.from({length: 180}, () => []);
let storyCommunityNodes = [];
let storyLitSeeds = 0;
let storyLitComm = 0;

function seedStoryNodes(){
  if (!W || W.n < 12) return;
  for (let i=0; i<SEED_NODES.length && i<W.n; i++){
    W.lat[i] = SEED_NODES[i].lat;
    W.lon[i] = SEED_NODES[i].lon;
    W.isSeat[i] = 1;
  }
  for (let i=0; i<REGIONAL_CLUSTER.length && (12+i)<W.n; i++){
    W.lat[12+i] = REGIONAL_CLUSTER[i].lat;
    W.lon[12+i] = REGIONAL_CLUSTER[i].lon;
    if (i % 2 === 0) W.isSeat[12+i] = 1;
  }
  storyActivity = new Uint8Array(W.n);
  storyRingSample = Array.from({length: 180}, () => []);
  const commCandidates = [];
  const z0 = seedOriginXyz;
  for (let i=12; i<W.n; i++){
    const v = xyz(W.lat[i], W.lon[i]);
    const dot = Math.max(-1, Math.min(1, v[0]*z0[0] + v[1]*z0[1] + v[2]*z0[2]));
    const angDeg = Math.acos(dot) * 180 / Math.PI;
    const deg = Math.min(179, Math.max(1, Math.floor(angDeg)));
    if (storyRingSample[deg].length < 48) storyRingSample[deg].push(i);
    if (i < 40) {
      commCandidates.push({ idx: i, ang: angDeg * 0.25 });
    } else if (angDeg <= 12.5 && commCandidates.length < 520) {
      commCandidates.push({ idx: i, ang: angDeg });
    }
  }
  commCandidates.sort((a, b) => a.ang - b.ang);
  storyCommunityNodes = commCandidates.slice(0, 488).map(c => c.idx);
  uploadPoints();
}

function pickRingNode(minDeg, maxDeg) {
  const lo = Math.max(1, Math.min(179, Math.floor(minDeg)));
  const hi = Math.max(lo, Math.min(179, Math.floor(maxDeg)));
  for (let tries = 0; tries < 8; tries++) {
    const d = lo + Math.floor(Math.random() * (hi - lo + 1));
    const ring = storyRingSample[d];
    if (ring && ring.length > 0) return ring[Math.floor(Math.random() * ring.length)];
  }
  return Math.floor(Math.random() * Math.min(W ? W.n : 12, 40));
}

function storyLive(){ const c = STORY_ACTS[storyCurrentAct]; return !!(c && c.live); }
// evidence:scan
const STORY_CAPTIONS = [
  [ `<b>This is a digital twin of the CIRIS mesh</b>: a simulation whose <span class="fig" data-claim="seat1-windows">serve-seat costs are calibrated to measurements of the real network</span>. <span class="fig" data-claim="all-simulated">Everyone and everything you'll see is simulated.</span> <a href="#" data-story-sources>How we know</a>` ],
  [ "Two neighbours connect directly, with no company in the middle" ],
  [ "A whole street of phones, laptops and home servers" ],
  [
    "Neighbours form a local community and agree on their own rules together",
    "Members keep backup copies of each other's shared photos and files"
  ],
  [
    "Communities in different cities connect, passing live video from member to member",
    `<span class="fig" data-claim="shaping-scope">Traffic on shaped links is sent at a steady pace, so private conversations blend in with everything else</span>`
  ],
  [
    "Regions connect across oceans, while local disagreements stay local",
    "Every community runs itself. No single company is in charge"
  ],
  [
    `<span class="fig" data-claim="world-population">A billion people</span> connected, and <span class="fig" data-claim="kernel-traffic">each person uses only a little data</span>`,
    "Activity rises and falls with day and night around the world"
  ],
  [ `<b>Want to join?</b> Install the CIRIS agent: <code>pip install ciris-agent</code>, then run <code>ciris-agent</code>. <a href="https://ciris.ai/install" target="_blank" rel="noopener">Install guide</a> · <a href="https://pypi.org/project/ciris-agent/" target="_blank" rel="noopener">PyPI</a>. <span class="fig" data-claim="all-simulated">This globe shows a model, not real people.</span>` ]
];
// evidence:end

// pacing: a caption stays on screen for its length, ~250 ms a word, never under 3 s or over 7 s
const capWords = (html) => html.replace(/<[^>]+>/g, " ").trim().split(/\s+/).filter(Boolean).length;
const capSecs = (html) => Math.min(7, Math.max(3, 0.25 * capWords(html)));
// a planetary step lasts as long as its captions; the street steps set their own length beat by beat
STORY_ACTS.forEach((cfg, i) => { if (cfg.micro) return; cfg.capT = (STORY_CAPTIONS[i] || [cfg.title]).map(capSecs); cfg.duration = cfg.capT.reduce((a, b) => a + b, 0); });
function storySubIx(cfg, t){ if (!cfg.capT) return 0; let acc = 0; for (let i = 0; i < cfg.capT.length; i++){ acc += cfg.capT[i]; if (t < acc) return i; } return cfg.capT.length - 1; }

function updateStoryUI(){
  const cfg = STORY_ACTS[storyCurrentAct];
  if ($("story-act-badge")) $("story-act-badge").textContent = `${cfg.num}/${STORY_ACTS.length}`;
  if ($("story-title")) $("story-title").textContent = cfg.title;
  if ($("story-line") && !cfg.micro) {
    const lines = STORY_CAPTIONS[storyCurrentAct] || [cfg.title];
    $("story-line").innerHTML = lines[Math.min(lines.length - 1, storySubIx(cfg, storyActElapsed))];
  }
  if ($("story-btn-play")) {
    const end = storyCurrentAct === STORY_ACTS.length - 1 && storyActElapsed >= cfg.duration;
    $("story-btn-play").textContent = end ? "↻" : storyPaused ? "▶" : "❚❚";
    $("story-btn-play").title = end ? "Play the story again" : "Pause / Resume";
  }
  if ($("story-progress-bar")) {
    const pct = Math.min(100, (storyActElapsed / cfg.duration) * 100);
    $("story-progress-bar").style.width = pct.toFixed(1) + "%";
  }
}

function storySetupAct(actIx){
  storyCurrentAct = actIx;
  storyActElapsed = 0;
  storyUserCam = false;
  const cfg = STORY_ACTS[actIx];
  storyTargetCamDist = cfg.camDist;
  storyTargetZoom = cfg.targetZoom;
  storyTargetYaw = cfg.yaw;
  storyTargetPitch = cfg.pitch;

  if (cfg.pop) selectSize(cfg.pop);
  if (cfg.holonRes) {
    currentHolonRes = cfg.holonRes;
    document.querySelectorAll("#holon-res-seg button").forEach(b => {
      b.classList.toggle("active", b.getAttribute("data-res") === currentHolonRes);
    });
  }

  if (W) tick(); // refresh telemetry panels without overwriting story globe buffers
  if (!storyActivity || (W && storyActivity.length !== W.n)) {
    if (W) seedStoryNodes();
  }

  const nowSec = getTime();
  const o = actIx - 1 - STORY_LEAD; // the planetary acts keep their original numbering (0 was First Contact, now told at street scale)
  if (cfg.intro || cfg.micro){ geoLoad50(); geoLoadPlaces(); } // the street steps stand on the tiling
  if (cfg.micro) storyMicroSetup(actIx - STORY_LEAD); else if (microStory) storyMicroEnd();
  if (cfg.intro || cfg.outro) {
    // the whole planet at its ordinary look: the opening and closing steps tell, they do not stage
    uStoryCosVal = -1.05;
    uStoryDensityVal = 1.0;
  } else if (cfg.micro) {
    // the globe is silent while the camera is on the street: 0 lit cohorts, 0 arcs
    uStoryCosVal = 2.0;
    uStoryDensityVal = 0.0;
    storyLitSeeds = 0;
    storyLitComm = 0;
    activeArcs.length = 0;
    arcCount = 0;
    arcsNeedUpload = true;
    if (storyActivity) {
      storyActivity.fill(0);
      gl.bindBuffer(gl.ARRAY_BUFFER, actBuf);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, storyActivity);
    }
    pruneAndPackArcs(nowSec);
  } else if (o === 1) {
    uStoryCosVal = 2.0;
    uStoryDensityVal = 0.0;
    storyLitSeeds = 12;
    storyLitComm = 0;
    if (storyActivity) {
      storyActivity.fill(0);
      for (let i = 0; i < 12 && i < storyActivity.length; i++) storyActivity[i] = 160;
      gl.bindBuffer(gl.ARRAY_BUFFER, actBuf);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, storyActivity);
    }
  } else if (o === 2) {
    uStoryCosVal = Math.cos(10 * Math.PI / 180);
    uStoryDensityVal = 0.25;
  } else if (o === 3) {
    uStoryCosVal = Math.cos(48 * Math.PI / 180);
    uStoryDensityVal = 0.55;
  } else if (o === 4) {
    uStoryCosVal = Math.cos(118 * Math.PI / 180);
    uStoryDensityVal = 0.85;
  }

  updateStoryUI();
}

function storyTickAct(actIx, dt) {
  if (!W || !storyActivity) return;
  const nowSec = getTime();
  const cfg = STORY_ACTS[actIx];
  if (cfg.intro || cfg.outro) return;
  if (cfg.micro) { storyMicroTick(actIx - STORY_LEAD, storyActElapsed); return; }
  actIx = actIx - 1 - STORY_LEAD; // original planetary numbering below

  if (actIx === 0) {
    // 0.0s..0.8s: silent globe; 0.8s..2.1s: first 2 nodes spark & connect; 2.1s..12.5s: 2 -> 12 nodes light up one by one
    uStoryCosVal = 2.0;
    uStoryDensityVal = 0.0;
    let targetSeeds = 0;
    if (storyActElapsed >= 0.8 && storyActElapsed < 2.1) {
      targetSeeds = 2;
    } else if (storyActElapsed >= 2.1) {
      targetSeeds = Math.min(12, 2 + Math.floor((storyActElapsed - 2.1) / 0.95));
    }
    for (let i = 0; i < storyLitSeeds && i < W.n; i++) {
      if (storyActivity[i] > 14) storyActivity[i] = Math.max(14, (storyActivity[i] * 0.91) | 0);
    }
    while (storyLitSeeds < targetSeeds && storyLitSeeds < W.n) {
      const idx = storyLitSeeds++;
      storyActivity[idx] = 255;
      if (idx === 1) {
        storyActivity[0] = 255;
        addArc(0, 1 | (1 << 28), nowSec);
        addArc(1, 0 | (1 << 28), nowSec);
        arcsNeedUpload = true;
      } else if (idx >= 2) {
        const p1 = (idx - 1) % idx;
        const p2 = (idx * 3) % idx;
        storyActivity[p1] = 255;
        addArc(p1, idx | (1 << 28), nowSec);
        if (p2 !== p1) { storyActivity[p2] = 220; addArc(p2, idx | (2 << 28), nowSec); }
        arcsNeedUpload = true;
      }
    }
    if (storyLitSeeds >= 2 && Math.floor(nowSec * 2.4) !== Math.floor((nowSec - dt) * 2.4)) {
      const a = Math.floor(Math.random() * storyLitSeeds);
      const b = (a + 1 + Math.floor(Math.random() * Math.max(1, storyLitSeeds - 1))) % storyLitSeeds;
      if (a !== b) {
        storyActivity[a] = 255;
        storyActivity[b] = 255;
        addArc(a, b | ((1 + (a % 2)) << 28), nowSec);
        arcsNeedUpload = true;
      }
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, actBuf);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, storyActivity.subarray(0, Math.min(W.n, 16)));
  } else if (actIx === 1) {
    // 12 -> 500 community nodes wake up radially across Europe and flash as arcs connect them
    uStoryCosVal = 2.0;
    uStoryDensityVal = 0.0;
    for (let i = 0; i < 12 && i < W.n; i++) {
      if (storyActivity[i] > 14) storyActivity[i] = Math.max(14, (storyActivity[i] * 0.91) | 0);
    }
    for (let i = 0; i < storyLitComm; i++) {
      const ni = storyCommunityNodes[i];
      if (storyActivity[ni] > 12) storyActivity[ni] = Math.max(12, (storyActivity[ni] * 0.90) | 0);
    }
    const progress = Math.min(1.0, storyActElapsed / (0.85 * cfg.duration));
    const targetComm = Math.min(storyCommunityNodes.length, Math.floor(progress * storyCommunityNodes.length));
    while (storyLitComm < targetComm) {
      const nodeIdx = storyCommunityNodes[storyLitComm++];
      storyActivity[nodeIdx] = 255;
      if (storyLitComm % 10 === 0) {
        const src = (storyLitComm < 40) ? (storyLitComm % 12) : storyCommunityNodes[Math.floor(Math.random() * (storyLitComm - 1))];
        storyActivity[src] = 255;
        addArc(src, nodeIdx | (((storyLitComm % 3 === 0) ? 1 : 2) << 28), nowSec);
        arcsNeedUpload = true;
      }
    }
    if (storyLitComm > 4 && Math.floor(nowSec * 4.5) !== Math.floor((nowSec - dt) * 4.5)) {
      const src = Math.random() < 0.45 ? Math.floor(Math.random() * 12) : storyCommunityNodes[Math.floor(Math.random() * storyLitComm)];
      const dst = storyCommunityNodes[Math.floor(Math.random() * storyLitComm)];
      if (src !== dst) {
        storyActivity[src] = 255;
        storyActivity[dst] = 255;
        addArc(src, dst | (2 << 28), nowSec);
        arcsNeedUpload = true;
      }
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, actBuf);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, storyActivity);
  } else if (actIx === 2) {
    // Wavefront expands 10 deg -> 48 deg; live tick() + decay() flashes traffic inside the wave
    const progress = Math.min(1.0, storyActElapsed / (0.85 * cfg.duration));
    const waveDeg = 10 + progress * 38;
    uStoryCosVal = Math.cos(waveDeg * Math.PI / 180);
    uStoryDensityVal = 0.25 + progress * 0.30;
  } else if (actIx === 3) {
    // Wavefront expands 48 deg -> 118 deg across oceans; live tick() + decay() flashes diurnal traffic
    const progress = Math.min(1.0, storyActElapsed / (0.85 * cfg.duration));
    const waveDeg = 48 + progress * 70;
    uStoryCosVal = Math.cos(waveDeg * Math.PI / 180);
    uStoryDensityVal = 0.55 + progress * 0.30;
  } else if (actIx === 4) {
    // Wavefront completes 118 deg -> 180 deg -> 100% planetary diurnal network
    const progress = Math.min(1.0, storyActElapsed / (0.6 * cfg.duration));
    const waveDeg = 118 + progress * 62;
    uStoryCosVal = progress >= 1.0 ? -1.05 : Math.cos(waveDeg * Math.PI / 180);
    uStoryDensityVal = 0.85 + progress * 0.15;
  }
}

function startStoryMode(){
  storyActive = true;
  storyPaused = false;
  if ($("story-hud")) $("story-hud").classList.remove("panel-hidden");
  sfStoryOn(true);
  document.querySelectorAll(".nav-story-btn, .fab-story, .btn-story-main").forEach(b => b.classList.add("active"));
  if (window.innerWidth <= 800) {
    ["hud", "stats", "foot"].forEach(p => {
      const el = $(p); if (el) el.classList.add("panel-hidden");
      const btn = $("btn-" + p); if (btn) btn.classList.remove("open");
    });
  }
  storySetupAct(0);
}

function exitStoryMode(){
  storyActive = false;
  storyMicroEnd();
  uStoryCosVal = -1.05;
  uStoryDensityVal = 1.0;
  if ($("story-hud")) $("story-hud").classList.add("panel-hidden");
  sfStoryOn(false);
  document.querySelectorAll(".nav-story-btn, .fab-story, .btn-story-main").forEach(b => b.classList.remove("active"));
  targetZoom = 0.96;
  storyTargetCamDist = 3.10;
  selectSize(5000000000);
  lastTouchTime = performance.now() - 4000; // immediately continue smooth planetary rotation
  if (W) tick();
}

function initStoryMode(){
  if ($("btn-story-nav")) $("btn-story-nav").onclick = startStoryMode;
  if ($("btn-story-fab")) $("btn-story-fab").onclick = startStoryMode;
  if ($("btn-story-hud")) $("btn-story-hud").onclick = startStoryMode;
  if ($("btn-story-hud-bottom")) $("btn-story-hud-bottom").onclick = startStoryMode;
  if ($("story-btn-prev")) $("story-btn-prev").onclick = () => { storyPaused = false; storySetupAct(Math.max(0, storyCurrentAct - 1)); };
  if ($("story-btn-next")) $("story-btn-next").onclick = () => {
    storyPaused = false;
    if (storyCurrentAct < STORY_ACTS.length - 1) storySetupAct(storyCurrentAct + 1);
    else exitStoryMode();
  };
  if ($("story-btn-play")) $("story-btn-play").onclick = () => {
    const end = storyCurrentAct === STORY_ACTS.length - 1 && storyActElapsed >= STORY_ACTS[storyCurrentAct].duration;
    if (end){ storyPaused = false; storySetupAct(0); return; }
    storyPaused = !storyPaused; if (!storyPaused) storyUserCam = false;
    $("story-btn-play").textContent = storyPaused ? "▶" : "❚❚";
  };
  if ($("story-btn-exit")) $("story-btn-exit").onclick = exitStoryMode;

  if (window.innerWidth <= 800) {
    ["hud", "stats", "foot"].forEach(p => {
      const el = $(p); if (el) el.classList.add("panel-hidden");
      const btn = $("btn-" + p); if (btn) btn.classList.remove("open");
    });
  }
  document.querySelectorAll(".nav-story-btn, .fab-story, .btn-story-main").forEach(b => b.classList.add("active"));
  sfStoryOn(true);
  storySetupAct(0);
}

// ------------------------------------------------------------------ R5 micro tier: the focal 1:1 neighbourhood
// The world below is `cewp_sim::micro::MicroWorld` (crates/cewp-sim/src/micro.rs), run in the wasm kernel.
// This code only builds it for the tile under the camera, steps it, and draws what it exports:
// every street, house, device, link and row on screen is the sim's state, nothing decorative.
const FOCAL_ZOOM = 24, EARTH_M = 6371000, TILE_DEG = 0.02, MICRO_MF = (EARTH_M * Math.PI / 180) / 111320;
const mcv = document.getElementById("micro"), mctx = mcv.getContext("2d");
const MK = ["phone", "home node", "laptop", "hosted server", "serve seat", "CI runner"];
const MK_COL = ["#8fd3ff", "#5ad38f", "#c3b6ff", "#ffcf7a", "#6cb6ff", "#ff9ec7"];
const MACT = ["online, idle", "asleep: suspended, every link closed", "resumed: waiting for its re-announce", "punching a direct path", "signing and sending a row", "forwarding a sealed row", "holding mail for a sleeping member", "booting and homing", "running its CI job", "not present"];
// cewp_sim::transport::BudgetClass codes (Off 0 … Seat 5)
const MCLASS = ["off", "phone-cellular", "phone-wifi", "home", "server", "seat"];
const MCLASS_BPS = [0, 350, 1400, 2800, 7000, 33000];
const MFONT = "ui-monospace,Menlo,Consolas,monospace";
const MLINK = ["homing link to a serve seat", "seat mesh", "LAN (AutoInterface)", "dialled to a public listener", "punched direct path"];
const MROUTE = ["direct", "relayed", "held at a mailbox", "mail handed over", "to the federation"];
const MROUTER = ["public listener (IPv6 / forwarded port)", "NAT a facilitated punch can traverse", "hard NAT / CGNAT: no direct path in"];
const MDCLASS = ["apartment blocks", "terraced streets", "suburban streets", "village"];
// cewp_sim::micro::P, in index order (asserted against micro_param_count at build)
const MP = { reach_home_share: 0, server_tier_share: 4, relay_offering_share: 5, phone_wifi_share: 7, phone_charging_share: 8, home_shaping_opt_in: 9, phone_shaping_opt_in: 10, adoption: 11, serve_seats: 17, homing_k: 18, start_local_hour: 39, claimed_share: 40, bundle_bootstrap: 41, seat_km_min: 42, seat_km_max: 43 };
const MP_COUNT = 44;
// default = production's shape (one serve seat, k = 1, far away); true = the design's multi-seat case
// (MicroParams::design_multi_seat: three seats, k = 2, 120-650 km), a named scenario, not the baseline
let microDesign = false;
// edge #881 bundle bootstrap: 1 = v45 (shipping: edge release/45.0.0 at 8e46165, untagged when read 2026-11-11), 0 = v44 (today),
// 2 = the whole-plane diff (a design comparison, never shipped). Button cycles 1 → 0 → 2.
let microBootstrap = 1;
const BS_LABEL = ["v44 (today)", "v45 (shipping)", "whole-plane (design comparison)"];
const BS_SHORT = BS_LABEL;
const BS_TITLE = ["edge v44, today's release: bundle bootstrap off; an unclaimed device announces but no seat serves it", "edge release/45.0.0 at 8e46165 (untagged when read 2026-11-11), the default: every device rounds with the bundle's seats, its required set by subject Pull and one Revocation round per cadence, with the read fix, the listing cache and the read budget", "on with the whole-plane diff of #881 before #971: never shipped, a design comparison"];
// the capacity model's arguments for each setting (cap_seat: shape, read set, candidate listing
// changes): v44 = whole-plane rounds as shipped; v45 = required-set Pull with the read fix, the
// listing cache and the read budget; whole-plane = v45's read path with whole-plane rounds
const CAP_ARGS = [[0, 0, 0], [1, 1, 1], [0, 1, 1]];
const bsCap = () => CAP_ARGS[microBootstrap];
// deepest zoom: about 6 px per metre on this screen (a house is ~60 px)
function zoomMax(){ const m = Math.min(cv.clientWidth || 800, cv.clientHeight || 800); return 6 * EARTH_M / (0.86 * m / 2); }
let M = null, microTimeScale = 20, microSel = -1, microLastReal = performance.now(), microGlide = 0, microNoLand = false, microStory = false;
let microStoryDim = null; // Set of node indices drawn at full strength while the story follows them
const sstep = (a, b, x) => { const t = Math.max(0, Math.min(1, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
const microOk = () => !!(wasm && wasm.micro_build && wasm.micro_param_count && wasm.micro_param_count() === MP_COUNT);
function hash32(a, b){ let h = (Math.imul(a | 0, 0x9E3779B1) ^ Math.imul(b | 0, 0x85EBCA77)) >>> 0; h ^= h >>> 15; h = Math.imul(h, 0x2C1B3C6D) >>> 0; h ^= h >>> 12; h = Math.imul(h, 0x297A2D39) >>> 0; h ^= h >>> 15; return h >>> 0; }
// a cell's own ground: the grid is 1.5° squares whose people are scattered within ±0.48° of the
// centre (cellRad), so a cell's tile is cellRad × 1.5625 on each side of its centre
const cellHalf = (G, c) => Math.max(0.04, G.cellRad[c]) * 1.5625;
function cellAt(lat, lon){
  const G = initPopGrid(); let best = -1, bd = 1e9;
  for (let c = 0; c < G.nCells; c++){
    const r = cellHalf(G, c) * 1.02; const dla = Math.abs(G.cellLat[c] - lat); if (dla > r) continue;
    const dlo = Math.abs(((G.cellLon[c] - lon + 540) % 360) - 180) * Math.cos(lat * Math.PI / 180); if (dlo > r) continue;
    const d = dla + dlo; if (d < bd){ bd = d; best = c; }
  }
  return best;
}
function cellDensity(c){
  const G = initPopGrid(); const share = G.cumW[c] - (c > 0 ? G.cumW[c - 1] : 0); const r = cellHalf(G, c);
  const area = (2 * r * 111.32) * (2 * r * 111.32 * Math.cos(G.cellLat[c] * Math.PI / 180));
  return share * 8.05e9 / Math.max(1, area); // the grid is real population density; the page's adoption is a separate parameter
}
function microSetParams(adoption, lon){
  wasm.micro_params_reset();
  const p = new Float64Array(mem.buffer, wasm.micro_params_ptr() >>> 0, MP_COUNT);
  const v = (id) => +$(id).value / 100;
  p[MP.reach_home_share] = 1 - v("sl_nat");
  p[MP.server_tier_share] = v("sl_srv");
  p[MP.relay_offering_share] = v("sl_relay");
  p[MP.phone_wifi_share] = v("sl_wifi");
  p[MP.phone_charging_share] = v("sl_chg");
  p[MP.home_shaping_opt_in] = v("sl_sh_home");
  p[MP.phone_shaping_opt_in] = v("sl_sh_phone");
  p[MP.adoption] = adoption;
  p[MP.bundle_bootstrap] = microBootstrap ? 1 : 0; // the world rounds the same for 1 and 2; the shape is costed in capacity
  if (microDesign){ p[MP.serve_seats] = 3; p[MP.homing_k] = 2; p[MP.seat_km_min] = 120; p[MP.seat_km_max] = 650; }
  p[MP.start_local_hour] = ((getSimUtcHours(getTime()) + lon / 15) % 24 + 24) % 24;
}
function microBuf(i, T){ return new T(mem.buffer, wasm.micro_buf_ptr(i) >>> 0, wasm.micro_buf_len(i) >>> 0).slice(); }
function microBuild(lat, lon, density, exact, seedLo, seedHi, adoption, key){
  microSetParams(adoption, lon);
  wasm.micro_build(lat, lon, density, seedLo >>> 0, seedHi >>> 0, exact ? 1 : 0);
  M = { key, lat0: lat, lon0: lon, roads: microBuf(4, Float64Array), sites: microBuf(7, Float64Array) };
  microRead();
  const a = lat * Math.PI / 180, o = lon * Math.PI / 180;
  M.c = [Math.cos(a) * Math.sin(o), Math.sin(a), Math.cos(a) * Math.cos(o)];
  M.E = [Math.cos(o), 0, -Math.sin(o)];
  M.N = [-Math.sin(a) * Math.sin(o), Math.cos(a), -Math.sin(a) * Math.cos(o)];
  microSel = -1; renderNodeCard();
}
function microBuildLeaf(leaf, adoption, key){
  microSetParams(adoption, tlCentre(leaf)[1]);
  const ctx = tlTree().context(leaf), cp = wasm.holon_ctx_ptr(); new Float64Array(mem.buffer, cp, 32).set(ctx);
  const id = HOLON.pack(leaf); wasm.micro_build_leaf(id);
  M = { key, leaf: id, lat0: 0, lon0: 0, roads: microBuf(4, Float64Array), sites: microBuf(7, Float64Array) };
  microRead(); M.lat0 = M.meta[35]; M.lon0 = M.meta[36];
  const st = HOLON.STEP[leaf.level] / HOLON.UPD * 111320; M.leafHW = st * Math.cos(M.lat0 * Math.PI / 180) / 2; M.leafHH = st / 2; // the leaf, not a square
  const a = M.lat0 * Math.PI / 180, o = M.lon0 * Math.PI / 180;
  M.c = [Math.cos(a) * Math.sin(o), Math.sin(a), Math.cos(a) * Math.cos(o)];
  M.E = [Math.cos(o), 0, -Math.sin(o)];
  M.N = [-Math.sin(a) * Math.sin(o), Math.cos(a), -Math.sin(a) * Math.cos(o)];
  microSel = -1; renderNodeCard();
}
function microRead(){
  wasm.micro_export();
  M.nf = microBuf(0, Float64Array); M.nu = microBuf(1, Uint32Array); M.links = microBuf(2, Uint32Array); M.ev = microBuf(3, Float64Array);
  M.bld = microBuf(5, Float64Array); M.hh = microBuf(6, Float64Array); M.pun = microBuf(8, Uint32Array); M.meta = microBuf(9, Float64Array);
  M.n = M.nu.length / 8; M.t = M.meta[0]; M.half = M.meta[3];
  // rounds served / announces per node: a served round flashes the device for a moment
  const cnt = microBuf(11, Uint32Array), prev = M.cnt, now = performance.now();
  if (!M.flash || M.flash.length !== M.n) M.flash = new Float64Array(M.n);
  if (prev && prev.length === cnt.length) for (let i = 0; i < M.n; i++) if (cnt[i * 2] > prev[i * 2]) M.flash[i] = now;
  M.cnt = cnt;
  if (!M.capT || now - M.capT > 1000){ M.capT = now; M.seatCap = seatCapPatch(); }
}
// ---- seat capacity: the calibrated seat model (cewp_sim::seat_view / seat_hooks through kernel-wasm views.rs).
// kernel-wasm's cap_seat (one point, growth fixed at 1, no interval) is no longer read by the page.
const KEY_PLANE = ["Key", "occurrence", "Revocation", "occurrence-withdrawal"];
// the seat card's rows on the calibrated seat model (cewp_sim::seat_view::seat_view_fleet via kernel-wasm
// view_seat_fleet, slot 32): v45.0.0, one seat, CostParams::calibrated() with key_publish_set growth g by
// scenario (0 = g 0, today's publish set; 4 = g 1, grows with the fleet), each figure with its outer bounds over
// the input intervals. Only lower and upper are printed (the central value is never shown alone).
const seatFleetMemo = new Map();
function seatFleet(devices, sc){
  if (!wasm || !wasm.view_seat_fleet) return null;
  const d = Math.max(1, Math.round(devices)), key = d + ":" + sc; if (seatFleetMemo.has(key)) return seatFleetMemo.get(key);
  if (seatFleetMemo.size > 256) seatFleetMemo.clear();
  const n = wasm.view_seat_fleet(d, sc) >>> 0; const p = wasm.view_ptr(32) >>> 0, b = new Float32Array(mem.buffer, p, wasm.view_len(32) >>> 0).slice();
  const r = n === 18 && b.length === 18 ? { devices: b[0], g: b[1], cores: [b[3], b[4]], reads: [b[6], b[7]], rho: [b[9], b[10]], binds: b[11], lock: [b[13], b[14]], plane: b[15], subjects: b[16], cls: b[17] } : null;
  seatFleetMemo.set(key, r); return r;
}
const ivN = (a, f) => f(a[0]) === f(a[1]) ? f(a[0]) : f(a[0]) + " to " + f(a[1]);
const ivRho = (a) => ivN(a, (x) => x >= 10 ? x.toFixed(0) : x.toFixed(2));
const seatCell = (r) => r ? `${ivN(r.cores, fmtN)} cores · ρ ${ivRho(r.rho)}${r.rho[0] >= 1 ? " · <b>saturated</b>" : r.rho[1] >= 1 ? " · may saturate" : ""}` : "–";
// the seat knee as an interval (twin-calibration, cewp_sim::seat_hooks::calibration_view_knee via
// kernel-wasm view_calibration_knee, slot 18): [lower, central, upper, kind, binds lower, binds upper,
// class], -1 = no claim. Scenario 0..4 is key_publish_set growth g = 0, 0.25, 0.5, 0.75, 1 (FRONTIER).
// Shown only as lower to upper with g named; the central value is never printed alone.
const CAL_G = [0, 0.25, 0.5, 0.75, 1];
const calKneeMemo = new Map();
function calKnee(sc){
  if (!wasm || !wasm.view_calibration_knee) return null;
  if (calKneeMemo.has(sc)) return calKneeMemo.get(sc);
  wasm.view_calibration_knee(sc);
  const p = wasm.view_ptr(18) >>> 0, n = wasm.view_len(18) >>> 0, b = new Float32Array(mem.buffer, p, n).slice();
  const r = n === 7 ? { lo: b[0], hi: b[2], kind: b[3], bindsLo: b[4], bindsHi: b[5], cls: b[6], g: CAL_G[sc] } : null;
  calKneeMemo.set(sc, r); return r;
}
const calBinds = (c) => c === 0 ? "CPU" : c === 1 ? "sweep permits" : c >= 2 ? (KEY_PLANE[c - 2] || "a key-plane") + " plane lock" : "nothing";
// the seat card's knee sentence, both intervals from the view (publish set growing with the fleet = g 1,
// staying at today's size = g 0); "no knee claimed" where the view says -1
function seatKneeText(){
  const g1 = calKnee(4), g0 = calKnee(0), d = (x) => x < 0 ? "no knee claimed" : Math.round(x).toLocaleString();
  if (!g1 || !g0) return "–";
  return d(g1.lo) + " to " + d(g1.hi) + " devices if the server's publish set grows with the fleet (binds on " + calBinds(g1.bindsLo) + "); " + d(g0.lo) + " to " + d(g0.hi) + " if it stays at today's size (binds on " + calBinds(g0.bindsLo) + " at the low end, " + calBinds(g0.bindsHi) + " at the high end)";
}
// devices in this patch that round with seat s now (online, homed on s, with a seat as peer)
function seatRounders(s){
  let n = 0;
  for (let i = 0; i < M.n; i++){
    const f = nodeFlags(i); if (!(f & NF.online) || !(f & NF.served)) continue;
    if (M.nu[i * 8 + 3] === s || M.nu[i * 8 + 4] === s) n++;
  }
  return n;
}
function seatCapPatch(){
  const out = new Map();
  for (let i = 0; i < M.n; i++) if (nodeKind(i) === 4){ const d = seatRounders(i), a = seatFleet(d, 0), b = seatFleet(d, 4); out.set(i, { d, c: a && b ? { cores: [Math.min(a.cores[0], b.cores[0]), Math.max(a.cores[1], b.cores[1])] } : null }); }
  return out;
}
// free roam: the tile under the screen centre (the centre of an orthographic view is lat = pitch, lon = yaw)
function microFreeRoam(){
  if (!microOk() || microStory) return;
  if (zoom < FOCAL_ZOOM){ return; }
  if (drag) return;
  // at street scale keep the patch you are exploring; a patch built before the tiling was ready
  // is replaced by the leaf under the centre, so the focal world always sits in the tiled streets
  const tiled = typeof tlReady === "function" && tlReady();
  if (M && zoom >= 1500 && !(tiled && !String(M.key).startsWith("h"))) return;
  const lat = pitch * 180 / Math.PI, lon = ((yaw * 180 / Math.PI + 540) % 360) - 180;
  const adoption = Math.min(0.98, currentPop / 8.05e9);
  // with the tiling ready, the focal world is the leaf holon under the centre (MicroWorld::from_leaf)
  if (tiled){
    const T = tlTree(), leaf = T.leafAt(lat, lon), id = HOLON.pack(leaf), key = "h" + id + ":" + paramKey();
    if (M && M.key === key) return;
    const b = HOLON.boundsDeg(leaf);
    if (!T.budget(leaf) || !tlLand().squareOnLand(b[0], b[1], b[2] - b[0])){ microNoLand = true; M = null; return; }
    microNoLand = false; microBuildLeaf(leaf, adoption, key); microGlide = 1; return;
  }
  const ti = Math.floor(lat / TILE_DEG), tj = Math.floor(lon / TILE_DEG);
  const key = ti + ":" + tj + ":" + paramKey();
  if (M && M.key === key) return;
  const tlat = (ti + 0.5) * TILE_DEG, tlon = (tj + 0.5) * TILE_DEG;
  const c = cellAt(tlat, tlon);
  // streets are built only on a tile wholly on land: inside the coastline, with no coast crossing it
  if (c < 0 || !LAND || !LAND.squareOnLand(ti * TILE_DEG, tj * TILE_DEG, TILE_DEG)){ microNoLand = true; M = null; return; }
  microNoLand = false;
  microBuild(tlat, tlon, cellDensity(c), false, hash32(ti, tj), hash32(tj ^ 0x5bd1e995, ti), adoption, key);
  microGlide = 1;
}
function paramKey(){ return ["sl_nat", "sl_srv", "sl_relay", "sl_wifi", "sl_chg", "sl_sh_home", "sl_sh_phone"].map(id => $(id).value).join(",") + "," + currentPop + (microDesign ? ",design" : ""); }
// the seats a node homes on, as the world built them (one in production's shape)
function homesOf(i){ return [M.nu[i * 8 + 3], M.nu[i * 8 + 4]].filter(s => s < 4294967295); }
const seatPhrase = (n) => n === 1 ? "the one serve seat it homes on" : n === 2 ? "the two serve seats it homes on (k=2, design case)" : "no serve seat";
// camera, in float64: q = R p with R from rot()
function camQ(x, y, z){ const cy = Math.cos(yaw), sy = Math.sin(yaw), cp = Math.cos(pitch), sp = Math.sin(pitch);
  return [cy * x - sy * z, -sp * sy * x + cp * y - sp * cy * z, cp * sy * x + sp * y + cp * cy * z]; }
function camFrame(){ const w = cv.clientWidth, h = cv.clientHeight; const k = 0.86 * zoom * Math.min(w, h) / 2; return { w, h, k }; }
function projLL(F, lat, lon){ const v = xyz(lat, lon); const q = camQ(v[0], v[1], v[2]); return [F.w / 2 + F.k * q[0], F.h / 2 - F.k * q[1], q[2]]; }
function patchAffine(F){
  // the world's metres are 111,320 a degree (MicroWorld::local_xy); scale them onto the globe's sphere
  const qc = camQ(...M.c), qE = camQ(...M.E), qN = camQ(...M.N), s = F.k / EARTH_M * MICRO_MF;
  return { a: s * qE[0], b: -s * qE[1], c: s * qN[0], d: -s * qN[1], e: F.w / 2 + F.k * qc[0], f: F.h / 2 - F.k * qc[1], front: qc[2], pxPerM: s };
}
const nodeKind = (i) => M.nu[i * 8] & 0xff, nodeAct = (i) => (M.nu[i * 8] >>> 8) & 0xff, nodeClass = (i) => (M.nu[i * 8] >>> 16) & 0xff, nodeShaped = (i) => M.nu[i * 8] >>> 24;
const nodeFlags = (i) => M.nu[i * 8 + 1];
const NF = { present: 1, awake: 2, online: 4, unmetered: 8, charging: 16, public: 32, punchable: 64, clientOnly: 128, shapesAll: 256, relay: 512, old: 1024, pinned: 2048, mailbox: 4096, offPatch: 8192, optIn: 16384, relayPref: 32768, deviceRelays: 65536, claimed: 131072, served: 262144 };
const fmtN = (x) => !isFinite(x) ? "∞" : x >= 1e6 ? (x / 1e6).toFixed(2) + "M" : x >= 1e4 ? (x / 1e3).toFixed(1) + "k" : x >= 100 ? Math.round(x).toLocaleString() : x >= 1 ? x.toFixed(2) : x.toFixed(3);
function nodeScreen(F, A, i){
  if (nodeFlags(i) & NF.offPatch){ const p = projLL(F, M.nf[i * 4 + 2], M.nf[i * 4 + 3]); return p; }
  const x = M.nf[i * 4], y = M.nf[i * 4 + 1]; return [A.a * x + A.c * y + A.e, A.b * x + A.d * y + A.f, A.front];
}
function microVisible(){ return M && (zoom >= FOCAL_ZOOM * 0.6 || microStory); }
function microStep(now){
  const dtReal = Math.min(0.1, (now - microLastReal) / 1000); microLastReal = now;
  if (!M || !microOk()) return;
  if (zoom >= FOCAL_ZOOM * 0.6 || microStory){
    if (!(storyActive && storyPaused) && (typeof microRunning!=="function" || microRunning())) wasm.micro_step(dtReal * microTimeScale);
    microRead();
  }
  if (microGlide > 0 && !drag && !microStory){
    const tl = M.lat0 * Math.PI / 180, to = M.lon0 * Math.PI / 180;
    let dy = to - yaw; while (dy < -Math.PI) dy += 2 * Math.PI; while (dy > Math.PI) dy -= 2 * Math.PI;
    yaw += dy * 0.08; pitch += (tl - pitch) * 0.08; velYaw = 0; velPitch = 0;
    if (Math.abs(dy) < 1e-9 && Math.abs(tl - pitch) < 1e-9) microGlide = 0;
  }
}
// ---- drawing
const GROUND = ["#2a2e35", "#2c3330", "#26332a", "#28372a"], ROADC = ["#4a505b", "#4c525a", "#4a5048", "#5a5644"];
function microDraw(){
  const dpr = window.devicePixelRatio || 1; const F = camFrame();
  if (mcv.width !== (F.w * dpr | 0) || mcv.height !== (F.h * dpr | 0)){ mcv.width = F.w * dpr | 0; mcv.height = F.h * dpr | 0; }
  const g = mctx; g.setTransform(1, 0, 0, 1, 0, 0); g.clearRect(0, 0, mcv.width, mcv.height);
  updateFocalChip(F); if (frame % 15 === 0) measureSafe(F);
  if (!M || !microOk()) return;
  const A = patchAffine(F); if (A.front <= 0.02) return;
  const P = 2 * M.half * A.pxPerM, ppm = A.pxPerM; // patch size on screen, pixels per metre
  const cls = M.meta[2] | 0, hour = M.meta[1];
  const day = sstep(5.5, 7.5, hour) * (1 - sstep(19.5, 21.5, hour));
  const outsideA = sstep(110, 420, P), groundA = sstep(50, 200, P), roadA = sstep(70, 240, P), bldA = sstep(0.22, 0.6, ppm), nodeA = sstep(30, 120, P);
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  // beyond the focal subgraph: land the twin keeps as cohorts, dimmed
  if (outsideA > 0 && !(typeof TL !== "undefined" && TL.on)){ g.fillStyle = `rgba(12,15,14,${0.55 * outsideA})`; g.fillRect(0, 0, F.w, F.h); }
  // the patch, in metres
  g.setTransform(dpr * A.a, dpr * A.b, dpr * A.c, dpr * A.d, dpr * A.e, dpr * A.f);
  const H = M.half;
  // with the street tiling on, the GPU draws this leaf's ground and streets with its neighbours';
  // the canvas adds the boundary of the simulated neighbourhood, its buildings and its devices
  const tiled = typeof TL !== "undefined" && TL.on && M.leaf !== undefined;
  if (groundA > 0){
    if (!tiled){ g.globalAlpha = groundA; g.fillStyle = GROUND[cls]; g.fillRect(-H, -H, 2 * H, 2 * H); }
    g.globalAlpha = groundA * 0.9; g.strokeStyle = "rgba(108,182,255,0.55)"; g.lineWidth = 1.4 / ppm; g.setLineDash([6 / ppm, 5 / ppm]); g.strokeRect(-(M.leafHW || H), -(M.leafHH || H), 2 * (M.leafHW || H), 2 * (M.leafHH || H)); g.setLineDash([]);
  }
  if (roadA > 0 && !tiled){
    g.save(); g.beginPath(); g.rect(-H, -H, 2 * H, 2 * H); g.clip(); g.lineCap = "round";
    const R = M.roads;
    for (let pass = 0; pass < 2; pass++){
      g.globalAlpha = roadA * (pass ? 1 : 0.8); g.strokeStyle = pass ? ROADC[cls] : "#1a1d22";
      for (let r = 0; r < R.length; r += 5){ g.lineWidth = R[r + 4] + (pass ? 0 : 2.2); g.beginPath(); g.moveTo(R[r], R[r + 1]); g.lineTo(R[r + 2], R[r + 3]); g.stroke(); }
    }
    if (ppm > 1.6){ g.globalAlpha = roadA * 0.35; g.strokeStyle = "#d9d2a8"; g.lineWidth = 0.25; g.setLineDash([3, 4]);
      for (let r = 0; r < R.length; r += 5){ if (R[r + 4] < 7) continue; g.beginPath(); g.moveTo(R[r], R[r + 1]); g.lineTo(R[r + 2], R[r + 3]); g.stroke(); }
      g.setLineDash([]); }
    g.restore();
  }
  if (bldA > 0){
    const B = M.bld;
    for (let b = 0; b < B.length; b += 8){
      const cx = B[b], cy = B[b + 1], w = B[b + 2], d = B[b + 3], ang = B[b + 4], units = B[b + 6], adopt = B[b + 7];
      // a cast shadow (fixed light from the north-west, display only), then a pitched roof in two tones
      // split along the long axis, so a footprint reads as a building and not a flat block
      g.save(); g.globalAlpha = bldA * 0.45; g.translate(cx + 1.2 + 0.08 * Math.min(w, d), cy + 1.2 + 0.08 * Math.min(w, d)); g.rotate(ang); g.fillStyle = "#05070a"; g.fillRect(-w / 2, -d / 2, w, d); g.restore();
      g.save(); g.translate(cx, cy); g.rotate(ang); g.globalAlpha = bldA;
      const base = units === 0 ? [92, 90, 84] : adopt > 0 ? (day > 0.5 ? [170, 152, 128] : [118, 104, 86]) : [128, 130, 138];
      const lit = base.map((v) => Math.min(255, Math.round(v * 1.18))), shade = base.map((v) => Math.round(v * 0.72));
      if (w >= d){ g.fillStyle = `rgb(${lit})`; g.fillRect(-w / 2, -d / 2, w, d / 2); g.fillStyle = `rgb(${shade})`; g.fillRect(-w / 2, 0, w, d / 2); }
      else { g.fillStyle = `rgb(${lit})`; g.fillRect(-w / 2, -d / 2, w / 2, d); g.fillStyle = `rgb(${shade})`; g.fillRect(0, -d / 2, w / 2, d); }
      g.strokeStyle = "rgba(255,255,255,0.18)"; g.lineWidth = 0.3; g.beginPath(); if (w >= d){ g.moveTo(-w / 2, 0); g.lineTo(w / 2, 0); } else { g.moveTo(0, -d / 2); g.lineTo(0, d / 2); } g.stroke(); // the ridge
      g.strokeStyle = adopt > 0 ? "rgba(255,207,122,0.7)" : "rgba(0,0,0,0.6)"; g.lineWidth = adopt > 0 ? 0.7 : 0.5; g.strokeRect(-w / 2, -d / 2, w, d);
      if (adopt > 0 && day < 0.6){ g.fillStyle = `rgba(255,200,110,${0.22 * (1 - day)})`; g.fillRect(-w / 2 + 1, -d / 2 + 1, w - 2, d - 2); }
      g.restore();
    }
  }
  g.globalAlpha = 1; g.setTransform(dpr, 0, 0, dpr, 0, 0);
  // the patch as a lit spot from far away
  const ctr = [A.e, A.f];
  if (P < 260){
    const rad = Math.max(4, P * 0.55), a = 1 - sstep(60, 220, P);
    const gr = g.createRadialGradient(ctr[0], ctr[1], 0, ctr[0], ctr[1], rad * 2.2);
    gr.addColorStop(0, `rgba(255,214,140,${0.85 * a})`); gr.addColorStop(0.35, `rgba(255,177,92,${0.35 * a})`); gr.addColorStop(1, "rgba(255,177,92,0)");
    g.fillStyle = gr; g.beginPath(); g.arc(ctr[0], ctr[1], rad * 2.2, 0, 7); g.fill();
  }
  // screen positions of every present node
  const n = M.n, sx = new Float64Array(n), sy = new Float64Array(n), vis = new Uint8Array(n);
  for (let i = 0; i < n; i++){
    if (!(nodeFlags(i) & NF.present)) continue;
    const p = nodeScreen(F, A, i); sx[i] = p[0]; sy[i] = p[1]; vis[i] = p[2] > 0 ? 1 : 0;
  }
  M.sx = sx; M.sy = sy; M.vis = vis;
  if (P > 300) placeEdgeMarkers(F); else M.markers = [];
  const onScreen = (i) => vis[i] && sx[i] > -40 && sy[i] > -40 && sx[i] < F.w + 40 && sy[i] < F.h + 40;
  const dimOf = (i) => (microStoryDim && !microStoryDim.has(i)) ? 0.22 : 1;
  const sel = microSel;
  const selDim = (a, b) => sel < 0 ? 1 : (a === sel || b === sel) ? 1.6 : 0.35;
  // far view: arcs from the street to its seats and servers
  if (P < 160){
    const a = 1 - sstep(60, 160, P), seen = new Set();
    for (let l = 0; l < M.links.length; l += 4){
      const x = M.links[l], y = M.links[l + 1]; const kx = nodeFlags(x) & NF.offPatch, ky = nodeFlags(y) & NF.offPatch;
      if (!!kx === !!ky) continue; const far = kx ? x : y; if (seen.has(far)) continue; seen.add(far);
      const shaped = ((M.links[l + 2] >>> 8) & 0xff) > 0;
      drawArc(g, F, M.lat0, M.lon0, M.nf[far * 4 + 2], M.nf[far * 4 + 3], shaped ? "255,177,92" : "90,211,143", 0.55 * a);
    }
  }
  // links
  if (nodeA > 0){
    g.lineCap = "round";
    for (let l = 0; l < M.links.length; l += 4){
      const x = M.links[l], y = M.links[l + 1], pk = M.links[l + 2], kind = pk & 0xff, shaped = (pk >>> 8) & 0xff, atBound = (pk >>> 16) & 1;
      if (!vis[x] && !vis[y]) continue;
      const fx = nodeFlags(x) & NF.offPatch, fy = nodeFlags(y) & NF.offPatch;
      if (fx && fy && P > 160) continue; // seat mesh / server-to-seat: drawn from far away only
      let ax = sx[x], ay = sy[x], bx = sx[y], by = sy[y];
      let alpha = nodeA * selDim(x, y) * Math.min(dimOf(x), dimOf(y)) * 1.0;
      let col, wdt;
      if (kind === 0){ col = shaped ? "255,177,92" : "90,211,143"; alpha *= (shaped ? 0.5 : 0.12) * (0.3 + 0.7 * sstep(0.8, 2.5, ppm)); wdt = shaped ? 0.8 + 0.6 * sstep(0.8, 2.5, ppm) : 0.8; }
      else if (kind === 1){ col = "108,182,255"; alpha *= 0.5; wdt = 1.6; }
      else if (kind === 2){ col = "90,211,143"; alpha *= 0.8; wdt = 1.6; }
      else if (kind === 3){ col = shaped ? "255,177,92" : "90,211,143"; alpha *= 0.7; wdt = 1.5; }
      else { col = shaped ? "255,177,92" : "120,240,170"; alpha *= 0.95; wdt = 2.2; }
      if (microStoryDim && (microStoryDim.has(x) || microStoryDim.has(y))){ alpha = Math.max(alpha * 4, 0.75); wdt += 0.8; }
      if (alpha <= 0.01) continue;
      // a link to a node off the screen ends at the screen edge, pointing at it
      [bx, by] = clipToScreen(ax, ay, bx, by, F); [ax, ay] = clipToScreen(bx, by, ax, ay, F);
      g.strokeStyle = `rgba(${col},${Math.min(1, alpha)})`; g.lineWidth = wdt;
      if (kind === 4) g.setLineDash([]); else if (atBound && kind === 0) g.setLineDash([2, 3]);
      g.beginPath(); g.moveTo(ax, ay); g.lineTo(bx, by); g.stroke(); g.setLineDash([]);
    }
    // punches in progress
    const tt = getTime();
    for (let p = 0; p < M.pun.length; p += 2){
      const x = M.pun[p], y = M.pun[p + 1]; if (!vis[x] || !vis[y]) continue;
      g.strokeStyle = `rgba(255,236,120,${0.9 * nodeA})`; g.lineWidth = 2; g.setLineDash([5, 5]); g.lineDashOffset = -tt * 30;
      g.beginPath(); g.moveTo(sx[x], sy[x]); g.lineTo(sx[y], sy[y]); g.stroke(); g.setLineDash([]); g.lineDashOffset = 0;
    }
    drawPulses(g, F, nodeA, dimOf);
  }
  // nodes
  MTAGS.length = 0; MBOXES.length = 0;
  if (nodeA > 0){
    const s = Math.max(F.w < 600 ? 7 : 5, Math.min(F.w < 600 ? 14 : 12, 3.4 + ppm * 2.6)); // at least 7 px on a phone: a glyph you can tell apart and tap
    for (let i = 0; i < n; i++){
      if (!(nodeFlags(i) & NF.present) || !onScreen(i)) continue;
      const off = nodeFlags(i) & NF.offPatch; if (off && P > 300) continue; // drawn as edge markers instead
      const big = microStoryDim && microStoryDim.has(i) ? 1.5 : 1;
      drawNodeGlyph(g, i, sx[i], sy[i], (off ? Math.max(3, s * 0.8) : s) * big, nodeA * dimOf(i) * (sel < 0 || sel === i ? 1 : 0.55), ppm);
    }
    if (P > 300) drawEdgeMarkers(g, F, nodeA);
    if (sel >= 0 && sel < n && vis[sel]){
      let px = sx[sel], py = sy[sel]; [px, py] = clipToScreen(F.w / 2, F.h / 2, px, py, F, 30);
      g.strokeStyle = "#fff"; g.lineWidth = 2; g.beginPath(); g.arc(px, py, Math.max(9, s * 1.5), 0, 7); g.stroke();
    }
  }
  const boxes = drawStoryTags(g, F, nodeA);
  microTags(g, F, boxes, sel, dimOf);
  drawScaleBar(g, F, ppm);
}
let safeTop = 60, safeBot = 60;
function measureSafe(F){
  const nav = document.querySelector(".nav-strip"), sfb = $("sf-bar");
  safeTop = nav && nav.offsetParent !== null ? nav.getBoundingClientRect().bottom + 8 : sfb ? sfb.getBoundingClientRect().bottom + 8 : 60;
  const ls = $("live-strip"); if (ls && ls.style.display !== "none"){ safeTop = ls.getBoundingClientRect().bottom + 8; }
  let b = F.h; for (const id of ["focal-chip", "story-hud"]){ const el = $(id); if (el && !el.classList.contains("panel-hidden") && el.offsetParent !== null){ const r = el.getBoundingClientRect(); if (r.height) b = Math.min(b, r.top); } }
  safeBot = F.h - b + 8;
}
function clipToScreen(ax, ay, bx, by, F, m = 14){
  const x0 = m, y0 = m + safeTop, x1 = F.w - m, y1 = F.h - m - safeBot;
  if (bx >= x0 && bx <= x1 && by >= y0 && by <= y1) return [bx, by];
  let t = 1; const dx = bx - ax, dy = by - ay;
  if (dx > 0) t = Math.min(t, (x1 - ax) / dx); else if (dx < 0) t = Math.min(t, (x0 - ax) / dx);
  if (dy > 0) t = Math.min(t, (y1 - ay) / dy); else if (dy < 0) t = Math.min(t, (y0 - ay) / dy);
  t = Math.max(0, t); return [ax + dx * t, ay + dy * t];
}
function drawArc(g, F, la1, lo1, la2, lo2, rgb, alpha){
  const a = xyz(la1, lo1), b = xyz(la2, lo2); const dot = Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2])); const th = Math.acos(dot); if (th < 1e-7) return;
  g.strokeStyle = `rgba(${rgb},${alpha})`; g.lineWidth = 1.3; g.beginPath(); let started = false;
  for (let s = 0; s <= 24; s++){
    const t = s / 24, w1 = Math.sin((1 - t) * th) / Math.sin(th), w2 = Math.sin(t * th) / Math.sin(th), h = 1 + Math.min(0.12, th * 0.25) * Math.sin(t * Math.PI);
    const q = camQ((a[0] * w1 + b[0] * w2) * h, (a[1] * w1 + b[1] * w2) * h, (a[2] * w1 + b[2] * w2) * h);
    const X = F.w / 2 + F.k * q[0], Y = F.h / 2 - F.k * q[1];
    if (q[2] < -0.05){ started = false; continue; }
    if (!started){ g.moveTo(X, Y); started = true; } else g.lineTo(X, Y);
  }
  g.stroke();
}
function drawPulses(g, F, alpha, dimOf){
  const E = M.ev, span = 2.2 * microTimeScale; // a row's pulse lasts 2.2 s of real time
  for (let e = 0; e < E.length; e += 7){
    const age = M.t - E[e]; if (age < 0 || age > span) continue;
    const src = E[e + 1], dst = E[e + 2], via = E[e + 3], via2 = E[e + 4], route = E[e + 5], shaped = E[e + 6];
    const pts = [src]; if (via < 4294967295 && via !== dst) pts.push(via); if (via2 < 4294967295 && via2 !== dst) pts.push(via2); if (route !== 2) pts.push(dst);
    if (pts.length < 2) continue;
    if (!pts.some(i => M.vis[i])) continue;
    const xy = pts.map(i => [M.sx[i], M.sy[i]]);
    const col = route === 1 ? "235,170,255" : route === 2 || route === 3 ? "255,236,150" : route === 4 ? "108,182,255" : shaped ? "255,177,92" : "120,240,170";
    const a = alpha * (1 - age / span) * Math.min(dimOf(src), dimOf(dst) * 2);
    g.strokeStyle = `rgba(${col},${0.55 * a})`; g.lineWidth = 2; g.setLineDash(route === 1 ? [4, 4] : []);
    g.beginPath(); xy.forEach((p, k) => { const c = clipToScreen(F.w / 2, F.h / 2, p[0], p[1], F, 6); k ? g.lineTo(c[0], c[1]) : g.moveTo(c[0], c[1]); }); g.stroke(); g.setLineDash([]);
    // the head travels the path
    const segs = []; let L = 0; for (let k = 1; k < xy.length; k++){ const d = Math.hypot(xy[k][0] - xy[k - 1][0], xy[k][1] - xy[k - 1][1]); segs.push(d); L += d; }
    let at = Math.min(1, age / (span * 0.6)) * L, k = 0; while (k < segs.length - 1 && at > segs[k]){ at -= segs[k]; k++; }
    const f = segs[k] ? at / segs[k] : 1; const hx = xy[k][0] + (xy[k + 1][0] - xy[k][0]) * f, hy = xy[k][1] + (xy[k + 1][1] - xy[k][1]) * f;
    if (hx < -20 || hy < -20 || hx > F.w + 20 || hy > F.h + 20) continue;
    g.fillStyle = `rgba(${col},${Math.min(1, a * 1.5)})`; g.beginPath(); g.arc(hx, hy, route === 2 ? 5 : 4.2, 0, 7); g.fill();
    g.strokeStyle = `rgba(10,12,16,${Math.min(1, a)})`; g.lineWidth = 1; g.stroke();
  }
}
function drawNodeGlyph(g, i, x, y, s, alpha, ppm){
  if (alpha <= 0.01) return;
  const kind = nodeKind(i), f = nodeFlags(i), act = nodeAct(i), online = f & NF.online;
  const col = MK_COL[kind]; g.globalAlpha = Math.min(1, alpha);
  g.lineWidth = Math.max(1, s * 0.16);
  g.fillStyle = online ? col : "rgba(20,24,30,0.9)"; g.strokeStyle = online ? "rgba(10,12,16,0.9)" : "#7d8796";
  if (!online) g.setLineDash([2, 2]);
  g.beginPath();
  if (kind === 0) roundRect(g, x - s * 0.32, y - s * 0.55, s * 0.64, s * 1.1, s * 0.16);
  else if (kind === 1) g.rect(x - s * 0.5, y - s * 0.5, s, s);
  else if (kind === 2){ g.moveTo(x - s * 0.62, y + s * 0.35); g.lineTo(x + s * 0.62, y + s * 0.35); g.lineTo(x + s * 0.45, y - s * 0.4); g.lineTo(x - s * 0.45, y - s * 0.4); g.closePath(); }
  else if (kind === 3) g.rect(x - s * 0.55, y - s * 0.45, s * 1.1, s * 0.9);
  else if (kind === 4){ for (let k = 0; k < 6; k++){ const a = k * Math.PI / 3 + Math.PI / 6; k ? g.lineTo(x + s * 0.7 * Math.cos(a), y + s * 0.7 * Math.sin(a)) : g.moveTo(x + s * 0.7 * Math.cos(a), y + s * 0.7 * Math.sin(a)); } g.closePath(); }
  else { g.moveTo(x, y - s * 0.6); g.lineTo(x + s * 0.6, y + s * 0.45); g.lineTo(x - s * 0.6, y + s * 0.45); g.closePath(); }
  g.fill(); g.stroke(); g.setLineDash([]);
  // state rings
  const tt = getTime();
  if (act === 2 || act === 7){ g.strokeStyle = "rgba(255,255,255,0.8)"; g.lineWidth = 1.2; g.beginPath(); g.arc(x, y, s * (0.9 + 0.3 * Math.sin(tt * 8)), 0, 7); g.stroke(); }
  else if (act === 4){ g.strokeStyle = "rgba(255,255,255,0.9)"; g.lineWidth = 1.3; g.beginPath(); g.arc(x, y, s * 0.95, 0, 7); g.stroke(); }
  else if (act === 5){ g.strokeStyle = "rgba(235,170,255,0.95)"; g.lineWidth = 1.5; g.beginPath(); g.arc(x, y, s * 1.0, 0, 7); g.stroke(); }
  else if (act === 3){ g.strokeStyle = "rgba(255,236,120,0.95)"; g.lineWidth = 1.3; g.setLineDash([2, 2]); g.beginPath(); g.arc(x, y, s * 1.05, 0, 7); g.stroke(); g.setLineDash([]); }
  if (M.flash && performance.now() - M.flash[i] < 900 && online){
    const k = (performance.now() - M.flash[i]) / 900; g.strokeStyle = `rgba(110,230,255,${0.9 * (1 - k)})`; g.lineWidth = 1.4;
    g.beginPath(); g.arc(x, y, s * (0.8 + 0.7 * k), 0, 7); g.stroke();
  }
  const holding = M.nu[i * 8 + 5];
  if (holding > 0){ envelope(g, x + s * 0.8, y - s * 0.85, Math.max(6, s * 0.6)); }
  if (ppm > 2.2 && kind !== 4 && !(f & NF.offPatch)){
    g.font = `${Math.max(8, Math.min(11, s * 0.85))}px ${MFONT}`; g.fillStyle = "rgba(230,236,245,0.85)"; g.textAlign = "left"; g.textBaseline = "middle";
    let tag = "";
    if (kind === 0) tag = (f & NF.unmetered ? "wifi" : "cell") + (f & NF.charging ? " ⚡" : "");
    if (f & NF.old) tag += (tag ? " " : "") + "old";
    if (f & NF.relay) tag += (tag ? " " : "") + "R";
    if (!(f & NF.claimed) && kind !== 3 && kind !== 5) tag += (tag ? " " : "") + ((f & NF.served) ? "unclaimed·bootstrap" : "unclaimed·no peer");
    // placed after every glyph and the story's name tags, so no two labels overlap (microTags)
    if (tag) MTAGS.push({ i, tag, x: x + s * 0.8, y: y + s * 0.45, a: Math.min(1, alpha), font: g.font });
  }
  g.globalAlpha = 1;
}
function roundRect(g, x, y, w, h, r){ g.moveTo(x + r, y); g.lineTo(x + w - r, y); g.quadraticCurveTo(x + w, y, x + w, y + r); g.lineTo(x + w, y + h - r); g.quadraticCurveTo(x + w, y + h, x + w - r, y + h); g.lineTo(x + r, y + h); g.quadraticCurveTo(x, y + h, x, y + h - r); g.lineTo(x, y + r); g.quadraticCurveTo(x, y, x + r, y); }
function envelope(g, x, y, s){ g.fillStyle = "#ffe696"; g.strokeStyle = "#3a3220"; g.lineWidth = 1; g.beginPath(); g.rect(x - s * 0.6, y - s * 0.4, s * 1.2, s * 0.8); g.fill(); g.stroke(); g.beginPath(); g.moveTo(x - s * 0.6, y - s * 0.4); g.lineTo(x, y + s * 0.1); g.lineTo(x + s * 0.6, y - s * 0.4); g.stroke(); }
// off-street nodes at street scale: one marker per data-centre site, at the screen edge in its direction
function placeEdgeMarkers(F){
  const bySite = new Map();
  for (let i = 0; i < M.n; i++){
    const f = nodeFlags(i); if (!(f & NF.present) || !(f & NF.offPatch)) continue;
    const site = M.nu[i * 8 + 7] & 0xffffff; if (!bySite.has(site)) bySite.set(site, []); bySite.get(site).push(i);
  }
  M.markers = [];
  const cx = F.w / 2, cy = F.h / 2;
  for (const [site, nodes] of bySite){
    const S = M.sites; const p = projLL(F, S[site * 4], S[site * 4 + 1]);
    const [mx, my] = clipToScreen(cx, cy, p[0], p[1], F, 26);
    for (const i of nodes){ M.sx[i] = mx; M.sy[i] = my; M.vis[i] = 1; }
    M.markers.push({ x: mx, y: my, node: nodes.find(i => nodeKind(i) === 4) ?? nodes[0], nodes, km: S[site * 4 + 2] });
  }
}
function drawEdgeMarkers(g, F, alpha){
  const placed = [];
  for (const mk of [...M.markers].sort((a, b) => a.y - b.y)){
    const { x: mx, y: my, nodes, km } = mk;
    const seats = nodes.filter(i => nodeKind(i) === 4), servers = nodes.filter(i => nodeKind(i) === 3), ci = nodes.filter(i => nodeKind(i) === 5);
    const parts = [];
    if (seats.length) parts.push("serve seat");
    if (servers.length) parts.push(servers.length > 1 ? `${servers.length} hosted servers` : "hosted server");
    if (ci.length) parts.push(ci.length > 1 ? `${ci.length} CI runners` : "CI runner");
    const label = parts.join(" + ") + " · " + Math.round(km).toLocaleString() + " km";
    let sub = "";
    if (seats.length){ const s0 = seats[0]; let homed = 0; for (let l = 0; l < M.links.length; l += 4) if ((M.links[l + 2] & 0xff) === 0 && (M.links[l] === s0 || M.links[l + 1] === s0)) homed++; sub = `${nodeShaped(s0)}/8 shaped · ${homed} homed here`;
      const sc = M.seatCap && M.seatCap.get(s0); if (sc && sc.c) sub += ` · CPU ${ivN(sc.c.cores, fmtN)} cores (MODEL, v45.0.0, interval over g 0 to 1)`; }
    const small = F.w < 800; let label2 = label;
    if (small){ label2 = (seats.length ? "seat" : ci.length ? "CI" : "server") + (parts.length > 1 ? "+" : "") + " · " + Math.round(km).toLocaleString() + " km"; sub = seats.length ? `${nodeShaped(seats[0])}/8 shaped` : ""; }
    drawNodeGlyph(g, mk.node, mx, my, 11, alpha * (microSel < 0 || nodes.includes(microSel) ? 1 : 0.6), 0);
    g.globalAlpha = alpha; g.font = `600 ${small ? 10 : 11}px ${MFONT}`; g.textBaseline = "middle";
    const tw = Math.max(g.measureText(label2).width, sub ? g.measureText(sub).width : 0) + 12;
    let lx = mx + 16, align = "left"; if (lx + tw > F.w - 6){ lx = mx - 16; align = "right"; }
    let by = Math.min(F.h - safeBot - 20, Math.max(safeTop + 12, my));
    for (const q of placed) if (Math.abs(q.y - by) < 38 && Math.abs(q.x - mx) < 300) by = q.y + 38;
    placed.push({ x: mx, y: by });
    g.fillStyle = "rgba(10,14,22,0.8)"; const bx = align === "left" ? lx - 5 : lx - tw + 5; g.fillRect(bx, by - 10, tw, sub ? 30 : 18); MBOXES.push([bx, by - 10, bx + tw, by + (sub ? 20 : 8)]);
    g.fillStyle = MK_COL[seats.length ? 4 : ci.length ? 5 : 3]; g.textAlign = align; g.fillText(label2, lx, by - 1);
    if (sub){ g.fillStyle = "rgba(220,228,240,0.8)"; g.font = `10px ${MFONT}`; g.fillText(sub, lx, by + 12); }
    g.globalAlpha = 1;
  }
}
// device tags, greedily: the selected device first, then the story's actors, then nearest the centre;
// a tag that would overlap a placed tag (or a story name tag) is left out, and while the story
// dims the rest of the street their tags are left out too
const MTAGS = [], MBOXES = []; // MBOXES: the off-patch marker cards drawn this frame
function microTags(g, F, boxes, sel, dimOf){
  if (!MTAGS.length) return;
  const pri = (t) => (t.i === sel ? -2e9 : 0) + (microStoryDim && microStoryDim.has(t.i) ? -1e9 : 0) + Math.hypot(t.x - F.w / 2, t.y - F.h / 2);
  const list = MTAGS.filter(t => dimOf(t.i) >= 1).sort((a, b) => pri(a) - pri(b));
  g.textAlign = "left"; g.textBaseline = "middle"; g.fillStyle = "rgba(230,236,245,0.85)";
  for (const t of list){
    g.font = t.font; const w = g.measureText(t.tag).width, h = 12, b = [t.x - 1, t.y - h / 2, t.x + w + 1, t.y + h / 2];
    if (b[2] < 0 || b[0] > F.w || b[3] < 0 || b[1] > F.h) continue;
    if (boxes.some(q => b[0] < q[2] && b[2] > q[0] && b[1] < q[3] && b[3] > q[1])) continue;
    boxes.push(b); g.globalAlpha = t.a; g.fillText(t.tag, t.x, t.y);
  }
  g.globalAlpha = 1;
}
function drawStoryTags(g, F, alpha){
  if (!microStory || !MSTORY.actors || alpha <= 0 || F.k / EARTH_M < 0.3) return MBOXES.slice();
  const names = ["Alice's phone", "Alice's home node", "Bob's phone", "Bob's home node"];
  g.font = `600 11px ${MFONT}`; g.textAlign = "center"; g.textBaseline = "bottom";
  const done = [], boxes = MBOXES.slice();
  MSTORY.actors.forEach((i, k) => {
    if (!M.vis[i]) return; const x = M.sx[i]; let y = M.sy[i] - 12; if (x < 0 || y < 0 || x > F.w || y > F.h) return;
    for (const q of done) if (Math.abs(q[0] - x) < 120 && Math.abs(q[1] - y) < 17) y = q[1] - 18;
    const tw = g.measureText(names[k]).width + 10;
    // a name tag under a marker card moves up past it
    for (let r = 0; r < 6; r++){ const q = MBOXES.find(c => x - tw / 2 < c[2] && x + tw / 2 > c[0] && y - 15 < c[3] && y + 1 > c[1]); if (!q) break; y = q[1] - 2; }
    done.push([x, y]); g.fillStyle = "rgba(10,14,22,0.75)"; g.fillRect(x - tw / 2, y - 15, tw, 16);
    g.fillStyle = k < 2 ? "#ffd59a" : "#a9e3ff"; g.fillText(names[k], x, y); boxes.push([x - tw / 2, y - 15, x + tw / 2, y + 1]);
  });
  return boxes;
}
function drawScaleBar(g, F, ppm){
  if (ppm < 0.02) return;
  const target = 110 / ppm; const p10 = Math.pow(10, Math.floor(Math.log10(target))); const m = [1, 2, 5, 10].map(k => k * p10).filter(v => v <= target).pop() || p10;
  const L = m * ppm, x = F.w - 24 - L, y = F.h - 22;
  g.strokeStyle = "rgba(230,236,245,0.85)"; g.lineWidth = 2; g.beginPath(); g.moveTo(x, y - 5); g.lineTo(x, y); g.lineTo(x + L, y); g.lineTo(x + L, y - 5); g.stroke();
  g.font = `11px ${MFONT}`; g.fillStyle = "rgba(230,236,245,0.9)"; g.textAlign = "center"; g.textBaseline = "bottom";
  g.fillText(m >= 1000 ? (m / 1000) + " km" : m + " m", x + L / 2, y - 6);
}
// ---- the chip that says what you are looking at
const pad2 = (n) => String(n).padStart(2, "0"); // the street clock's two-digit fields
// evidence:scan
function updateFocalChip(F){
  const chip = $("focal-chip"); if (!chip) return;
  const show = zoom >= FOCAL_ZOOM * 0.6;
  chip.classList.toggle("panel-hidden", !show); if (!show) return;
  if (!microOk()){ chip.innerHTML = "<b>R5 focal tier</b> needs WebAssembly: not available in this browser"; return; }
  if (!M){ chip.innerHTML = microNoLand ? "<b>R5 focal tier</b> · this tile is not wholly settled land (sea, coast or empty): streets are built only on whole land tiles" : "<b>R5 focal tier</b> · zoom in to build this tile's neighbourhood"; return; }
  if (chip._t && performance.now() - chip._t < 250) return;
  if (chip.contains(document.activeElement)) return; // a figure focused from the keyboard keeps its focus
  chip._t = performance.now();
  const m = M.meta, hh = m[1], hm = pad2(Math.floor(hh)) + ":" + pad2(Math.floor((hh % 1) * 60));
  const kinds = [27, 28, 29, 30, 31, 32].map(k => m[k]), dclass = MDCLASS[m[2] | 0];
  const ppm = F.k / EARTH_M; const scale = ppm > 1.5 ? "street" : ppm > 0.25 ? "neighbourhood" : ppm > 0.02 ? "district" : "region";
  if (F.w < 800){
    chip.innerHTML = `<b>R5 1:1</b> · ${dclass} · ${fig("micro-generated", Math.round(m[4]).toLocaleString() + "/km² · " + kinds.reduce((a, b) => a + b, 0) + " devices")} · ${hm} ×<span id="ts-v" data-evidence="input">${microTimeScale}</span> <button class="chip-btn" id="ts-dn" type="button" title="slower">−</button><button class="chip-btn" id="ts-up" type="button" title="faster">+</button> · bootstrap <button class="chip-btn" id="bs-tg" type="button" title="${BS_TITLE[microBootstrap]} (tap for the next release)">${BS_SHORT[microBootstrap]}</button> · tap a device`;
  } else
  chip.innerHTML = `<b>R5 focal 1:1</b> · ${dclass} · ` +
    `${fig("micro-generated", Math.round(m[4]).toLocaleString() + " people/km² · " + Math.round(2 * m[3]) + " m patch · " + kinds[0] + " phones, " + kinds[1] + " home nodes, " + kinds[2] + " laptops, " + kinds[3] + " hosted servers, " + kinds[4] + " seats" + (kinds[5] ? ", " + kinds[5] + " CI" : ""))} · ` +
    `local ${hm} · sim ×<span id="ts-v" data-evidence="input">${microTimeScale}</span> <button class="chip-btn" id="ts-dn" type="button" title="slower">−</button><button class="chip-btn" id="ts-up" type="button" title="faster">+</button> · ${scale} scale` +
    `<div class="chip-sub">rows: ${fig("micro-activity", m[10] + " sent · " + m[11] + " direct (" + m[12] + " shaped) · " + m[13] + " relayed · " + m[15] + " held at a mailbox · " + m[16] + " handed over · punches " + m[20] + "/" + m[19] + " up")} · tap a device</div>` +
    `<div class="chip-sub">model's reading: replication rounds ${fig("micro-activity", (m[37] | 0) + " served · " + (m[38] | 0) + " cadences where an unclaimed device announced with no seat as peer")} · bundle bootstrap (edge #881) <button class="chip-btn" id="bs-tg" type="button" title="${BS_TITLE[microBootstrap]} (tap for the next release)">${BS_LABEL[microBootstrap]}</button>` +
    (microStory ? "" : ` · seats: <button class="chip-btn" id="ds-tg" type="button" title="production's shape is one seat; the design case is three seats, k = 2">${microDesign ? '<span class="inp">design case: 3, k=2</span>' : '<span class="inp">production: 1</span>'}</button>`) + `</div>`;
  $("ts-dn").onclick = () => { microTimeScale = Math.max(1, Math.round(microTimeScale / 2)); chip._t = 0; };
  $("ts-up").onclick = () => { microTimeScale = Math.min(480, microTimeScale * 2); chip._t = 0; };
  const bs = $("bs-tg"); if (bs) bs.onclick = () => { microBootstrap = [2, 0, 1][microBootstrap]; wasm.micro_cmd(10, 0, microBootstrap ? 1 : 0); chip._t = 0; renderNodeCard(); };
  const ds = $("ds-tg"); if (ds) ds.onclick = () => { microDesign = !microDesign; M = null; microSel = -1; chip._t = 0; renderNodeCard(); microFreeRoam(); };
}
// evidence:end
// ---- the node card: what one device is, from the sim's own state
function microPick(px, py){
  if (!M || !M.sx || !microVisible()) return -1;
  let best = -1, bd = 16 * 16;
  if (M.markers) for (const mk of M.markers){ const d = (mk.x - px) ** 2 + (mk.y - py) ** 2; if (d < bd){ bd = d; best = mk.node; } }
  for (let i = 0; i < M.n; i++){
    if (!(nodeFlags(i) & NF.present) || !M.vis[i]) continue;
    const d = (M.sx[i] - px) ** 2 + (M.sy[i] - py) ** 2; if (d < bd){ bd = d; best = i; }
  }
  return best;
}
// while a node or seat card is open the story sheet folds to its header (body.card-open), so it never covers the card
{ const c = document.getElementById("node-card"); if (c && window.MutationObserver) new MutationObserver(() => { document.body.classList.toggle("card-open", !c.classList.contains("panel-hidden")); if (typeof sfSheet === "function") sfSheet(); }).observe(c, { attributes: true, attributeFilter: ["class"] }); }
// evidence:scan
function renderNodeCard(){
  const card = $("node-card"); if (!card) return;
  if (microSel < 0 || !M || microSel >= M.n){ card.classList.add("panel-hidden"); return; }
  // the same device's card holds still while it is pointed at or a figure in it has focus, so a figure
  // can be clicked, tapped or tabbed to; it refreshes again when the pointer or focus leaves
  if (!card.classList.contains("panel-hidden") && card._sel === microSel && (card.matches(":hover") || (card.contains(document.activeElement) && document.activeElement.hasAttribute("data-claim")))) return;
  card._sel = microSel;
  const i = microSel, f = nodeFlags(i), kind = nodeKind(i), cls = nodeClass(i), act = nodeAct(i);
  const hh = M.nu[i * 8 + 2], h0 = M.nu[i * 8 + 3], h1 = M.nu[i * 8 + 4], holding = M.nu[i * 8 + 5], pending = M.nu[i * 8 + 6];
  const router = hh < 4294967295 ? MROUTER[M.hh[hh * 7 + 4] | 0] : null;
  let reach;
  if (kind === 0 && !(f & NF.unmetered)) reach = (f & NF.punchable) ? "cellular, carrier NAT a punch can traverse (FRONTIER share)" : "cellular, carrier-grade NAT: no direct path in";
  else if (f & NF.offPatch) reach = kind === 5 ? "hosted runner: dials out, no inbound listener (MODEL)" : "public listener in a data centre";
  else reach = router + (kind === 0 ? " (home Wi-Fi)" : "");
  const shapesWhy = kind === 3 || kind === 4 ? "on by default (server/seat)" : (f & NF.optIn) ? "on: the person opted in at this class" : "off by default; accepts a shaped link when a peer asks";
  let relayWhy;
  if (f & NF.relay) relayWhy = "offering relay";
  else if (f & NF.clientOnly) relayWhy = "no: client-only (every v45 phone), no slot to give";
  else if (!(f & NF.deviceRelays)) relayWhy = "no: a phone relays only on unmetered power";
  else if (!(f & NF.relayPref)) relayWhy = "no: owner turned relay off";
  else if (!(f & NF.online)) relayWhy = "no: not online";
  else relayWhy = "no: not reachable without the canonical";
  const rows = [];
  for (let l = 0; l < M.links.length; l += 4){
    const a = M.links[l], b = M.links[l + 1]; if (a !== i && b !== i) continue;
    const o = a === i ? b : a, pk = M.links[l + 2], k = pk & 0xff, sc = (pk >>> 8) & 0xff, bound = (pk >>> 16) & 1, asked = (pk >>> 17) & 1;
    const opener = (pk >>> 18) & 1 ? a : b, by = opener === i ? "opened by this node" : "opened by #" + opener;
    rows.push(`<li><span class="lk lk${k}${sc ? " shaped" : ""}"></span>${MLINK[k]} ↔ ${MK[nodeKind(o)]} #${o} (${by}): ${sc ? "<b>shaped " + MCLASS[sc] + "</b> " + fig("budget-class-rates", MCLASS_BPS[sc].toLocaleString() + " B/s") + (asked ? " (asked)" : "") : bound ? fig("shaped-link-bound", "unshaped: the bound of 8 was full") : "unshaped (ordinary)"}</li>`);
  }
  for (let p = 0; p < M.pun.length; p += 2) if (M.pun[p] === i || M.pun[p + 1] === i){ const o = M.pun[p] === i ? M.pun[p + 1] : M.pun[p]; rows.push(`<li><span class="lk lkp"></span>punch in progress with ${MK[nodeKind(o)]} #${o} (${fig("punch-budget", "20 s budget")})</li>`); }
  const seatKm = (s) => { if (s >= 4294967295) return "–"; const site = M.nu[s * 8 + 7] & 0xffffff; return `#${s} (${fig("micro-generated", Math.round(M.sites[site * 4 + 2]).toLocaleString() + " km")})`; };
  const cnt = M.cnt || [], rounds = cnt[i * 2] | 0, ann = cnt[i * 2 + 1] | 0;
  let repl = "";
  if (kind !== 4){
    if (f & NF.claimed) repl = `claimed: its seats are replication peers through consent · ${fig("micro-activity", rounds + " rounds served")}`;
    else if (f & NF.served) repl = `unclaimed, bundle bootstrap on (${BS_LABEL[microBootstrap]}): the bundle's seats serve it · ${fig("micro-activity", rounds + " rounds served")}`;
    else repl = `unclaimed: announces (${fig("micro-activity", String(ann))}) on the link it opened, but no seat opens a round for it (no owner, no consent; bundle bootstrap off, as on v44) · ${fig("micro-activity", rounds + " served")}`;
  }
  let capHtml = "";
  if (kind === 4 && wasm && wasm.view_seat_fleet){
    const here = seatRounders(i), sizes = [here, 1000, 10000, 100000];
    const tr = sizes.map((d, j) => `<tr><td>${fig("seat-load-model", j ? fmtN(d) : d + " (this patch)")}</td><td>${fig("seat-load-model", seatCell(seatFleet(d, 0)))}</td><td>${fig("seat-load-model", seatCell(seatFleet(d, 4)))}</td></tr>`).join("");
    const lk = (sc) => { const w = seatFleet(here, sc); return w ? (KEY_PLANE[w.plane - 2] || "a key") + " plane, ρ " + ivRho(w.lock) + " over " + fmtN(w.subjects) + " subjects" : "–"; };
    capHtml = `<div class="card-links"><span>${fig("seat-load-model", "seat load of one 4-core v45.0.0 seat (edge release/45.0.0 at 8e46165)")} · MODEL with its interval (outer bounds over the input intervals, not a confidence level)</span>
      ${microBootstrap !== 1 ? `<div class="kv"><span>this street runs ${BS_LABEL[microBootstrap]}; the calibrated seat model covers v45.0.0 only, so these rows are v45.0.0's</span></div>` : ""}
      <div class="kv"><span>includes today's measured peer load at seat-1 as a fixed baseline (MEASURED round mix, MODEL scaling)</span></div>
      <table class="cap"><tr><th>devices</th><th>publish set stays (${fig("publish-set-growth", "g 0")})</th><th>grows with the fleet (${fig("publish-set-growth", "g 1")})</th></tr>${tr}</table>
      <div class="kv"><span>${fig("publish-set-growth", "which column holds is not known: how the server's publish set grows with the fleet is not measured")}</span></div>
      <div class="kv"><span>${fig("seat-knee", "largest fleet of v45.0.0 devices rounding with one 4-core seat under 70% of it")}</span><b>${fig("seat-knee", seatKneeText())} (MODEL with its interval, outer bounds over the input intervals; ${fig("publish-set-growth", "which of the two holds is not known")})</b></div>
      <div class="kv cap-which"><span><span class="fig" data-claim="pull-permit-provisional">v45's figure is PROVISIONAL in two terms: how long a Pull holds its sweep permit (about 2 ms, derived from a key-plane refill, not measured on a Pull), and the Pull's own cost (MEASURED on a debug build; the release rerun is pending)</span>. ${fig("edge-source-read", "The listing cache and read budget are built and on by default at 8e46165; only their timings are MODEL.")}</span></div>
      <div class="kv"><span>key-plane locks, ${fig("micro-activity", Math.round(here).toLocaleString() + " devices")} (MODEL: each plane's listing holds one lock; ${fig("key-plane-refill", "the 10-minute TTL now revalidates at most 256 reads a listing, so no listing holds the lock for a whole refill")}; Key and occurrence walk thousands of subjects, ${fig("global-subjects-at-seat", "the Global planes every key the seat holds (8,698 at seat-1)")})</span><b>${fig("seat-lock-model", "busiest at g 0: " + lk(0) + "; at g 1: " + lk(4) + " (one server, cores do not help)")}</b></div>
      <div class="kv"><span>seat-1 measured</span><b><span class="fig" data-claim="seat1-cpu-quiet">0.65 cores</span>, <span class="fig" data-claim="seat1-att-queries">~329 attestation queries/s (quiet 60 s window)</span></b></div></div>`;
  }
  const state = !(f & NF.present) ? "not present" : !(f & NF.awake) ? "asleep" : (f & NF.online) ? "online" : "resuming";
  card.innerHTML = `<div class="panel-head"><b style="color:${MK_COL[kind]}">${MK[kind]} #${i}</b><button class="panel-close" id="close-card" type="button" title="Close">✕</button></div>
  <div class="kv"><span>doing</span><b>${MACT[act]}</b></div>
  <div class="kv"><span>state</span><b>${state}${kind === 0 ? " · " + (f & NF.unmetered ? "Wi-Fi" : "cellular") + (f & NF.charging ? " · charging" : " · battery") : ""}</b></div>
  <div class="kv"><span>budget class (CC 5.4.3)</span><b>${MCLASS[cls]}${kind === 4 ? " (to seats; server to others)" : ""}</b></div>
  <div class="kv"><span>shaped links</span><b>${fig("micro-activity", String(nodeShaped(i)))} of ${fig("shaped-link-bound", "8")} · ${shapesWhy}</b></div>
  <div class="kv"><span>reachability</span><b>${reach}</b></div>
  <div class="kv"><span>relay</span><b>${relayWhy}</b></div>
  <div class="kv"><span>homes on (${fig("micro-activity", "k=" + (homesOf(i).length || "–"))}${microDesign ? ", design case" : ""})</span><b>${kind === 4 ? "is a canonical" : homesOf(i).map(seatKm).join(" · ") || "–"}</b></div>
  ${repl ? `<div class="kv"><span>replication (model's reading)</span><b>${repl}${kind !== 3 && kind !== 5 ? ` <button class="chip-btn" id="claim-tg" type="button">${f & NF.claimed ? "unclaim" : "claim"}</button>` : ""}</b></div>` : ""}
  <div class="kv"><span>edge version</span><b>${f & NF.old ? "older (pre-v44: never proposes a direct path)" : "current (v45)"}</b></div>
  ${holding || pending || (f & NF.mailbox) ? `<div class="kv"><span>mailbox</span><b>${f & NF.mailbox ? "can hold mail · " : ""}${fig("micro-activity", "holding " + holding + " · waiting for it " + pending)}</b></div>` : ""}
  ${capHtml}
  <div class="card-links"><span>links now (${fig("micro-activity", String(rows.length))})</span><ul>${rows.join("") || "<li>none: " + (state === "asleep" ? "suspend closed every link" : "no traffic") + "</li>"}</ul></div>`;
  card.classList.remove("panel-hidden");
  $("close-card").onclick = () => { microSel = -1; renderNodeCard(); };
  const ct = $("claim-tg"); if (ct) ct.onclick = () => { wasm.micro_cmd(9, i, f & NF.claimed ? 0 : 1); microRead(); renderNodeCard(); };
}
// evidence:end
let pickDown = null;
cv.addEventListener("mousedown", e => { pickDown = [e.clientX, e.clientY, performance.now()]; });
cv.addEventListener("mouseup", e => {
  if (!pickDown) return; const [x, y, t] = pickDown; pickDown = null;
  if (Math.hypot(e.clientX - x, e.clientY - y) > 5 || performance.now() - t > 500) return;
  const i = microPick(e.clientX, e.clientY); microSel = i; renderNodeCard(); if (i >= 0) microGlide = 0;
});
cv.addEventListener("touchstart", e => { if (e.touches.length === 1) pickDown = [e.touches[0].clientX, e.touches[0].clientY, performance.now()]; else pickDown = null; }, { passive: true });
cv.addEventListener("touchend", e => {
  if (!pickDown || e.changedTouches.length !== 1) return; const [x, y, t] = pickDown; pickDown = null; const c = e.changedTouches[0];
  if (Math.hypot(c.clientX - x, c.clientY - y) > 8 || performance.now() - t > 500) return;
  const i = microPick(c.clientX, c.clientY); if (i >= 0 || microSel >= 0){ microSel = i; renderNodeCard(); }
}, { passive: true });
setInterval(() => { if (microSel >= 0) renderNodeCard(); }, 300);

// ---- the street-scale story (acts 1 and 2): a script of what happens to whom; the rules decide how
const MSTORY = { actors: null, fired: new Set(), outcome: null, route1: null, ci: -1, lat: 0, lon: 0 };
function storyPinStart(ap, ah, bp, bh){ wasm.micro_cmd(1, ap, 2); wasm.micro_cmd(1, bp, 2); wasm.micro_cmd(1, ah, 3); wasm.micro_cmd(1, bh, 3); } // phones asleep on Wi-Fi; home nodes up
function storyMicroWorld(){
  // the story's street is a leaf holon of the street tiling near Zurich, the same world free roam
  // builds there, so its neighbours tile around it out to the acuity ring: the first nearby leaf whose
  // layout is a residential street with two households a punch can join (found once)
  if (!microOk()) return false;
  if (tlReady()) return storyMicroLeaf();
  const build = (T) => microBuild(T.lat, T.lon, T.dens, false, T.s1, T.s2, 0.62, "story");
  if (MSTORY.tile){ build(MSTORY.tile); if (wasm.micro_cmd(6, 0, 0) === 1){ const a = new Uint32Array(mem.buffer, wasm.micro_buf_ptr(10) >>> 0, 4).slice(); storyPinStart(...a); } return true; }
  const c0 = cellAt(47.3769, 8.5417); const dens = c0 >= 0 ? cellDensity(c0) : 1200;
  const ti0 = Math.floor(47.3769 / TILE_DEG), tj0 = Math.floor(8.5417 / TILE_DEG); let fallback = null;
  for (let r = 0; r < 81; r++){
    const ti = ti0 + (r % 9) - 4, tj = tj0 + Math.floor(r / 9) - 4;
    const T = { lat: (ti + 0.5) * TILE_DEG, lon: (tj + 0.5) * TILE_DEG, dens, s1: hash32(ti, tj), s2: hash32(tj ^ 0x5bd1e995, ti) };
    build(T);
    const cls = M.meta[2] | 0;
    if ((cls === 1 || cls === 2) && wasm.micro_cmd(6, 0, 0) === 1){
      const [ap, ah, bp, bh] = new Uint32Array(mem.buffer, wasm.micro_buf_ptr(10) >>> 0, 4).slice();
      storyPinStart(ap, ah, bp, bh);
      // the story follows a pair whose two routers a punch traverses (the sim decides that, per pair of networks)
      if (wasm.micro_cmd(8, ap, bp) === 1){ MSTORY.tile = T; return true; }
      if (!fallback) fallback = T;
    }
  }
  if (fallback){ MSTORY.tile = fallback; return storyMicroWorld(); } // no pair nearby punches: tell the relayed story instead
  return false;
}
function storyMicroLeaf(){
  const pin = () => { if (wasm.micro_cmd(6, 0, 0) !== 1) return null; const a = new Uint32Array(mem.buffer, wasm.micro_buf_ptr(10) >>> 0, 4).slice(); storyPinStart(...a); return a; };
  const build = (leaf) => microBuildLeaf(leaf, 0.62, "h" + HOLON.pack(leaf) + ":story");
  if (MSTORY.leaf){ build(MSTORY.leaf); pin(); return true; }
  // candidates: the distinct land leaves under a 9 by 9 lattice around the city centre, nearest first
  const T = tlTree(), L = tlLand(), lat0 = 47.3769, lon0 = 8.5417, l0 = T.leafAt(lat0, lon0);
  const st = HOLON.STEP[l0.level] / HOLON.UPD, seen = new Set(), cand = [];
  for (let r = 0; r < 81; r++){
    const a = (r % 9) - 4, q = Math.floor(r / 9) - 4, leaf = T.leafAt(lat0 + a * st, lon0 + q * st / Math.cos(lat0 * Math.PI / 180)), id = HOLON.pack(leaf);
    if (seen.has(id)) continue; seen.add(id);
    const b = HOLON.boundsDeg(leaf);
    if (T.budget(leaf) && L.squareOnLand(b[0], b[1], b[2] - b[0])) cand.push({ leaf, d: Math.hypot(a, q) });
  }
  cand.sort((x, y) => x.d - y.d); let fallback = null;
  for (const { leaf } of cand){
    build(leaf);
    const cls = M.meta[2] | 0, a = (cls === 1 || cls === 2) ? pin() : null;
    if (!a) continue;
    // two homes on the same street, not in the same building
    const [ax, ay] = actorXY(a[1]), [bx, by] = actorXY(a[3]); if (Math.hypot(bx - ax, by - ay) < 40){ if (!fallback) fallback = leaf; continue; }
    // the story follows a pair whose two routers a punch traverses (the sim decides that, per pair of networks)
    if (wasm.micro_cmd(8, a[0], a[2]) === 1){ MSTORY.leaf = leaf; return true; }
    if (!fallback) fallback = leaf;
  }
  if (fallback){ MSTORY.leaf = fallback; return storyMicroLeaf(); } // no pair nearby punches: tell the relayed story instead
  return false;
}
function actorXY(i){ return [M.nf[i * 4], M.nf[i * 4 + 1]]; }
function localToLL(x, y){ return [M.lat0 + y / 111320, M.lon0 + x / (111320 * Math.cos(M.lat0 * Math.PI / 180))]; }
function storyCam(x, y, z){ const [la, lo] = localToLL(x, y); storyTargetPitch = la * Math.PI / 180; storyTargetYaw = lo * Math.PI / 180; storyTargetZoom = z; }
const expLerp = (a, b, t) => a * Math.pow(b / a, Math.max(0, Math.min(1, t)));
function storyMicroSetup(act){
  microStory = true; MSTORY.fired = new Set(); MSTORY.outcome = null; MSTORY.beat = undefined; MSTORY.waitFrom = 0; MSTORY.beatT = undefined;
  if (act === 0){
    // the street is a leaf of the tiling: wait for the tiling's land and places (asked for at the
    // story's start), and only after a long wait tell it on a lone square tile
    MSTORY.actors = null;
    if (microOk() && !tlReady() && !MSTORY.waitDone){ MSTORY.waitTiled = performance.now(); return; }
    // rebuilt at the act's start, so it plays from the same state every time
    if (!storyMicroWorld()){ MSTORY.actors = null; return; }
    wasm.micro_cmd(6, 0, 0); const a = new Uint32Array(mem.buffer, wasm.micro_buf_ptr(10) >>> 0, 4).slice(); MSTORY.actors = Array.from(a);
    const [ap, ah, bp, bh] = MSTORY.actors;
    wasm.micro_quiet(1);
    microRead(); microStoryDim = new Set(MSTORY.actors); microTimeScale = 10;
    const [x, y] = actorXY(ah); MSTORY.home = [x, y];
  } else {
    if (!MSTORY.actors || !M) return;
    microStoryDim = null; wasm.micro_quiet(0); MSTORY.actors.forEach(i => wasm.micro_cmd(2, i, 0)); microTimeScale = 40;
  }
}
// "How we know" in the opening caption opens the Sources panel, which keeps every measured detail
document.addEventListener("click", (e) => { const a = e.target.closest && e.target.closest("[data-story-sources]"); if (!a) return; e.preventDefault();
  const f = $("foot"); if (f && f.classList.contains("panel-hidden")) togglePanel("foot", "btn-foot"); });
function storyMicroCaption(text){ if ($("story-line")) $("story-line").innerHTML = text; }
// the two street steps as beats: a beat runs its action, shows its caption, and stays for that
// caption's length (capSecs); a beat with a `wait` holds the previous caption until the sim answers
function storyMicroCtx(){
  const [ap, ah, bp, bh] = MSTORY.actors; const [axh, ayh] = actorXY(ah), [bxh, byh] = actorXY(bh);
  const F = camFrame(), frameZ = (m) => Math.min(zoomMax(), (Math.min(F.w, F.h) * 0.62) / (m * 0.86 * Math.min(F.w, F.h) / 2 / EARTH_M));
  return { ap, ah, bp, bh, axh, ayh, mid: [(axh + bxh) / 2, (ayh + byh) / 2], span: Math.hypot(bxh - axh, byh - ayh), frameZ,
    kmOf: (s) => Math.round(M.sites[(M.nu[s * 8 + 7] & 0xffffff) * 4 + 2]).toLocaleString() };
}
const camAlice = (c) => storyCam(c.axh, c.ayh, c.frameZ(70));
const camPair = (c, u) => storyCam(c.mid[0], c.mid[1], expLerp(c.frameZ(70), c.frameZ(c.span * 1.35 + 40), u === undefined ? 1 : Math.min(1, u * 2)));
const camOut = (c) => { const p = actorXY(c.ap); storyCam((p[0] + c.axh) / 2, (p[1] + c.ayh) / 2, c.frameZ(Math.max(90, Math.hypot(p[0] - c.axh, p[1] - c.ayh) * 1.4 + 60))); };
// evidence:scan
const helperPhrase = (c, hs) => hs.length === 1 ? `a helper server in a data centre ${fig("micro-generated", c.kmOf(hs[0]) + " km")} away` : `${fig("micro-generated", hs.length + " helper servers in data centres " + hs.map(c.kmOf).join(" and ") + " km")} away`;
// the street step frames the simulated neighbourhood, then pulls back until the neighbourhoods the
// tiling draws around it fill the view (600 m across: well inside the street-detail scale)
const STREET_WIDE_M = 600;
const streetNear = () => Math.max(160, M.half * 0.9), streetWide = () => Math.max(STREET_WIDE_M, M.half * 2.6);
const MICRO_BEATS = [
  [ // two neighbours connect
    { cap: () => `We zoom in on one neighbourhood near Zurich. <span class="fig" data-claim="all-simulated">Every phone and computer here is simulated</span>`,
      cam: (c, u) => storyCam(c.axh, c.ayh, u < 0.15 ? 2.25 : expLerp(2.25, c.frameZ(70), (u - 0.15) / 0.85)) },
    { run: (c) => wasm.micro_cmd(1, c.ap, 1 | 2), cap: () => "Alice's phone wakes up at home. Her home server is always on", cam: camAlice },
    { run: (c) => wasm.micro_cmd(3, c.ap, c.ah), cap: () => "Her phone finds the home server over Wi-Fi: a direct link, with no company in between", cam: camAlice },
    { cap: (c) => `Both also check in with ${helperPhrase(c, homesOf(c.ap))}, so friends can find them`, cam: camAlice },
    { run: (c) => { wasm.micro_cmd(1, c.bp, 1 | 2); wasm.micro_cmd(4, c.ap, c.bp); }, cap: (c) => `Bob lives ${fig("micro-generated", Math.round(c.span) + " m")} away. He and Alice swap contact details`, cam: camPair },
    { run: (c) => { microTimeScale = 6; MSTORY.route1 = wasm.micro_cmd(3, c.ap, c.bp); microRead(); MSTORY.fired.add("first");
        const e = M.ev.length - 7, via = M.ev[e + 3]; MSTORY.viaSeat = via < 4294967295 && nodeKind(via) === 4; },
      cap: () => MSTORY.route1 === 0 ? "Alice sends her first message. It goes straight to Bob's home, because his router accepts direct connections" :
        `Her first message passes through ${MSTORY.viaSeat ? "the helper" : "a member's home server"}, sealed so only Bob can read it. Their homes try connecting directly`,
      cam: camPair },
    { run: (c) => wasm.micro_cmd(3, c.ap, c.bp), wait: () => MSTORY.outcome !== null, waitMax: 8,
      cap: () => MSTORY.outcome === "ok" ? "It works: their two homes now talk directly, and the helper server steps out of the way" :
        MSTORY.outcome === "direct" ? "They already have a direct path, so no helper is needed" :
        MSTORY.outcome === "fail" ? "These two home networks can't connect directly, so messages keep going through the helper, still sealed" :
        "They are still trying for a direct path; meanwhile messages keep flowing through the helper",
      cam: camPair },
    { run: (c) => wasm.micro_cmd(1, c.ap, 1), cap: () => "Alice leaves home. Her phone switches to mobile data and loses the link to her home Wi-Fi", cam: camOut },
    { run: (c) => wasm.micro_cmd(1, c.ap, 0), cap: () => "In her pocket, the phone goes to sleep and closes its connections to save battery", cam: camOut },
    { run: (c) => wasm.micro_cmd(3, c.bp, c.ap), cap: () => "Bob replies. While her phone sleeps, her home server keeps the message for her", cam: camOut },
    { run: (c) => wasm.micro_cmd(1, c.ap, 1), cap: () => "On the tram she opens the app, and her home server passes Bob's message to her phone", cam: camOut }
  ],
  [ // the whole street, then the neighbourhoods tiled around it (STREET_WIDE_M across)
    { cap: () => "Now the whole street: phones sleep and wake, laptops close, and home servers stay on to hold messages",
      cam: (c) => storyCam(c.mid[0], c.mid[1], c.frameZ(streetNear())) },
    { cap: () => `<span class="fig" data-claim="shaped-link-bound">The helper evens out traffic for up to eight devices</span>, hiding their patterns; the rest still connect normally`,
      cam: (c, u) => storyCam(c.mid[0], c.mid[1], expLerp(c.frameZ(streetNear()), c.frameZ(streetWide()), u * 0.5)) },
    { run: () => { MSTORY.ci = wasm.micro_cmd(5, 0, 1); },
      cap: (c) => `A test computer in a data centre ${MSTORY.ci >= 0 ? fig("micro-generated", c.kmOf(MSTORY.ci) + " km") : "far"} away joins for a few minutes, runs its checks, then leaves`,
      cam: (c, u) => storyCam(c.mid[0], c.mid[1], expLerp(c.frameZ(streetNear()), c.frameZ(streetWide()), 0.5 + u * 0.5)) },
    { cap: () => `Pulling back: this is one street among millions. <span class="fig" data-claim="tier-cohorts">From up here the twin counts groups of devices, not each one</span>`,
      cam: (c, u) => storyCam(c.mid[0], c.mid[1], expLerp(c.frameZ(streetWide()), 30, u)) }
  ]
];
// evidence:end
// the step's length is the sum of its beats, re-planned as each beat starts (a wait adds its own time)
function storyMicroPlan(act, from, t0){
  const cfg = STORY_ACTS[act + STORY_LEAD], beats = MICRO_BEATS[act], c = storyMicroCtx();
  let d = t0; for (let i = from; i < beats.length; i++) d += i === from && MSTORY.beatT !== undefined ? MSTORY.beatT : capSecs(beats[i].cap(c));
  cfg.duration = Math.max(d, t0 + 0.1);
}
function storyMicroTick(act, t){
  if (MSTORY.waitTiled){
    const cfg = STORY_ACTS[act + STORY_LEAD];
    if (!tlReady() && performance.now() - MSTORY.waitTiled < 15000){ cfg.duration = Math.max(cfg.duration, t + 1); return; }
    MSTORY.waitTiled = 0; MSTORY.waitDone = true; storyMicroSetup(0); MSTORY.waitDone = false; storyActElapsed = 0; return;
  }
  if (!M || !MSTORY.actors) return;
  const beats = MICRO_BEATS[act], c = storyMicroCtx();
  if (MSTORY.beat === undefined){ MSTORY.beat = -1; MSTORY.beatEnd = 0; }
  storyPunchOutcome(c.ap, c.bp);
  const nx = MSTORY.beat + 1;
  if (nx < beats.length && t >= MSTORY.beatEnd){
    const b = beats[nx];
    if (b.wait && !MSTORY.waitFrom) MSTORY.waitFrom = t;
    const waiting = b.wait && !b.wait() && t - MSTORY.waitFrom < (b.waitMax || 6);
    if (!waiting){
      MSTORY.waitFrom = 0; MSTORY.beat = nx; MSTORY.beatStart = t;
      if (b.run) b.run(c);
      const txt = b.cap(c); storyMicroCaption(txt); MSTORY.beatT = capSecs(txt); MSTORY.beatEnd = t + MSTORY.beatT;
      storyMicroPlan(act, nx, t); MSTORY.beatT = undefined;
    } else storyMicroPlan(act, nx, t);
  }
  const cur = beats[Math.max(0, MSTORY.beat)];
  if (MSTORY.beat < 0) return;
  const u = Math.min(1, (t - MSTORY.beatStart) / Math.max(0.1, MSTORY.beatEnd - MSTORY.beatStart));
  cur.cam(c, u);
}
function storyPunchOutcome(ap, bp){
  if (MSTORY.outcome !== null || !MSTORY.fired.has("first") || M.pun.length !== 0) return;
  let punched = false; for (let l = 0; l < M.links.length; l += 4){ const a = M.links[l], b = M.links[l + 1]; if ((M.links[l + 2] & 0xff) === 4 && ((a === ap && b === bp) || (a === bp && b === ap))) punched = true; }
  MSTORY.outcome = punched ? "ok" : (MSTORY.route1 === 0 ? "direct" : "fail"); microTimeScale = 10;
}
function storyMicroEnd(){ microStory = false; microStoryDim = null; if (M) M.key = null; microTimeScale = 20; if (M && MSTORY.actors) MSTORY.actors.forEach(i => wasm.micro_cmd(2, i, 0)); if (wasm && wasm.micro_quiet) wasm.micro_quiet(0); }

let frame=0, lastTickTime=0, lastLoopNow=performance.now();
function loop(){
  const now=performance.now();
  if (typeof simClockAdvance==="function") simClockAdvance(now);
  const storyDt=Math.min(0.1,Math.max(0,(now-lastLoopNow)/1000)); lastLoopNow=now; // story time is real time, whatever the refresh rate
  if (storyActive) {
    if (!drag && !storyUserCam) {
      let dyaw = storyTargetYaw - yaw;
      while (dyaw < -Math.PI) dyaw += Math.PI * 2;
      while (dyaw > Math.PI) dyaw -= Math.PI * 2;
      yaw += dyaw * 0.05;
      pitch += (storyTargetPitch - pitch) * 0.05;
      if (storyCurrentAct === STORY_ACTS.length - 1 && !storyPaused) {
        storyTargetYaw += 0.0016;
      }
    }
    storyCamDist += (storyTargetCamDist - storyCamDist) * 0.05;
    if (!storyUserCam) targetZoom = storyTargetZoom;
    if (!storyPaused) {
      const prevSub = storySubIx(STORY_ACTS[storyCurrentAct], storyActElapsed);
      storyActElapsed += storyDt;
      const curCfg = STORY_ACTS[storyCurrentAct];
      if (prevSub !== storySubIx(curCfg, storyActElapsed)) updateStoryUI();
      if ($("story-progress-bar")) {
        const pct = Math.min(100, (storyActElapsed / curCfg.duration) * 100);
        $("story-progress-bar").style.width = pct.toFixed(1) + "%";
      }
      if (storyActElapsed >= curCfg.duration) {
        if (storyCurrentAct < STORY_ACTS.length - 1) {
          storySetupAct(storyCurrentAct + 1);
        } else {
          // the closing step stays up: the reader closes it, or plays the story again
          storyActElapsed = curCfg.duration; storyPaused = true; updateStoryUI();
        }
      }
    }
    if (storyActive) {
      storyTickAct(storyCurrentAct, storyDt);
      if (storyLive()) {
        if (playing && now-lastTickTime>=120){ lastTickTime=now; tick(); }
        decay();
      }
    }
  } else {
    if (!drag){
      velYaw*=0.93; velPitch*=0.93;
      if (Math.abs(velYaw)>0.00002) yaw+=velYaw;
      if (Math.abs(velPitch)>0.00002) pitch=Math.max(-1.4,Math.min(1.4,pitch+velPitch));
      const idle=now-lastTouchTime;
      // the slow turn when nobody touches the globe: only while the sim runs, never under reduced motion
      if (idle>3000 && zoom<3 && playing && !(typeof GFX!=="undefined" && GFX.reduced)){ const blend=Math.min(1.0,(idle-3000)/2000); yaw+=0.0016*blend; }
    }
    if (playing && now-lastTickTime>=120){ lastTickTime=now; tick(); if (typeof GFX!=="undefined") GFX.tickFrame=true; }
    decay();
  }
  { const z0=zoom; zoom*=Math.pow(targetZoom/zoom,0.12); zoomAnchorStep(z0,now); } // multiplicative: the same feel from orbit to a doorstep
  if (frame%4===0) pruneAndPackArcs(getTime());
  if (typeof tlFrame==="function") tlFrame(now);
  microFreeRoam(); microStep(now);
  // nothing moving, loading or running: skip the frame entirely (no GPU work while paused and still)
  if (typeof gfxNeedsDraw==="function" && !gfxNeedsDraw(now) && frame>30){ GFX.skipped++; frame++; requestAnimationFrame(loop); return; }
  const td0=performance.now();
  draw(); microDraw(); geoLabels(camFrame()); if (typeof LAYERS!=="undefined") LAYERS.draw2D(mctx, camFrame()); if (typeof tlMetrics==="function") tlMetrics();
  if (typeof GFX!=="undefined"){ GFX.drawn++; GFX.drawMs.push(performance.now()-td0); if (GFX.drawMs.length>240) GFX.drawMs.shift(); }
  frame++; requestAnimationFrame(loop);
}
async function build(){
  const wasPlaying=playing;
  playing=false; $("play").textContent="▶ run"; $("play").classList.remove("active");
  const n=currentSize; $("status").textContent="placing "+fmt(currentPop)+" people ("+fmt(currentPop*DEVICES_PER_PERSON)+" devices) on land density grid…"; await new Promise(r=>setTimeout(r,30));
  const t0=performance.now(); W=buildWorld(n); seedStoryNodes(); const t1=performance.now(); uploadWorld(); uploadPoints(); cum=0; lastActivity=null; activeBlocks=[]; activeArcs=[]; arcCount=0; arcsNeedUpload=true; pruneAndPackArcs(getTime()); computeSustainability();
  updateStatusLine((t1-t0)/1000);
  if (wasPlaying){ playing=true; $("play").textContent="❚❚ pause"; $("play").classList.add("active"); }
}
if ($("build")) $("build").onclick=build;
$("step").onclick=()=>{ if(W) tick(); };
$("play").onclick=(e)=>{ if(!W) return; playing=!playing; e.target.textContent=playing?"❚❚ pause":"▶ run"; e.target.classList.toggle("active",playing); };
$("eng").onclick=(e)=>{ useWasm=!useWasm; e.target.textContent="engine: "+(useWasm&&wasm?"WASM":"JS"); e.target.classList.toggle("on",useWasm&&!!wasm); avgMsPerAct=0; avgMsPerTick=0; if(W) tick(); };
$("arcsb").onclick=(e)=>{ showArcs=!showArcs; e.target.classList.toggle("on",showArcs); };
(async()=>{ initLand(); await loadWasm(); if(!wasm){ useWasm=false; $("eng").textContent="engine: JS"; $("eng").classList.remove("on"); } await build(); playing=true; $("play").textContent="❚❚ pause"; $("play").classList.add("active"); initStoryMode(); loop(); })().catch((e)=>{ console.warn("globe boot", e); const st=$("status"); if(st) st.textContent="the globe could not start here: "+(e&&e.message?e.message:String(e)); });
