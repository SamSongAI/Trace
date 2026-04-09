(function () {
  const data = window.TRACE_SITE;
  if (!data) return;

  const byId = (id) => document.getElementById(id);

  const setText = (id, value) => {
    const el = byId(id);
    if (el && value !== undefined && value !== null) {
      el.textContent = String(value);
    }
  };

  const setHTML = (id, html) => {
    const el = byId(id);
    if (el) {
      el.innerHTML = html;
    }
  };

  const current = data.current;
  const mac = current.platforms.macos;
  const win = current.platforms.windows;

  setText("site-version", `Version ${current.version}`);
  setText("site-date", `Released ${current.releasedAt}`);
  setText("release-title", current.releaseTitle);

  const notesEl = byId("release-notes");
  if (notesEl && Array.isArray(current.notes)) {
    notesEl.innerHTML = current.notes.map((note) => `<li>${note}</li>`).join("");
  }

  const setPlatform = (platform, config) => {
    const prefix = `platform-${platform}`;
    setText(`${prefix}-status`, config.label || config.status);
    setText(`${prefix}-arch`, config.architecture || "--");
    setText(`${prefix}-minos`, config.minOS || "--");
    setText(`${prefix}-size`, config.size || "--");
    setText(`${prefix}-sha`, config.sha256 || "pending");

    const pill = byId(`${prefix}-status`);
    if (pill) {
      pill.classList.toggle("available", config.status === "available");
    }

    const button = byId(`${prefix}-btn`);
    if (!button) return;

    if (config.status === "available" && config.url) {
      button.href = config.url;
      button.removeAttribute("aria-disabled");
      button.textContent = platform === "macos" ? "下载 macOS" : "下载 Windows";
      if (platform === "windows") {
        button.setAttribute("download", "");
      }
    } else {
      button.setAttribute("aria-disabled", "true");
      if (platform === "windows" && config.waitlistUrl) {
        button.removeAttribute("aria-disabled");
        button.href = config.waitlistUrl;
        button.removeAttribute("download");
        button.textContent = "加入 Windows 候补";
      } else {
        button.href = "#";
        button.textContent = platform === "windows" ? "Windows 即将开放" : "即将开放";
      }
    }
  };

  setPlatform("macos", mac);
  setPlatform("windows", win);

  const autoRecommendPlatform = () => {
    const ua = (navigator.userAgent || "").toLowerCase();
    const isWindows = ua.includes("windows");
    const isMac = ua.includes("macintosh") || ua.includes("mac os x");

    const recommendMac = byId("recommend-macos");
    const recommendWin = byId("recommend-windows");

    if (!recommendMac || !recommendWin) return;

    if (isWindows) {
      recommendWin.style.display = "inline-block";
      recommendMac.style.display = "none";
      return;
    }

    if (isMac) {
      recommendMac.style.display = "inline-block";
      recommendWin.style.display = "none";
      return;
    }

    recommendMac.style.display = "inline-block";
    recommendWin.style.display = "inline-block";
  };

  autoRecommendPlatform();

  const historyBody = byId("history-body");
  if (historyBody && Array.isArray(data.history)) {
    historyBody.innerHTML = data.history.map((entry) => {
      const highlights = entry.highlights.map((h) => `<div>${h}</div>`).join("");
      return `
        <tr>
          <td>${entry.version}</td>
          <td>${entry.releasedAt}</td>
          <td>${entry.title}</td>
          <td>${highlights}</td>
        </tr>
      `;
    }).join("");
  }

  const roadmapEl = byId("roadmap-list");
  if (roadmapEl && Array.isArray(data.roadmap)) {
    roadmapEl.innerHTML = data.roadmap.map((phase) => {
      const items = phase.items.map((item) => `<li>${item}</li>`).join("");
      return `
        <article class="timeline-item reveal delay-1">
          <h3>${phase.quarter} · ${phase.theme}</h3>
          <ul>${items}</ul>
        </article>
      `;
    }).join("");
  }

  const pricingEl = byId("pricing-grid");
  if (pricingEl && Array.isArray(data.pricing)) {
    pricingEl.innerHTML = data.pricing.map((plan, index) => {
      const features = plan.features.map((f) => `<li>${f}</li>`).join("");
      const ctaUrl = plan.ctaUrl || `mailto:team@flashnote.app?subject=${encodeURIComponent(plan.name + ' Plan')}`;
      return `
        <article class="card reveal delay-${Math.min(index + 1, 3)}">
          <h3>${plan.name}</h3>
          <p><strong style="font-size:26px;font-family:'Space Grotesk',sans-serif;">${plan.price}</strong> ${plan.period}</p>
          <p>${plan.description}</p>
          <ul class="meta-list">${features}</ul>
          <div style="margin-top:12px;"><a class="button soft" href="${ctaUrl}">${plan.cta}</a></div>
        </article>
      `;
    }).join("");
  }

  const faqEl = byId("faq-list");
  if (faqEl && Array.isArray(data.faq)) {
    faqEl.innerHTML = data.faq.map((item, index) => `
      <details class="faq-item reveal delay-${Math.min(index + 1, 3)}" ${index === 0 ? "open" : ""}>
        <summary>${item.question}</summary>
        <p>${item.answer}</p>
      </details>
    `).join("");
  }

  setText("product-tagline", data.product.tagline);
  setText("product-summary", data.product.summary);
})();
