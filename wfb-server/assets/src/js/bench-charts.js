/* global uPlot, htmx */

(function () {
  "use strict";

  function setLegendItems(legendRoot, items) {
    if (!legendRoot) return;
    legendRoot.replaceChildren();

    for (const item of items) {
      const row = document.createElement("span");
      row.className = "inline-flex items-center gap-2";

      const swatch = document.createElement("span");
      swatch.className = "h-2.5 w-2.5 rounded-full ring-1 ring-border";
      swatch.style.backgroundColor = item.color;
      swatch.setAttribute("aria-hidden", "true");

      const label = document.createElement("span");
      label.textContent = item.label;

      row.appendChild(swatch);
      row.appendChild(label);
      legendRoot.appendChild(row);
    }
  }

  function ensureTooltip(container) {
    let el = container.querySelector("[data-wfb-chart-tooltip]");
    if (el) return el;

    el = document.createElement("div");
    el.setAttribute("data-wfb-chart-tooltip", "");
    el.className = "wfb-chart-tooltip";
    el.style.display = "none";
    container.appendChild(el);
    return el;
  }

  function clamp(n, min, max) {
    return Math.max(min, Math.min(max, n));
  }

  function toFiniteNumber(v) {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  }

  function cssVar(name, fallback) {
    const v = getComputedStyle(document.documentElement).getPropertyValue(name);
    const trimmed = (v || "").trim();
    return trimmed || fallback;
  }

  function formatNumberCompact(n) {
    n = toFiniteNumber(n);
    if (n === null) return "";

    const abs = Math.abs(n);
    if (abs >= 1e9) return (n / 1e9).toFixed(1).replace(/\.0$/, "") + "B";
    if (abs >= 1e6) return (n / 1e6).toFixed(1).replace(/\.0$/, "") + "M";
    if (abs >= 1e3) return (n / 1e3).toFixed(1).replace(/\.0$/, "") + "K";
    return String(Math.round(n));
  }

  function formatBytesMb(mb) {
    mb = toFiniteNumber(mb);
    if (mb === null) return "";
    if (mb >= 1024) return (mb / 1024).toFixed(1).replace(/\.0$/, "") + " GB";
    return mb.toFixed(0) + " MB";
  }

  function formatMs(ms) {
    ms = toFiniteNumber(ms);
    if (ms === null) return "";
    if (ms >= 1000) return (ms / 1000).toFixed(2).replace(/\.0+$/, "") + " s";
    if (ms >= 10) return ms.toFixed(0) + " ms";
    return ms.toFixed(1).replace(/\.0$/, "") + " ms";
  }

  function parseChartData(container) {
    const el = container.querySelector(
      'script[type="application/json"][data-wfb-chart-data]'
    );
    if (!el) return null;

    try {
      const parsed = JSON.parse(el.textContent || "{}");
      if (!parsed || !Array.isArray(parsed.x)) return null;
      return parsed;
    } catch {
      return null;
    }
  }

  function initOne(container) {
    if (!container || container.dataset.wfbChartInit === "1") return;

    const root = container.querySelector("[data-wfb-chart-root]");
    if (!root) return;

    // Tooltip is positioned relative to the chart root.
    const rootPos = getComputedStyle(root).position;
    if (!rootPos || rootPos === "static") root.style.position = "relative";

    if (typeof uPlot === "undefined") {
      // uPlot wasn't loaded; don't hard-fail the page.
      return;
    }

    const data = parseChartData(container);
    if (!data) return;

    const measure = () => {
      const rect = root.getBoundingClientRect();
      const w = Math.floor(rect.width || 0);
      const h = Math.floor(rect.height || 0);
      return { width: w, height: h };
    };

    let { width, height } = measure();

    // When loaded via HTMX, layout can be 0x0 for a moment.
    // Defer init until the element has a stable size.
    if (width < 120 || height < 120) {
      if (typeof ResizeObserver !== "undefined") {
        const ro = new ResizeObserver(() => {
          const m = measure();
          if (m.width >= 120 && m.height >= 120) {
            ro.disconnect();
            container.__wfbInitRO = null;
            initOne(container);
          }
        });
        ro.observe(root);
        container.__wfbInitRO = ro;
      }
      return;
    }

    const gridStroke = cssVar("--border", "rgba(148,163,184,0.25)");
    const axisStroke = cssVar("--muted-foreground", "#94a3b8");

    const cRps = cssVar("--chart-1", "#6366f1");
    const cLat = cssVar("--chart-2", "#22d3ee");
    const cMem = cssVar("--chart-3", "#34d399");

    const opts = {
      width,
      height,
      padding: [12, 14, 10, 10],
      legend: { show: false },
      cursor: { drag: { setScale: false } },
      scales: {
        x: { time: false },
        rps: { auto: true },
        p99: { auto: true },
        mem: { auto: true },
      },
      axes: [
        {
          scale: "x",
          stroke: axisStroke,
          grid: { stroke: gridStroke, width: 1 },
          values: (u, ticks) =>
            ticks.map((v) => {
              const n = toFiniteNumber(v);
              return n === null ? "" : n.toFixed(0) + "s";
            }),
        },
        {
          scale: "rps",
          label: "RPS",
          stroke: axisStroke,
          grid: { stroke: gridStroke, width: 1 },
          values: (u, ticks) => ticks.map(formatNumberCompact),
        },
        {
          scale: "p99",
          side: 1,
          label: "P99",
          stroke: axisStroke,
          grid: { show: false },
          values: (u, ticks) => ticks.map((v) => formatMs(v).replace(/\s.*$/, "")),
        },
        {
          scale: "mem",
          side: 1,
          label: "Mem",
          stroke: axisStroke,
          grid: { show: false },
          values: (u, ticks) => ticks.map((v) => formatBytesMb(v).replace(/\s.*$/, "")),
        },
      ],
      series: [
        { label: "t" },
        {
          label: "RPS",
          scale: "rps",
          stroke: cRps,
          width: 2,
          points: { show: false, size: 5 },
          value: (u, v) => (isFinite(v) ? formatNumberCompact(v) + " req/s" : ""),
        },
        {
          label: "P99",
          scale: "p99",
          stroke: cLat,
          width: 2,
          points: { show: false, size: 5 },
          value: (u, v) => (isFinite(v) ? formatMs(v) : ""),
        },
        {
          label: "Mem",
          scale: "mem",
          stroke: cMem,
          width: 2,
          points: { show: false, size: 5 },
          value: (u, v) => (isFinite(v) ? formatBytesMb(v) : ""),
        },
      ],
    };

    const udata = [data.x, data.rps, data.p99_ms, data.mem_mb];

    const tooltip = ensureTooltip(root);
    const tooltipSeries = [
      { label: "RPS", color: cRps, fmt: (v) => (isFinite(v) ? formatNumberCompact(v) + " req/s" : "") },
      { label: "P99", color: cLat, fmt: (v) => (isFinite(v) ? formatMs(v) : "") },
      { label: "Mem", color: cMem, fmt: (v) => (isFinite(v) ? formatBytesMb(v) : "") },
    ];

    // Defer tooltip DOM updates into rAF to avoid excessive layout while dragging cursor.
    let pendingTooltip = null;
    let rafId = 0;
    function scheduleTooltipUpdate(plot, idx) {
      pendingTooltip = { plot, idx };
      if (rafId) return;
      rafId = requestAnimationFrame(() => {
        rafId = 0;
        const p = pendingTooltip;
        pendingTooltip = null;
        if (!p) return;

        const u = p.plot;
        const i = p.idx;
        if (i == null || i < 0) {
          tooltip.style.display = "none";
          return;
        }

        const x = u.data[0] && u.data[0][i];
        const lines = [];
        const header = isFinite(x) ? "t=" + Number(x).toFixed(0) + "s" : "";
        if (header) lines.push('<div class="wfb-chart-tooltip-title">' + header + "</div>");

        // u.data aligns with opts.series: 0=x, 1=rps, 2=p99, 3=mem
        const vals = [u.data[1]?.[i], u.data[2]?.[i], u.data[3]?.[i]];
        for (let s = 0; s < tooltipSeries.length; s++) {
          const meta = tooltipSeries[s];
          const val = meta.fmt(vals[s]);
          if (!val) continue;
          lines.push(
            '<div class="wfb-chart-tooltip-row">' +
              '<span class="wfb-chart-tooltip-swatch" style="background:' +
              meta.color +
              '"></span>' +
              '<span class="wfb-chart-tooltip-label">' +
              meta.label +
              ":</span>" +
              '<span class="wfb-chart-tooltip-value">' +
              val +
              "</span>" +
              "</div>"
          );
        }

        if (lines.length === 0) {
          tooltip.style.display = "none";
          return;
        }

        tooltip.innerHTML = lines.join("");
        tooltip.style.display = "block";

        // Position the tooltip near the cursor and keep it inside the chart.
        const left = Number.isFinite(u.cursor.left) ? u.cursor.left : 0;
        const top = Number.isFinite(u.cursor.top) ? u.cursor.top : 0;
        const rect = root.getBoundingClientRect();
        const tipRect = tooltip.getBoundingClientRect();

        // Place tooltip relative to the cursor:
        // always below, and either bottom-right or bottom-left depending on available space.
        // Don't clamp to the bottom edge: clamping makes it feel "stuck" to the cursor near the bottom.
        const edgePad = 10;
        const padX = -80;
        const padY = 22;

        let xPx = left + padX;
        if (xPx + tipRect.width > rect.width - edgePad) {
          xPx = left - tipRect.width - padX;
        }

        let yPx = top + padY;

        // Keep within left/top so it doesn't disappear; allow overflow to the right/bottom.
        xPx = Math.max(edgePad, xPx);
        yPx = Math.max(edgePad, yPx);

        tooltip.style.left = xPx + "px";
        tooltip.style.top = yPx + "px";
      });
    }

    const plot = new uPlot(
      {
        ...opts,
        hooks: {
          ...(opts.hooks || {}),
          setCursor: [
            ...(opts.hooks?.setCursor || []),
            (u) => {
              // idx is shared across series; for mode=1 charts it's u.cursor.idx
              scheduleTooltipUpdate(u, u.cursor && Number.isFinite(u.cursor.idx) ? u.cursor.idx : null);
            },
          ],
        },
      },
      udata,
      root
    );

    root.addEventListener("mouseleave", function () {
      tooltip.style.display = "none";
    });

    setLegendItems(container.querySelector("[data-wfb-chart-legend]"), [
      { label: "RPS", color: cRps },
      { label: "P99 latency", color: cLat },
      { label: "Memory", color: cMem },
    ]);

    container.dataset.wfbChartInit = "1";
    container.__wfbPlot = plot;

    if (typeof ResizeObserver !== "undefined") {
      const ro = new ResizeObserver(() => {
        const rect = root.getBoundingClientRect();
        const w = Math.floor(rect.width || 0);
        const h = Math.floor(rect.height || 0);
        if (w < 120 || h < 120) return;
        plot.setSize({ width: w, height: h });
      });
      ro.observe(root);
      container.__wfbChartRO = ro;
    }
  }

  function initAll(scope) {
    const root = scope && scope.querySelectorAll ? scope : document;
    const charts = root.querySelectorAll("[data-wfb-bench-chart]");
    for (const c of charts) initOne(c);
  }

  document.addEventListener("DOMContentLoaded", function () {
    initAll(document);
  });

  document.addEventListener("htmx:afterSwap", function (e) {
    // When charts partial is swapped in.
    initAll(e.target);
  });

  // If htmx loads content outside the swap target (rare), this still catches it.
  document.addEventListener("htmx:afterSettle", function (e) {
    initAll(e.target);
  });
})();
