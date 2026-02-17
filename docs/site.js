(function () {
  "use strict";

  /* ------------------------------------------------
     Fade-in observer
     ------------------------------------------------ */
  var observer = new IntersectionObserver(function (entries) {
    entries.forEach(function (entry) {
      if (entry.isIntersecting) entry.target.classList.add("visible");
    });
  }, { threshold: 0.12, rootMargin: "0px 0px -60px 0px" });

  document.querySelectorAll(".fade-in").forEach(function (el) {
    observer.observe(el);
  });

  /* ------------------------------------------------
     Mobile nav toggle
     ------------------------------------------------ */
  var toggle = document.querySelector(".nav-toggle");
  var links = document.querySelector(".site-nav-links");

  if (toggle && links) {
    toggle.addEventListener("click", function () {
      var open = links.classList.toggle("open");
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    });

    links.querySelectorAll("a").forEach(function (link) {
      link.addEventListener("click", function () {
        links.classList.remove("open");
        toggle.setAttribute("aria-expanded", "false");
      });
    });
  }

  /* ------------------------------------------------
     Schedule row expansion (accordion)
     ------------------------------------------------ */
  document.querySelectorAll(".schedule-row").forEach(function (row) {
    row.addEventListener("click", function () {
      var wasExpanded = row.classList.contains("expanded");

      document.querySelectorAll(".schedule-row.expanded").forEach(function (open) {
        open.classList.remove("expanded");
      });

      if (!wasExpanded) {
        row.classList.add("expanded");
      }
    });
  });

  /* ------------------------------------------------
     Action checkbox interaction
     ------------------------------------------------ */
  document.querySelectorAll(".action-check").forEach(function (check) {
    check.addEventListener("click", function (e) {
      e.stopPropagation();
      var row = check.closest(".action-row");
      var checked = check.classList.toggle("checked");

      if (row) {
        row.classList.toggle("checked", checked);
      }
    });
  });
})();
