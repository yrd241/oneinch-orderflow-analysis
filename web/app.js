(function () {
  const sankeyEl = document.getElementById("sankey");
  const chart = echarts.init(sankeyEl, null, { renderer: "canvas" });
  const filterEls = {
    user: document.getElementById("filter-user"),
    frontend: document.getElementById("filter-frontend"),
    solver: document.getElementById("filter-solver"),
    mempool: document.getElementById("filter-mempool"),
    ofa: document.getElementById("filter-ofa"),
    builder: document.getElementById("filter-builder"),
    reset: document.getElementById("filter-reset"),
  };

  const FILTERS = [
    { key: "user", label: "User", depth: 0 },
    { key: "frontend", label: "Frontend", depth: 1 },
    { key: "solver", label: "Solver", depth: 2 },
    { key: "mempool", label: "Mempool", depth: 3 },
    { key: "ofa", label: "OFA", depth: 4 },
    { key: "builder", label: "Builder", depth: 5 },
  ];

  let fullPayload = null;
  const filterState = {
    user: "__ALL__",
    frontend: "__ALL__",
    solver: "__ALL__",
    mempool: "__ALL__",
    ofa: "__ALL__",
    builder: "__ALL__",
  };

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

  function buildDepthMap(nodes) {
    const m = new Map();
    for (const n of nodes) m.set(n.name, n.depth);
    return m;
  }

  function uniqSorted(arr) {
    return Array.from(new Set(arr)).sort((a, b) => String(a).localeCompare(String(b)));
  }

  function setOptions(selectEl, options, current) {
    if (!selectEl) return;
    const opts = ["__ALL__", ...options];
    selectEl.replaceChildren();
    for (const v of opts) {
      const o = document.createElement("option");
      o.value = v;
      o.textContent = v === "__ALL__" ? "All" : v;
      if (v === current) o.selected = true;
      selectEl.appendChild(o);
    }
  }

  function populateFilters(payload) {
    if (!payload || !payload.sankey) return;
    const nodes = payload.sankey.nodes || [];
    const byDepth = new Map();
    for (const { depth } of FILTERS) byDepth.set(depth, []);
    for (const n of nodes) {
      if (n.depth == null) continue;
      if (byDepth.has(n.depth)) byDepth.get(n.depth).push(n.name);
    }
    for (const f of FILTERS) {
      const options = uniqSorted(byDepth.get(f.depth) || []);
      setOptions(filterEls[f.key], options, filterState[f.key]);
    }
  }

  function applyFilters(payload) {
    const nodes = payload.sankey.nodes || [];
    const links = payload.sankey.links || [];
    const depthByName = buildDepthMap(nodes);

    const allowNode = (name) => {
      const d = depthByName.get(name);
      if (d == null) return true;
      for (const f of FILTERS) {
        if (d !== f.depth) continue;
        const chosen = filterState[f.key];
        if (chosen !== "__ALL__" && name !== chosen) return false;
      }
      return true;
    };

    const filteredLinks = links.filter((l) => allowNode(l.source) && allowNode(l.target));
    const used = new Set();
    for (const l of filteredLinks) {
      used.add(l.source);
      used.add(l.target);
    }
    const filteredNodes = nodes.filter((n) => used.has(n.name));
    return { nodes: filteredNodes, links: filteredLinks };
  }

  function renderSankey(payload) {
    const isReal = payload.source === "cache";

    const filtered = applyFilters(payload);

    const nodes = (filtered.nodes || []).map((n) => ({
      name: n.name,
      depth: n.depth,
    }));

    const links = (filtered.links || []).map((l) => ({
      source: l.source,
      target: l.target,
      value: l.value,
      volume_usd: l.volume_usd,
    }));

    // Pixel-game palette: one bold color per layer depth
    const depthColors = ["#1a1208", "#c0392b", "#6b4c2a", "#2c5f4a", "#1a3a5c"];
    const nodesWithColor = nodes.map((n) => ({
      ...n,
      itemStyle: { color: depthColors[n.depth] || "#1a1208" },
    }));

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
          fontSize: 11,
          color: "#f0ede4",
          lineHeight: 20,
        },
        formatter: function (p) {
          if (p.dataType === "edge") {
            const d = p.data;
            const txs = isReal ? d.value.toLocaleString() : "$" + fmtM(d.value);
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
          emphasis: { focus: "adjacency" },
          nodeAlign: "justify",
          nodeGap: 16,
          nodeWidth: 14,
          lineStyle: {
            color: "gradient",
            curveness: 0.5,
            opacity: 0.2,
          },
          label: {
            color: "#1a1208",
            fontSize: 11,
            fontFamily: "'Press Start 2P', monospace",
            fontWeight: "normal",
          },
          itemStyle: { borderWidth: 0 },
          data: nodesWithColor,
          links: links,
        },
      ],
    });
  }

  function wireFilters() {
    for (const f of FILTERS) {
      const el = filterEls[f.key];
      if (!el) continue;
      el.addEventListener("change", () => {
        filterState[f.key] = el.value;
        if (fullPayload) renderSankey(fullPayload);
      });
    }
    if (filterEls.reset) {
      filterEls.reset.addEventListener("click", () => {
        for (const f of FILTERS) filterState[f.key] = "__ALL__";
        if (fullPayload) {
          populateFilters(fullPayload);
          renderSankey(fullPayload);
        }
      });
    }
  }

  async function load() {
    try {
      const res = await fetch("/api/summary");
      const body = await res.json();
      if (!body.ok || !body.data) {
        showError("API error: " + (body.error || res.statusText || "unknown"));
        return;
      }
      const d = body.data;
      const bar = document.getElementById("time-bar");
      if (d.block_time_range) {
        // `block_time_range` is sourced from SQL as a UTC timestamp string.
        // Convert it into the browser's local time (date + time).
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
        const rangeText = fmt(d.block_time_range[0]) + "  –  " + fmt(d.block_time_range[1]);
        if (bar) bar.textContent = rangeText;
      }
      fullPayload = d;
      populateFilters(d);
      renderSankey(d);
    } catch (e) {
      showError("Failed to load /api/summary: " + e);
    }
  }

  window.addEventListener("resize", () => chart.resize());
  wireFilters();
  load();
})();
