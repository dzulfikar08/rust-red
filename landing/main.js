/**
 * RustRED Landing Page — Main Script
 */

(function () {
  'use strict';

  // --- Theme Toggle ---
  var themeToggle = document.getElementById('themeToggle');
  var root = document.documentElement;

  function getStoredTheme() {
    try {
      return localStorage.getItem('rustred-theme');
    } catch (e) {
      return null;
    }
  }

  function setTheme(theme) {
    root.setAttribute('data-theme', theme);
    try {
      localStorage.setItem('rustred-theme', theme);
    } catch (e) {
      // ignore
    }
  }

  var stored = getStoredTheme();
  if (stored) {
    setTheme(stored);
  }

  themeToggle.addEventListener('click', function () {
    var current = root.getAttribute('data-theme');
    setTheme(current === 'dark' ? 'light' : 'dark');
  });

  // --- Mobile Navigation ---
  var hamburger = document.getElementById('navHamburger');
  var navLinks = document.getElementById('navLinks');

  hamburger.addEventListener('click', function () {
    var isActive = navLinks.classList.toggle('active');
    hamburger.classList.toggle('active');
    hamburger.setAttribute('aria-expanded', isActive);
  });

  navLinks.addEventListener('click', function (e) {
    if (e.target.tagName === 'A') {
      navLinks.classList.remove('active');
      hamburger.classList.remove('active');
      hamburger.setAttribute('aria-expanded', 'false');
    }
  });

  // --- Smooth Scroll ---
  document.addEventListener('click', function (e) {
    var anchor = e.target.closest('a[href^="#"]');
    if (!anchor) return;

    var id = anchor.getAttribute('href').slice(1);
    if (!id) return;

    var target = document.getElementById(id);
    if (!target) return;

    e.preventDefault();
    target.scrollIntoView({ behavior: 'smooth', block: 'start' });

    if (history.pushState) {
      history.pushState(null, null, '#' + id);
    }
  });

  // --- Intersection Observer: Fade-In ---
  var animElements = document.querySelectorAll('.animate-on-scroll');

  if ('IntersectionObserver' in window) {
    var animObserver = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add('is-visible');
            animObserver.unobserve(entry.target);
          }
        });
      },
      {
        threshold: 0.12,
        rootMargin: '0px 0px -30px 0px',
      }
    );

    animElements.forEach(function (el) {
      animObserver.observe(el);
    });
  } else {
    animElements.forEach(function (el) {
      el.classList.add('is-visible');
    });
  }

  // --- Stats Counter Animation ---
  var statValues = document.querySelectorAll('.metric-value');
  var statsAnimated = false;

  function animateCounter(el) {
    var target = parseFloat(el.getAttribute('data-target'));
    var suffix = el.getAttribute('data-suffix') || '';
    var prefix = el.getAttribute('data-prefix') || '';
    var isDecimal = el.getAttribute('data-decimal') === 'true';
    var duration = 1500;
    var startTime = null;

    function easeOutExpo(t) {
      return t === 1 ? 1 : 1 - Math.pow(2, -10 * t);
    }

    function step(timestamp) {
      if (!startTime) startTime = timestamp;
      var progress = Math.min((timestamp - startTime) / duration, 1);
      var easedProgress = easeOutExpo(progress);
      var current = easedProgress * target;

      if (isDecimal) {
        el.textContent = prefix + current.toFixed(1) + suffix;
      } else {
        el.textContent = prefix + Math.round(current) + suffix;
      }

      if (progress < 1) {
        requestAnimationFrame(step);
      }
    }

    requestAnimationFrame(step);
  }

  if ('IntersectionObserver' in window && statValues.length > 0) {
    var metricsSection = document.getElementById('metrics');

    if (metricsSection) {
      var statsObserver = new IntersectionObserver(
        function (entries) {
          entries.forEach(function (entry) {
            if (entry.isIntersecting && !statsAnimated) {
              statsAnimated = true;
              statValues.forEach(function (sv) {
                animateCounter(sv);
              });
              statsObserver.unobserve(entry.target);
            }
          });
        },
        { threshold: 0.4 }
      );

      statsObserver.observe(metricsSection);
    }
  }

  // --- Nav Background on Scroll ---
  var nav = document.querySelector('.nav');

  function handleNavScroll() {
    if (window.scrollY > 50) {
      nav.classList.add('nav-scrolled');
    } else {
      nav.classList.remove('nav-scrolled');
    }
  }

  window.addEventListener('scroll', handleNavScroll, { passive: true });
  handleNavScroll();

  // --- Keyboard: Escape closes mobile nav ---
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
      navLinks.classList.remove('active');
      hamburger.classList.remove('active');
      hamburger.setAttribute('aria-expanded', 'false');
    }
  });
})();
