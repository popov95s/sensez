(function () {
  const STORAGE_KEY = "sensez.findings.language";
  const MARKER_ID = "sensez-findings-tabs";

  function markerPresent() {
    return document.getElementById(MARKER_ID) !== null;
  }

  function radioGroups() {
    return Array.from(document.querySelectorAll(".tabbed-set input[type='radio']"));
  }

  function languageFor(input) {
    const label = input.nextElementSibling;
    return normalizeLanguage(label ? label.textContent.trim() : "");
  }

  function normalizeLanguage(language) {
    const value = language.toLowerCase().replace(/\s+/g, " ").trim();
    if (value === "js / ts" || value === "typescript" || value === "javascript") {
      return "typescript";
    }
    if (value === "python") return "python";
    return value;
  }

  function setLanguage(language) {
    language = normalizeLanguage(language);
    if (!language) return;
    radioGroups().forEach((input) => {
      if (languageFor(input) === language) {
        input.checked = true;
      }
    });
    try {
      localStorage.setItem(STORAGE_KEY, language);
    } catch (_error) {
      // Ignore storage errors in private browsing / locked-down contexts.
    }
  }

  function currentLanguage() {
    const checked = radioGroups().find((input) => input.checked);
    return checked ? languageFor(checked) : "";
  }

  if (!markerPresent()) return;

  document.documentElement.classList.add("sensez-findings-page");

  document.addEventListener("change", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (!target.matches(".tabbed-set input[type='radio']")) return;
    setLanguage(languageFor(target));
  });

  function init() {
    let language = "";
    try {
      language = localStorage.getItem(STORAGE_KEY) || "";
    } catch (_error) {
      language = "";
    }
    setLanguage(language || currentLanguage() || "python");
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init, { once: true });
  } else {
    init();
  }
})();
