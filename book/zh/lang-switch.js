// Language switcher: jumps to the same page in the other language tree.
// Works both for local `mdbook serve` (port 3000 <-> 3001) and for a
// deployed site where the two books live under /zh/ and /en/.
(function () {
  var THIS_LANG = "zh";
  var OTHER_LANG = "en";
  var LABEL = "English";
  var PORTS = { zh: "8300", en: "8301" };

  function otherUrl() {
    var loc = window.location;
    // Local serve: two ports, same path.
    if (loc.port === PORTS[THIS_LANG]) {
      return loc.protocol + "//" + loc.hostname + ":" + PORTS[OTHER_LANG] + loc.pathname + loc.hash;
    }
    // Deployed: swap the /zh/ <-> /en/ path segment.
    var seg = "/" + THIS_LANG + "/";
    if (loc.pathname.indexOf(seg) !== -1) {
      return loc.pathname.replace(seg, "/" + OTHER_LANG + "/") + loc.hash;
    }
    // Fallback: other language root.
    return "/" + OTHER_LANG + "/";
  }

  function addButton() {
    var bar = document.querySelector(".right-buttons");
    if (!bar) return;
    var a = document.createElement("a");
    a.href = otherUrl();
    a.title = "Switch language / 切换语言";
    a.setAttribute("aria-label", a.title);
    a.style.cssText = "padding:0 10px;font-size:1.4rem;line-height:50px;";
    a.textContent = LABEL;
    bar.insertBefore(a, bar.firstChild);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", addButton);
  } else {
    addButton();
  }
})();
