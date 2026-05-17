(function () {
  const sankeyEl = document.getElementById("sankey-eoa");
  if (!sankeyEl) return;

  const chart = echarts.init(sankeyEl, null, { renderer: "canvas" });

  const filterEls = {
    user: document.getElementById("filter-user"),
    frontend: document.getElementById("filter-frontend"),
    reset: document.getElementById("filter-reset"),
  };

  let fullPayload = null;
  const filterState = { user: "__ALL__", frontend: "__ALL__" };

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

  function uniqSorted(arr) {
    return Array.from(new Set(arr)).sort((a, b) => String(a).localeCompare(String(b)));
  }

  function setOptions(el, options, current) {
    if (!el) return;
    el.replaceChildren();
    for (const v of ["__ALL__", ...options]) {
      const o = document.createElement("option");
      o.value = v;
      o.textContent = v === "__ALL__" ? "All" : v;
      if (v === current) o.selected = true;
      el.appendChild(o);
    }
  }

  // "User: EOA (Unlabeled)" → "User (Unlabeled)"
  function cleanName(s) {
    return s.replace(": EOA ", " ");
  }

  // Rename all node names and link sources/targets consistently
  function applyNameClean(payload) {
    const nodes = (payload.sankey.nodes || []).map((n) => ({ ...n, name: cleanName(n.name) }));
    const links = (payload.sankey.links || []).map((l) => ({
      ...l,
      source: cleanName(l.source),
      target: cleanName(l.target),
    }));
    return { ...payload, sankey: { nodes, links } };
  }

  // Extract only L1→L2 edges (source depth 0, target depth 1)
  function extractL1L2(payload) {
    const nodes = payload.sankey.nodes || [];
    const links = payload.sankey.links || [];
    const depthByName = new Map(nodes.map((n) => [n.name, n.depth]));

    const l1l2Links = links.filter(
      (l) => depthByName.get(l.source) === 0 && depthByName.get(l.target) === 1
    );
    const used = new Set();
    for (const l of l1l2Links) { used.add(l.source); used.add(l.target); }
    const l1l2Nodes = nodes.filter((n) => used.has(n.name));
    return { nodes: l1l2Nodes, links: l1l2Links };
  }

  function applyFilters(l1l2) {
    let links = l1l2.links;
    if (filterState.user !== "__ALL__") {
      links = links.filter((l) => l.source === filterState.user);
    }
    if (filterState.frontend !== "__ALL__") {
      links = links.filter((l) => l.target === filterState.frontend);
    }
    const used = new Set();
    for (const l of links) { used.add(l.source); used.add(l.target); }
    return { nodes: l1l2.nodes.filter((n) => used.has(n.name)), links };
  }

  function populateFilters(l1l2) {
    const userNames = uniqSorted(
      l1l2.nodes.filter((n) => n.depth === 0).map((n) => n.name)
    );
    const frontendNames = uniqSorted(
      l1l2.nodes.filter((n) => n.depth === 1).map((n) => n.name)
    );
    setOptions(filterEls.user, userNames, filterState.user);
    setOptions(filterEls.frontend, frontendNames, filterState.frontend);
  }

  function renderSankey(filtered) {
    const isReal = fullPayload && fullPayload.source === "cache";

    const depthColors = ["#c0392b", "#2c5f4a"];

    // Drive Sankey layout (bar width + vertical ordering) by USD volume when available,
    // so users are ranked by volume from top to bottom. Fall back to the raw `value`
    // (tx count from real data, or USD from demo) when volume is missing.
    const links = filtered.links.map((l) => ({
      source: l.source,
      target: l.target,
      value: l.volume_usd != null ? l.volume_usd : l.value,
      tx_count: l.value,
      volume_usd: l.volume_usd,
    }));

    // ECharts' default sankey layout (`layoutIterations` > 0) reshuffles nodes to
    // minimize edge crossings, which can put a smaller-volume node above a larger
    // one. To enforce strict volume-descending order we (1) compute each node's
    // total value from its links and (2) pre-sort `data` by depth then -value, and
    // then disable layoutIterations below so ECharts honors our input order.
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
      .sort((a, b) => (a.depth - b.depth) || (b._v - a._v));

    chart.setOption({
      backgroundColor: "transparent",
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
        formatter: function (p) {
          if (p.dataType === "edge") {
            const d = p.data;
            const txRaw = d.tx_count != null ? d.tx_count : d.value;
            const txs = isReal ? Math.round(txRaw).toLocaleString() : "$" + fmtM(txRaw);
            const vol = d.volume_usd != null ? "\nVOL  $" + fmtM(d.volume_usd) : "";
            return d.source + "\n> " + d.target + "\nTXS  " + txs + vol;
          }
          return p.name;
        },
      },
      series: [
        {
          type: "sankey",
          layout: "none",
          // 0 iterations → ECharts keeps the order of `data` exactly as provided,
          // which we sorted by descending volume above.
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
            fontWeight: "normal",
          },
          itemStyle: { borderWidth: 0 },
          data: nodes,
          links,
        },
      ],
    });

    // Click on a user node (depth 0) → open address modal
    chart.off("click");
    chart.on("click", (params) => {
      if (params.dataType === "node" && params.data.depth === 0) {
        openModal(params.data.name);
      }
    });
  }

  function renderTable(l1l2) {
    const byUser = new Map();
    for (const l of l1l2.links) {
      const cur = byUser.get(l.source) || { txs: 0, vol: 0 };
      cur.txs += l.value;
      cur.vol += l.volume_usd || 0;
      byUser.set(l.source, cur);
    }

    const rows = Array.from(byUser.entries()).sort((a, b) => b[1].vol - a[1].vol);
    const totalVol = rows.reduce((s, [, v]) => s + v.vol, 0);

    const tbody = document.getElementById("eoa-tbody");
    if (!tbody) return;
    tbody.replaceChildren();

    for (const [name, { txs, vol }] of rows) {
      const pct = totalVol > 0 ? (vol / totalVol) * 100 : 0;
      const tr = document.createElement("tr");
      tr.style.cursor = "pointer";
      tr.title = "Click to view addresses";
      tr.innerHTML = `
        <td>${name} <span style="font-size:0.45rem;color:var(--accent);margin-left:0.4em">▶</span></td>
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
    filterEls.user.addEventListener("change", () => {
      filterState.user = filterEls.user.value;
      refresh();
    });
    filterEls.frontend.addEventListener("change", () => {
      filterState.frontend = filterEls.frontend.value;
      refresh();
    });
    filterEls.reset.addEventListener("click", () => {
      filterState.user = "__ALL__";
      filterState.frontend = "__ALL__";
      if (fullPayload) {
        populateFilters(extractL1L2(fullPayload));
        refresh();
      }
    });
  }

  async function load() {
    try {
      const body = await OrderflowData.loadSummary();
      if (!body.ok || !body.data) {
        showError("API error: " + (body.error || res.statusText || "unknown"));
        return;
      }
      const d = body.data;

      const bar = document.getElementById("time-bar");
      if (d.block_time_range && bar) {
        const fmt = (s) => {
          const cleaned = String(s).trim().replace(/ UTC$/, "").split(".")[0];
          const dt = new Date(cleaned.replace(" ", "T") + "Z");
          if (Number.isNaN(dt.getTime())) return s;
          return dt.toLocaleDateString(undefined, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
          });
        };
        bar.textContent = fmt(d.block_time_range[0]) + "  –  " + fmt(d.block_time_range[1]);
      }

      fullPayload = applyNameClean(d);
      populateFilters(extractL1L2(fullPayload));
      refresh();
    } catch (e) {
      showError("Failed to load summary: " + e);
    }
  }

  // ── Modal ──────────────────────────────────────────────
  async function openModal(bucketName) {
    const frontend = filterState.frontend !== "__ALL__" ? filterState.frontend : null;

    const titleEl = document.getElementById("modal-title");
    titleEl.textContent = frontend
      ? bucketName + " → " + frontend.replace("Frontend: ", "")
      : bucketName;

    const modalBody = document.getElementById("modal-body");
    modalBody.replaceChildren();

    let addrs = [];
    try {
      addrs = await OrderflowData.lookupAddresses(bucketName, frontend);
    } catch (e) {
      console.error("Failed to load addresses:", e);
    }

    document.getElementById("modal-count").textContent = addrs.length + " addresses";

    for (const addr of addrs) {
      const span = document.createElement("span");
      span.className = "addr";
      span.textContent = addr;
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
    const addrs = Array.from(
      document.querySelectorAll("#modal-body .addr")
    ).map((el) => el.textContent);
    await navigator.clipboard.writeText(addrs.join("\n"));
    const btn = document.getElementById("modal-copy");
    btn.textContent = "COPIED!";
    setTimeout(() => (btn.textContent = "COPY ALL"), 1500);
  });
  // ── End Modal ──────────────────────────────────────────

  window.addEventListener("resize", () => chart.resize());
  wireFilters();
  load();
})();
