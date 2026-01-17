(() => {
  const PANEL_ID = "wfb-env-spec";
  const OPEN_ID = "wfb-env-spec-open";
  const DEFAULT_STORAGE_KEY = "wfb.envSpecPanel.closed";

  function getPanel() {
    return document.getElementById(PANEL_ID);
  }

  function getOpenButton() {
    return document.getElementById(OPEN_ID);
  }

  function getStorageKey(panel) {
    if (!panel) return DEFAULT_STORAGE_KEY;
    const key = panel.dataset.wfbEnvSpecStorageKey;
    return key && key.trim().length > 0 ? key : DEFAULT_STORAGE_KEY;
  }

  function isClosed(storageKey) {
    try {
      return localStorage.getItem(storageKey) === "1";
    } catch {
      return false;
    }
  }

  function setClosed(storageKey, closed) {
    try {
      if (closed) {
        localStorage.setItem(storageKey, "1");
      } else {
        localStorage.removeItem(storageKey);
      }
    } catch {
      // Ignore storage errors (private mode, disabled storage, etc.)
    }
  }

  function applyState() {
    const panel = getPanel();
    const openBtn = getOpenButton();
    if (!panel && !openBtn) return;

    const hasSpec = (panel?.dataset.wfbEnvSpecHasSpec ?? openBtn?.dataset.wfbEnvSpecHasSpec) === "1";
    if (!hasSpec) {
      if (panel) {
        panel.style.display = "none";
        delete panel.dataset.wfbEnvSpecClosed;
      }
      if (openBtn) {
        openBtn.style.display = "none";
      }
      return;
    }

    const storageKey = getStorageKey(panel ?? openBtn);
    if (isClosed(storageKey)) {
      if (panel) {
        panel.style.display = "none";
        panel.dataset.wfbEnvSpecClosed = "1";
      }
      if (openBtn) {
        openBtn.style.display = "";
      }
    } else {
      if (panel) {
        panel.style.display = "";
        delete panel.dataset.wfbEnvSpecClosed;
      }
      if (openBtn) {
        openBtn.style.display = "none";
      }
    }
  }

  function closePanel() {
    const panel = getPanel();
    const openBtn = getOpenButton();
    if (!panel) return;

    const storageKey = getStorageKey(panel);
    panel.style.display = "none";
    panel.dataset.wfbEnvSpecClosed = "1";
    setClosed(storageKey, true);

    if (openBtn) {
      openBtn.style.display = "";
    }
  }

  function openPanel() {
    const panel = getPanel();
    const openBtn = getOpenButton();
    if (!panel) return;

    const storageKey = getStorageKey(panel ?? openBtn);
    setClosed(storageKey, false);

    panel.style.display = "";
    delete panel.dataset.wfbEnvSpecClosed;
    if (openBtn) {
      openBtn.style.display = "none";
    }
  }

  // Event delegation so it keeps working after HTMX swaps.
  document.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;

    const closeBtn = target.closest("[data-wfb-env-spec-close]");
    if (!closeBtn) return;

    event.preventDefault();
    closePanel();
  });

  document.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;

    const openBtn = target.closest("[data-wfb-env-spec-open]");
    if (!openBtn) return;

    event.preventDefault();
    openPanel();
  });

  document.addEventListener("DOMContentLoaded", applyState);

  // Re-apply after HTMX navigation and OOB swaps.
  for (const evt of ["htmx:afterSwap", "htmx:oobAfterSwap", "htmx:afterSettle"]) {
    document.addEventListener(evt, applyState);
  }
})();
