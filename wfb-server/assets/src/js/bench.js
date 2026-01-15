(function () {
    const root = document.getElementById('bench-detail');
    if (!root) return;

    const base = {
        run: root.dataset.run,
        env: root.dataset.env,
        test: root.dataset.test,
        framework: root.dataset.framework,
    };
    const primaryLabel = root.dataset.primaryLabel || 'Primary';

    const dataEl = document.getElementById('bench-data');
    const runsRaw = dataEl ? JSON.parse(dataEl.dataset.runs || '[]') : [];
    const envsRaw = dataEl ? JSON.parse(dataEl.dataset.envs || '[]') : [];
    const testsRaw = dataEl ? JSON.parse(dataEl.dataset.tests || '[]') : [];

    const runsData = runsRaw.map((run) => ({
        value: run.id,
        label: `Run ${run.id}`,
        subLabel: run.created_at_fmt,
        leadingHtml: '<span class="inline-flex h-2 w-2 rounded-full bg-muted-foreground/50"></span>',
    }));

    const envsData = envsRaw.map((env) => ({
        value: env.name,
        label: env.title,
        icon: env.icon,
    }));

    const testsData = testsRaw.map((test) => ({
        value: test.id,
        label: test.name,
        icon: test.icon,
    }));

    const elRun = document.getElementById('compare-run');
    const elEnv = document.getElementById('compare-env');
    const elTest = document.getElementById('compare-test');
    const elFramework = document.getElementById('compare-framework');
    const elBtn = document.getElementById('compare-btn');

    const crosshairPlugin = {
        id: 'crosshairLine',
        afterDraw(chart) {
            const xValue = chart.$crosshairX;
            if (xValue === null || xValue === undefined) return;
            const xScale = chart.scales.x;
            if (!xScale) return;
            if (xValue < xScale.min || xValue > xScale.max) return;
            const xPixel = xScale.getPixelForValue(xValue);
            const area = chart.chartArea;
            if (!area) return;
            if (xPixel < area.left || xPixel > area.right) return;
            const ctx = chart.ctx;
            ctx.save();
            ctx.strokeStyle = 'rgba(148,163,184,0.6)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(xPixel, area.top);
            ctx.lineTo(xPixel, area.bottom);
            ctx.stroke();
            ctx.restore();
        }
    };

    Chart.register(crosshairPlugin);

    let currentPrimaryRaw = [];
    let currentCompareRaw = null;

    function findNearestRow(data, xValue) {
        if (!data || data.length === 0) return null;
        let best = data[0];
        let bestDist = Math.abs(best.elapsedSecs - xValue);
        for (let i = 1; i < data.length; i++) {
            const dist = Math.abs(data[i].elapsedSecs - xValue);
            if (dist < bestDist) {
                bestDist = dist;
                best = data[i];
            }
        }
        return best;
    }

    function formatNumber(value) {
        if (!Number.isFinite(value)) return '0';
        return Math.round(value).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
    }

    function formatMs(micros) {
        const ms = micros / 1000;
        if (!Number.isFinite(ms)) return '0 ms';
        if (ms >= 100) return `${ms.toFixed(0)} ms`;
        if (ms >= 10) return `${ms.toFixed(1)} ms`;
        return `${ms.toFixed(2)} ms`;
    }

    function formatMb(bytes) {
        const mb = bytes / (1024 * 1024);
        if (!Number.isFinite(mb)) return '0 MB';
        if (mb >= 100) return `${mb.toFixed(0)} MB`;
        if (mb >= 10) return `${mb.toFixed(1)} MB`;
        return `${mb.toFixed(2)} MB`;
    }

    function buildTooltipHtml(xValue) {
        const primary = findNearestRow(currentPrimaryRaw, xValue);
        const compare = currentCompareRaw ? findNearestRow(currentCompareRaw, xValue) : null;
        const timeLabel = primary ? primary.elapsedSecs : xValue;
        const timeText = Number.isFinite(timeLabel) ? `${timeLabel.toFixed(1)}s` : '-';
        const compareLabel = elFramework?.dataset?.label || 'Compare';
        const section = (title, color, row) => {
            if (!row) return '';
            return `
                <div class="space-y-1">
                    <div class="flex items-center gap-2 text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                        <span class="inline-flex h-2 w-2 rounded-full" style="background:${color}"></span>
                        ${title}
                    </div>
                    <div class="grid grid-cols-2 gap-x-3 gap-y-1 text-[11px]">
                        <div class="text-muted-foreground">RPS</div>
                        <div class="text-right text-foreground font-semibold">${formatNumber(row.requestsPerSec)}</div>
                        <div class="text-muted-foreground">Latency P99</div>
                        <div class="text-right text-foreground font-semibold">${formatMs(row.latencyP99)}</div>
                        <div class="text-muted-foreground">Memory</div>
                        <div class="text-right text-foreground font-semibold">${formatMb(row.memoryUsageBytes)}</div>
                    </div>
                </div>
            `;
        };

        return `
            <div class="rounded-md border border-border bg-background/95 backdrop-blur px-3 py-2 text-xs shadow-lg space-y-2">
                <div class="text-[11px] font-semibold text-foreground">t = ${timeText}</div>
                ${section(primaryLabel, '#22c55e', primary)}
                ${section(compareLabel, '#3b82f6', compare)}
            </div>
        `;
    }

    function renderExternalTooltip(context) {
        const chart = context.chart;
        const tooltip = context.tooltip;
        let tooltipEl = chart.canvas.parentNode.querySelector('.wfb-tooltip');

        if (!tooltipEl) {
            tooltipEl = document.createElement('div');
            tooltipEl.className = 'wfb-tooltip';
            tooltipEl.style.position = 'absolute';
            tooltipEl.style.pointerEvents = 'none';
            tooltipEl.style.opacity = '0';
            chart.canvas.parentNode.appendChild(tooltipEl);
        }

        if (tooltip.opacity === 0 || chart.$crosshairX === null || chart.$crosshairX === undefined) {
            tooltipEl.style.opacity = '0';
            return;
        }

        tooltipEl.innerHTML = buildTooltipHtml(chart.$crosshairX);
        const area = chart.chartArea;
        const x = tooltip.caretX ?? area.left;
        const y = tooltip.caretY ?? area.top;
        const offset = 32;
        let left = x + offset;
        let top = y - offset;
        tooltipEl.style.left = `${Math.min(Math.max(left, area.left + 8), area.right - 180)}px`;
        tooltipEl.style.top = `${Math.min(Math.max(top, area.top + 8), area.bottom - 120)}px`;
        tooltipEl.style.opacity = '1';
    }

    const chartRps = new Chart(document.getElementById('chart-rps'), {
        type: 'line',
        data: { datasets: [] },
        options: {
            responsive: true,
            parsing: false,
            animation: false,
            normalized: true,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { display: false },
                tooltip: { enabled: false, external: renderExternalTooltip },
                decimation: { enabled: true, algorithm: 'lttb', samples: 200 }
            },
            scales: {
                x: { type: 'linear', title: { display: true, text: 'Time (s)' }, grid: { color: 'rgba(148,163,184,0.2)' }, ticks: { color: 'rgba(148,163,184,0.7)' } },
                y: { position: 'right', title: { display: true, text: 'RPS' }, grid: { color: 'rgba(148,163,184,0.2)' }, ticks: { color: 'rgba(148,163,184,0.7)' } },
            }
        }
    });

    const chartLatency = new Chart(document.getElementById('chart-latency'), {
        type: 'line',
        data: { datasets: [] },
        options: {
            responsive: true,
            parsing: false,
            animation: false,
            normalized: true,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { display: false },
                tooltip: { enabled: false, external: renderExternalTooltip },
                decimation: { enabled: true, algorithm: 'lttb', samples: 200 }
            },
            scales: {
                x: { type: 'linear', title: { display: true, text: 'Time (s)' }, grid: { color: 'rgba(148,163,184,0.2)' }, ticks: { color: 'rgba(148,163,184,0.7)' } },
                y: { position: 'right', title: { display: true, text: 'Latency P99 (ms)' }, grid: { color: 'rgba(148,163,184,0.2)' }, ticks: { color: 'rgba(148,163,184,0.7)' } },
            }
        }
    });

    const chartMemory = new Chart(document.getElementById('chart-memory'), {
        type: 'line',
        data: { datasets: [] },
        options: {
            responsive: true,
            parsing: false,
            animation: false,
            normalized: true,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { display: false },
                tooltip: { enabled: false, external: renderExternalTooltip },
                decimation: { enabled: true, algorithm: 'lttb', samples: 200 }
            },
            scales: {
                x: { type: 'linear', title: { display: true, text: 'Time (s)' }, grid: { color: 'rgba(148,163,184,0.2)' }, ticks: { color: 'rgba(148,163,184,0.7)' } },
                y: { position: 'right', title: { display: true, text: 'Memory (MB)' }, grid: { color: 'rgba(148,163,184,0.2)' }, ticks: { color: 'rgba(148,163,184,0.7)' } },
            }
        }
    });

    const charts = [chartRps, chartLatency, chartMemory];
    const setCrosshair = (xValue) => {
        charts.forEach((chart) => {
            chart.$crosshairX = xValue;
            chart.draw();
        });
    };
    const bindCrosshair = (chart) => {
        chart.canvas.addEventListener('mousemove', (event) => {
            const rect = chart.canvas.getBoundingClientRect();
            const x = event.clientX - rect.left;
            const area = chart.chartArea;
            if (!area || x < area.left || x > area.right) {
                setCrosshair(null);
                return;
            }
            const xValue = chart.scales.x.getValueForPixel(x);
            if (Number.isFinite(xValue)) setCrosshair(xValue);
        });
        chart.canvas.addEventListener('mouseleave', () => {
            setCrosshair(null);
        });
    };
    charts.forEach(bindCrosshair);

    function asSeries(data, key) {
        return data.map((row) => ({ x: row.elapsedSecs, y: row[key] }));
    }

    function toMs(valueMicros) {
        return valueMicros / 1000;
    }

    function toMb(bytes) {
        return bytes / (1024 * 1024);
    }

    function renderDropdown(root, items, selectedValue) {
        if (!root) return;
        if (root._handler) {
            root.removeEventListener('click', root._handler);
        }
        if (!items || items.length === 0) {
            root.innerHTML = '<div class="h-9 w-full rounded-md border border-dashed border-border/60 px-3 text-xs flex items-center text-muted-foreground">No options</div>';
            root.dataset.value = '';
            return;
        }

        const selected = items.find((item) => item.value === selectedValue) || items[0];
        root.dataset.value = selected.value;

        const iconHtml = (icon) => {
            if (!icon) return '';
            if (typeof getIcon === 'function') return getIcon(icon, "h-4 w-4");
            return `<i data-lucide="${icon}" class="h-4 w-4 text-muted-foreground" aria-hidden="true"></i>`;
        };
        const leadingHtml = (item) => item.leadingHtml || iconHtml(item.icon);
        const summaryIcon = leadingHtml(selected);
        root.dataset.label = selected.label || '';
        const summaryLabel = selected.subLabel
            ? `<span class="truncate">${selected.label} • ${selected.subLabel}</span>`
            : `<span class="truncate">${selected.label}</span>`;

        const optionsHtml = items.map((item) => `
            <button type="button" data-value="${item.value}"
                class="flex items-center gap-2 w-full rounded-md px-3 py-2 text-xs font-medium transition hover:bg-muted/70 text-muted-foreground hover:text-foreground">
                ${leadingHtml(item)}
                <span class="flex flex-col items-start truncate">
                    <span class="truncate">${item.label}</span>
                    ${item.subLabel ? `<span class="text-[10px] text-muted-foreground">${item.subLabel}</span>` : ''}
                </span>
            </button>
        `).join('');

        root.innerHTML = `
            <details class="relative w-full">
                <summary class="list-none h-9 w-full rounded-md border border-input bg-background px-3 text-xs font-semibold text-foreground shadow-sm flex items-center justify-between cursor-pointer">
                    <span class="inline-flex items-center gap-2">
                        ${summaryIcon}
                        ${summaryLabel}
                    </span>
                    ${typeof getIcon === 'function' ? getIcon("chevron-down", "h-4 w-4 text-muted-foreground") : `<i data-lucide="chevron-down" class="h-4 w-4 text-muted-foreground" aria-hidden="true"></i>`}
                </summary>
                <div class="absolute left-0 right-0 mt-2 rounded-md border border-border bg-popover text-popover-foreground shadow-xl ring-1 ring-black/5 z-20 max-h-64 overflow-y-auto p-1">
                    ${optionsHtml}
                </div>
            </details>
        `;

        root._handler = (event) => {
            const button = event.target.closest('button[data-value]');
            if (!button) return;
            const value = button.dataset.value;
            renderDropdown(root, items, value);
            const details = root.querySelector('details');
            if (details) details.open = false;

             // Dispatch a custom event to notify listeners that the value has changed
             root.dispatchEvent(new Event('change'));
        };
        root.addEventListener('click', root._handler);

        if (window.lucide && typeof window.lucide.createIcons === 'function') {
            window.lucide.createIcons();
        }
    }

    async function fetchRaw(selection) {
        const url = `/api/runs/${selection.run}/environments/${selection.env}/tests/${selection.test}/frameworks/${selection.framework}/raw`;
        const res = await fetch(url);
        if (!res.ok) return [];
        return res.json();
    }

    function renderCharts(primary, compare) {
        const primaryColor = '#22c55e';
        const primaryFill = 'rgba(34,197,94,0.0)';
        const compareColor = '#3b82f6';
        const compareFill = 'rgba(59,130,246,0.0)';

        currentPrimaryRaw = primary || [];
        currentCompareRaw = compare || null;

        chartRps.data.datasets = [
            {
                label: 'Primary',
                data: asSeries(primary, 'requestsPerSec'),
                borderColor: primaryColor,
                backgroundColor: primaryFill,
                tension: 0,
                fill: false,
                borderWidth: 1.5,
                pointRadius: 0,
                pointHitRadius: 8,
            },
        ];

        chartLatency.data.datasets = [
            {
                label: 'Primary',
                data: primary.map((row) => ({ x: row.elapsedSecs, y: toMs(row.latencyP99) })),
                borderColor: primaryColor,
                backgroundColor: primaryFill,
                tension: 0,
                fill: false,
                borderWidth: 1.5,
                pointRadius: 0,
                pointHitRadius: 8,
            },
        ];

        chartMemory.data.datasets = [
            {
                label: 'Primary',
                data: primary.map((row) => ({ x: row.elapsedSecs, y: toMb(row.memoryUsageBytes) })),
                borderColor: primaryColor,
                backgroundColor: primaryFill,
                tension: 0,
                fill: false,
                borderWidth: 1.5,
                pointRadius: 0,
                pointHitRadius: 8,
            },
        ];

        if (compare) {
            chartRps.data.datasets.push({
                label: 'Compare',
                data: asSeries(compare, 'requestsPerSec'),
                borderColor: compareColor,
                backgroundColor: compareFill,
                tension: 0,
                fill: false,
                borderWidth: 1.5,
                pointRadius: 0,
                pointHitRadius: 8,
            });
            chartLatency.data.datasets.push({
                label: 'Compare',
                data: compare.map((row) => ({ x: row.elapsedSecs, y: toMs(row.latencyP99) })),
                borderColor: compareColor,
                backgroundColor: compareFill,
                tension: 0,
                fill: false,
                borderWidth: 1.5,
                pointRadius: 0,
                pointHitRadius: 8,
            });
            chartMemory.data.datasets.push({
                label: 'Compare',
                data: compare.map((row) => ({ x: row.elapsedSecs, y: toMb(row.memoryUsageBytes) })),
                borderColor: compareColor,
                backgroundColor: compareFill,
                tension: 0,
                fill: false,
                borderWidth: 1.5,
                pointRadius: 0,
                pointHitRadius: 8,
            });
        }

        chartRps.update();
        chartLatency.update();
        chartMemory.update();
    }


    let languageMap = new Map();

    async function loadOptions() {
        const langsRes = await fetch('/api/languages');
        const langs = (await langsRes.json()) || [];

        languageMap = new Map(langs.map((l) => [l.name, l]));

        renderDropdown(elRun, runsData, base.run);
        renderDropdown(elEnv, envsData, base.env);
        renderDropdown(elTest, testsData, base.test);

        await loadFrameworks();
    }

    async function loadFrameworks() {
        const url = `/api/runs/${elRun.dataset.value}/environments/${elEnv.dataset.value}/tests/${elTest.dataset.value}`;
        const res = await fetch(url);
        const rows = res.ok ? await res.json() : [];
        const frameworkItems = rows
            .map((r) => {
                const lang = languageMap.get(r.language);
                const color = lang?.color || '#94a3b8';
                return {
                    value: r.framework,
                    label: `${r.language} / ${r.framework}`,
                    leadingHtml: `<span class="inline-flex h-3 w-3 rounded-full" style="background-color: ${color};"></span>`
                };
            })
            .sort((a, b) => a.label.localeCompare(b.label));
        renderDropdown(elFramework, frameworkItems, base.framework);
    }

    elRun.addEventListener('change', loadFrameworks);
    elEnv.addEventListener('change', loadFrameworks);
    elTest.addEventListener('change', loadFrameworks);

    elBtn.addEventListener('click', async () => {
        const compare = {
            run: elRun.dataset.value,
            env: elEnv.dataset.value,
            test: elTest.dataset.value,
            framework: elFramework.dataset.value,
        };
        const [primaryRaw, compareRaw] = await Promise.all([
            fetchRaw(base),
            fetchRaw(compare),
        ]);
        renderCharts(primaryRaw, compareRaw);
    });

    (async () => {
        const [_, primaryRaw] = await Promise.all([
            loadOptions(),
            fetchRaw(base),
        ]);
        renderCharts(primaryRaw, null);
    })();
})();
