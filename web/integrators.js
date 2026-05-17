(function () {
  const sankeyEl = document.getElementById("sankey-integrators");
  if (!sankeyEl) return;

  const chart = echarts.init(sankeyEl, null, { renderer: "canvas" });

  const filterEls = {
    recipient: document.getElementById("filter-recipient"),
    reset: document.getElementById("filter-reset"),
  };

  let fullPayload = null;
  let recipientsDetail = {};
  const filterState = { recipient: "__ALL__" };

  function fmtM(usd) {
    if (usd >= 1e9) return (usd / 1e9).toFixed(2) + "B";
    if (usd >= 1e6) return (usd / 1e6).toFixed(1) + "M";
    if (usd >= 1e3) return (usd / 1e3).toFixed(1) + "K";
    return usd.toFixed(0);
  }

  function showError(msg) {
    const p = document.createElement("p");
    p.className = "err";
    p.textContent = msg;
    document.body.replaceChildren(p);
  }

  const OTHER_RECIPIENT = "Recipient: …other";
  const RECIPIENT_PREFIX = "Recipient: ";

  /** Known feeRecipient → display name (lowercase address keys). */
  const FEE_RECIPIENT_BY_ADDR = {
    "0x8d413db42d6901de42b2c481cc0f6d0fd1c52828": "Coinbase Wallet",
    "0x39041f1b366fe33f9a5a79de5120f2aee2577ebc": "Rabby Wallet",
    "0x4a183b7ed67b9e14b3f45abfb2cf44ed22c29e54": "Zerion",
    "0xcd6b980029e6e6e0733ac8ec3e02be9410d09799": "Fly.trade",
    "0xb4f34d09124b8c9712957b76707b42510041ecbb": "SafePal",
  };

  function recipientDisplayName(sankeyLabel, detail) {
    const d = detail || recipientsDetail[sankeyLabel] || {};
    const wallet = d.wallet || FEE_RECIPIENT_BY_ADDR[(d.address || "").toLowerCase()];
    if (wallet) return wallet;
    if (sankeyLabel.startsWith(RECIPIENT_PREFIX)) {
      const rest = sankeyLabel.slice(RECIPIENT_PREFIX.length);
      if (rest.startsWith("0x")) return rest.slice(0, 6) + "…" + rest.slice(-4);
      return rest;
    }
    return sankeyLabel;
  }

  function volumeBySource(links) {
    const vol = new Map();
    for (const l of links) {
      vol.set(l.source, (vol.get(l.source) || 0) + (l.volume_usd != null ? l.volume_usd : 0));
    }
    return vol;
  }

  /** By volume high→low; …other always last (not alphabetical). */
  function sortRecipientNames(names, links) {
    const vol = volumeBySource(links);
    return Array.from(new Set(names)).sort((a, b) => {
      const aOther = a === OTHER_RECIPIENT ? 1 : 0;
      const bOther = b === OTHER_RECIPIENT ? 1 : 0;
      if (aOther !== bOther) return aOther - bOther;
      return (vol.get(b) || 0) - (vol.get(a) || 0);
    });
  }

  function setOptions(el, options, current) {
    if (!el) return;
    el.replaceChildren();
    for (const v of ["__ALL__", ...options]) {
      const o = document.createElement("option");
      o.value = v;
      o.textContent = v === "__ALL__" ? "All" : recipientDisplayName(v);
      if (v === current) o.selected = true;
      el.appendChild(o);
    }
  }

  function extractL1L2(payload) {
    const nodes = payload.sankey.nodes || [];
    const links = payload.sankey.links || [];
    const depthByName = new Map(nodes.map((n) => [n.name, n.depth]));
    const l1l2Links = links.filter(
      (l) => depthByName.get(l.source) === 0 && depthByName.get(l.target) === 1
    );
    const used = new Set();
    for (const l of l1l2Links) {
      used.add(l.source);
      used.add(l.target);
    }
    const l1l2Nodes = nodes.filter((n) => used.has(n.name));
    return { nodes: l1l2Nodes, links: l1l2Links };
  }

  function applyFilters(l1l2) {
    let links = l1l2.links;
    if (filterState.recipient !== "__ALL__") {
      links = links.filter((l) => l.source === filterState.recipient);
    }
    const used = new Set();
    for (const l of links) {
      used.add(l.source);
      used.add(l.target);
    }
    return { nodes: l1l2.nodes.filter((n) => used.has(n.name)), links };
  }

  function populateFilters(l1l2) {
    const names = l1l2.nodes.filter((n) => n.depth === 0).map((n) => n.name);
    setOptions(filterEls.recipient, sortRecipientNames(names, l1l2.links), filterState.recipient);
  }

  function renderSankey(filtered) {
    const depthColors = ["#c0392b", "#2c5f4a"];
    if (!filtered.links.length) {
      chart.clear();
      chart.setOption({
        backgroundColor: "transparent",
        title: {
          text: "No integrators",
          left: "center",
          top: "middle",
          textStyle: {
            color: "#1a1208",
            fontSize: 12,
            fontFamily: "'Press Start 2P', monospace",
          },
        },
      });
      return;
    }
    const links = filtered.links.map((l) => ({
      source: l.source,
      target: l.target,
      value: l.volume_usd != null ? l.volume_usd : l.value,
      tx_count: l.value,
      volume_usd: l.volume_usd,
    }));

    const nodeValue = new Map();
    for (const l of links) {
      nodeValue.set(l.source, (nodeValue.get(l.source) || 0) + l.value);
      nodeValue.set(l.target, (nodeValue.get(l.target) || 0) + l.value);
    }
    const nodes = filtered.nodes
      .map((n) => ({
        name: n.name,
        depth: n.depth,
        _v: nodeValue.get(n.name) || 0,
        itemStyle: { color: depthColors[n.depth] || "#1a1208" },
      }))
      .sort((a, b) => a.depth - b.depth || b._v - a._v);

    chart.setOption(
      {
        backgroundColor: "transparent",
        title: { show: false },
        tooltip: {
          trigger: "item",
          triggerOn: "mousemove",
          backgroundColor: "#1a1208",
          borderColor: "#c0392b",
          borderWidth: 2,
          padding: [10, 14],
          textStyle: {
            fontFamily: "'Press Start 2P', monospace",
            fontSize: 12,
            color: "#f0ede4",
            lineHeight: 20,
          },
          formatter(p) {
            if (p.dataType === "edge") {
              const d = p.data;
              const txs = Math.round(d.tx_count != null ? d.tx_count : d.value).toLocaleString();
              const vol = d.volume_usd != null ? "\nVOL  $" + fmtM(d.volume_usd) : "";
              const src = recipientDisplayName(d.source);
              return src + "\n> " + d.target + "\nTXS  " + txs + vol;
            }
            return recipientDisplayName(p.name);
          },
        },
        series: [
          {
            type: "sankey",
            layout: "none",
            layoutIterations: 0,
            emphasis: { focus: "adjacency" },
            nodeAlign: "justify",
            nodeGap: 16,
            nodeWidth: 14,
            lineStyle: { color: "gradient", curveness: 0.5, opacity: 0.25 },
            label: {
              color: "#1a1208",
              fontSize: 12,
              fontFamily: "'Press Start 2P', monospace",
              formatter: (params) => recipientDisplayName(params.name),
            },
            itemStyle: { borderWidth: 0 },
            data: nodes,
            links,
          },
        ],
      },
      true
    );

    chart.off("click");
    chart.on("click", (params) => {
      if (params.dataType === "node" && params.data.depth === 0) {
        openModal(params.data.name);
      }
    });
  }

  function renderTable(l1l2) {
    const byRec = new Map();
    for (const l of l1l2.links) {
      const cur = byRec.get(l.source) || { txs: 0, vol: 0 };
      cur.txs += l.value;
      cur.vol += l.volume_usd || 0;
      byRec.set(l.source, cur);
    }
    const rows = sortRecipientNames(
      Array.from(byRec.keys()),
      l1l2.links
    ).map((name) => [name, byRec.get(name)]);
    const totalVol = rows.reduce((s, [, v]) => s + v.vol, 0);
    const tbody = document.getElementById("integrators-tbody");
    if (!tbody) return;
    tbody.replaceChildren();
    for (const [name, { txs, vol }] of rows) {
      const pct = totalVol > 0 ? (vol / totalVol) * 100 : 0;
      const tr = document.createElement("tr");
      tr.style.cursor = "pointer";
      tr.title = "Click for address & tx hashes";
      tr.innerHTML = `
        <td>${recipientDisplayName(name)} <span style="font-size:0.45rem;color:var(--accent);margin-left:0.4em">▶</span></td>
        <td class="num">${Math.round(txs).toLocaleString()}</td>
        <td class="num">$${fmtM(vol)}</td>
        <td>
          <div class="bar-cell">
            <div class="bar" style="width:${Math.max(pct, 0.5).toFixed(1)}%"></div>
            <span style="font-size:0.5rem;color:var(--muted)">${pct.toFixed(1)}%</span>
          </div>
        </td>`;
      tr.addEventListener("click", () => openModal(name));
      tbody.appendChild(tr);
    }
  }

  function refresh() {
    if (!fullPayload) return;
    const l1l2 = extractL1L2(fullPayload);
    const filtered = applyFilters(l1l2);
    renderSankey(filtered);
    renderTable(filtered);
  }

  function wireFilters() {
    if (!filterEls.recipient || !filterEls.reset) {
      console.warn("integrators: filter controls missing from DOM");
      return;
    }
    filterEls.recipient.addEventListener("change", () => {
      filterState.recipient = filterEls.recipient.value;
      refresh();
    });
    filterEls.reset.addEventListener("click", () => {
      filterState.recipient = "__ALL__";
      if (fullPayload) {
        populateFilters(extractL1L2(fullPayload));
        refresh();
      }
    });
  }

  async function load() {
    try {
      const body = await OrderflowData.loadIntegrators();
      if (!body.ok || !body.data) {
        showError(body.error || "Run build_integrator_recipient_sankey.py first");
        return;
      }
      const d = body.data;
      recipientsDetail = d.recipients_detail || {};

      const bar = document.getElementById("time-bar");
      if (d.block_time_range && bar) {
        const fmt = (s) => {
          const cleaned = String(s).trim().replace(/ UTC$/, "").split(".")[0];
          const dt = new Date(cleaned.replace(" ", "T") + "Z");
          if (Number.isNaN(dt.getTime())) return s;
          return dt.toLocaleString(undefined, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
          });
        };
        bar.textContent = fmt(d.block_time_range[0]) + "  –  " + fmt(d.block_time_range[1]);
      }

      const meta = document.getElementById("meta-bar");
      if (meta && d.meta) {
        meta.textContent =
          `${d.meta.tx_with_fee || 0} / ${d.meta.tx_rows || 0} txs with fee recipient`;
      }

      fullPayload = d;
      populateFilters(extractL1L2(fullPayload));
      refresh();
    } catch (e) {
      showError("Failed to load: " + e);
    }
  }

  function openModal(label) {
    const detail = recipientsDetail[label] || {};
    const titleEl = document.getElementById("modal-title");
    titleEl.textContent = recipientDisplayName(label, detail) + " → 1inch Integrators";

    const modalBody = document.getElementById("modal-body");
    modalBody.replaceChildren();

    const wallet =
      detail.wallet || FEE_RECIPIENT_BY_ADDR[(detail.address || "").toLowerCase()];
    if (wallet) {
      const badge = document.createElement("div");
      badge.className = "addr-full";
      badge.style.marginBottom = "0.5em";
      badge.style.fontSize = "0.55rem";
      badge.textContent = wallet;
      modalBody.appendChild(badge);
    }

    if (detail.address) {
      const addr = document.createElement("div");
      addr.className = "addr-full";
      addr.textContent = detail.address;
      modalBody.appendChild(addr);
    }

    const hashes = detail.hashes || [];
    document.getElementById("modal-count").textContent =
      hashes.length + " tx" + (hashes.length === 1 ? "" : "s");

    for (const h of hashes) {
      const span = document.createElement("span");
      span.className = "addr";
      span.textContent = h;
      modalBody.appendChild(span);
    }

    document.getElementById("modal-backdrop").classList.add("open");
  }

  function closeModal() {
    document.getElementById("modal-backdrop").classList.remove("open");
  }

  document.getElementById("modal-close").addEventListener("click", closeModal);
  document.getElementById("modal-backdrop").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeModal();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeModal();
  });
  document.getElementById("modal-copy").addEventListener("click", async () => {
    const lines = Array.from(document.querySelectorAll("#modal-body .addr")).map((el) => el.textContent);
    const full = document.querySelector("#modal-body .addr-full");
    if (full) lines.unshift(full.textContent);
    await navigator.clipboard.writeText(lines.filter(Boolean).join("\n"));
    const btn = document.getElementById("modal-copy");
    btn.textContent = "COPIED!";
    setTimeout(() => (btn.textContent = "COPY ALL"), 1500);
  });

  window.addEventListener("resize", () => chart.resize());
  wireFilters();
  load();
})();
