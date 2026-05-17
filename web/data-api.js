/**
 * Load dashboard data from web/data/*.json (GitHub Pages) or fall back to local API.
 */
(function (global) {
  function dataUrl(file) {
    return new URL("data/" + file, document.baseURI).href;
  }

  async function fetchJson(url) {
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(res.status + " " + res.statusText);
    }
    return res.json();
  }

  async function loadSummary() {
    try {
      return await fetchJson(dataUrl("summary.json"));
    } catch (_) {
      /* static bundle missing — local orderflow serve */
    }
    const res = await fetch("/api/summary");
    return res.json();
  }

  async function loadIntegrators() {
    try {
      const raw = await fetchJson(dataUrl("integrator_recipients.json"));
      if (raw && typeof raw.ok === "boolean") {
        return raw;
      }
      return { ok: true, data: raw };
    } catch (_) {
      /* static bundle missing */
    }
    const res = await fetch("/api/integrators/recipients");
    return res.json();
  }

  let addressesIndexCache = null;

  async function loadAddressesIndex() {
    if (addressesIndexCache) {
      return addressesIndexCache;
    }
    try {
      const body = await fetchJson(dataUrl("addresses.json"));
      if (body.ok && body.index) {
        addressesIndexCache = body.index;
        return addressesIndexCache;
      }
    } catch (_) {
      /* static bundle missing */
    }
    addressesIndexCache = null;
    return null;
  }

  async function lookupAddresses(userType, frontend) {
    const index = await loadAddressesIndex();
    if (index) {
      const key = userType + "|" + (frontend || "");
      return index[key] || [];
    }
    const params = new URLSearchParams({ user_type: userType });
    if (frontend) {
      params.set("frontend", frontend);
    }
    const res = await fetch("/api/addresses?" + params);
    const body = await res.json();
    return body.ok && body.addresses ? body.addresses : [];
  }

  global.OrderflowData = {
    loadSummary,
    loadIntegrators,
    lookupAddresses,
  };
})(typeof window !== "undefined" ? window : globalThis);
