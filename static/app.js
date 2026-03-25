/**
 * poiesisD — IDE UI
 *
 * Modules:
 *   Theme     – dark/light toggle, persisted in localStorage
 *   Tabs      – multi-tab state in sessionStorage, context menu
 *   Sidebar   – toggle visibility
 *   Search    – filter explorer, pause auto-refresh while active
 *   Panel     – output log panel, resizable via drag
 *   Palette   – command palette overlay
 *   Hotkeys   – global keyboard shortcuts
 *   Logs      – fetch and display executor logs
 */

const IS_MAC = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
const MOD_LABEL = IS_MAC ? '\u2318' : 'Ctrl';

/* ====================================================================
   THEME
   ==================================================================== */
const Theme = (() => {
  function current() {
    return document.documentElement.hasAttribute('data-theme') ? 'light' : 'dark';
  }

  function set(t) {
    if (t === 'light') {
      document.documentElement.setAttribute('data-theme', 'light');
    } else {
      document.documentElement.removeAttribute('data-theme');
    }
    localStorage.setItem('poiesisd-theme', t);
    updateIcon();
  }

  function toggle() {
    set(current() === 'dark' ? 'light' : 'dark');
  }

  function init() {
    set(localStorage.getItem('poiesisd-theme') || 'dark');
  }

  function updateIcon() {
    const btn = document.getElementById('theme-btn');
    if (btn) btn.textContent = current() === 'dark' ? 'D' : 'L';
  }

  return { init, toggle, current };
})();

/* ====================================================================
   TABS — multi-tab with sessionStorage
   ==================================================================== */
const Tabs = (() => {
  const STORAGE_KEY = 'poiesisd-tabs';
  const WELCOME = { path: '/', title: 'Welcome' };

  function load() {
    try {
      return JSON.parse(sessionStorage.getItem(STORAGE_KEY)) || [WELCOME];
    } catch {
      return [WELCOME];
    }
  }

  function save(tabs) {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(tabs));
  }

  function getAll() {
    return load();
  }

  /** Ensure current page is in the tab list */
  function ensureCurrent(path, title) {
    const tabs = load();
    if (!tabs.find((t) => t.path === path)) {
      tabs.push({ path, title });
      save(tabs);
    }
    render(path);
  }

  function close(path) {
    let tabs = load();
    tabs = tabs.filter((t) => t.path !== path);
    if (tabs.length === 0) tabs = [WELCOME];
    save(tabs);
    const cur = window.location.pathname;
    if (cur === path) {
      // navigate to nearest tab
      window.location.href = tabs[tabs.length - 1].path;
    } else {
      render(cur);
    }
  }

  function closeOthers(keepPath) {
    const tabs = load().filter((t) => t.path === keepPath);
    if (tabs.length === 0) tabs.push(WELCOME);
    save(tabs);
    if (window.location.pathname !== keepPath) {
      window.location.href = keepPath;
    } else {
      render(keepPath);
    }
  }

  function closeToRight(path) {
    const tabs = load();
    const idx = tabs.findIndex((t) => t.path === path);
    if (idx === -1) return;
    const kept = tabs.slice(0, idx + 1);
    save(kept);
    const cur = window.location.pathname;
    if (!kept.find((t) => t.path === cur)) {
      window.location.href = kept[kept.length - 1].path;
    } else {
      render(cur);
    }
  }

  function closeAll() {
    save([WELCOME]);
    window.location.href = '/';
  }

  function render(activePath) {
    const bar = document.getElementById('tabbar');
    if (!bar) return;
    const tabs = load();

    bar.innerHTML = tabs
      .map((t) => {
        const active = t.path === activePath ? ' active' : '';
        const esc = t.title.replace(/</g, '&lt;');
        return (
          `<div class="tab${active}" data-path="${t.path}">` +
          `<span class="tab-title" onclick="Tabs.go('${t.path}')">${esc}</span>` +
          (t.path !== '/'
            ? `<span class="tab-close" onclick="event.stopPropagation();Tabs.close('${t.path}')">&times;</span>`
            : '') +
          `</div>`
        );
      })
      .join('');

    // attach right-click context menu
    bar.querySelectorAll('.tab').forEach((el) => {
      el.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        ContextMenu.show(e.clientX, e.clientY, el.dataset.path);
      });
    });
  }

  function go(path) {
    if (window.location.pathname !== path) {
      window.location.href = path;
    }
  }

  /** Navigate to next/prev tab */
  function cycle(dir) {
    const tabs = load();
    const cur = window.location.pathname;
    const idx = tabs.findIndex((t) => t.path === cur);
    let next = idx + dir;
    if (next < 0) next = tabs.length - 1;
    if (next >= tabs.length) next = 0;
    window.location.href = tabs[next].path;
  }

  return { ensureCurrent, close, closeOthers, closeToRight, closeAll, render, go, cycle, getAll };
})();

/* ====================================================================
   CONTEXT MENU — right-click on tabs
   ==================================================================== */
const ContextMenu = (() => {
  let menuEl = null;

  function create() {
    if (menuEl) return;
    menuEl = document.createElement('div');
    menuEl.className = 'ctx-menu';
    menuEl.style.display = 'none';
    document.body.appendChild(menuEl);

    document.addEventListener('click', () => hide());
    document.addEventListener('contextmenu', (e) => {
      if (!e.target.closest('.tab')) hide();
    });
  }

  function show(x, y, path) {
    create();
    const items = [
      { label: 'Close', action: () => Tabs.close(path) },
      { label: 'Close Others', action: () => Tabs.closeOthers(path) },
      { label: 'Close to the Right', action: () => Tabs.closeToRight(path) },
      { label: 'Close All', action: () => Tabs.closeAll() },
    ];

    menuEl.innerHTML = items
      .map((it, i) => `<div class="ctx-item" data-i="${i}">${it.label}</div>`)
      .join('');

    menuEl.querySelectorAll('.ctx-item').forEach((el, i) => {
      el.addEventListener('click', () => {
        items[i].action();
        hide();
      });
    });

    menuEl.style.left = `${x}px`;
    menuEl.style.top = `${y}px`;
    menuEl.style.display = 'block';

    // keep on screen
    requestAnimationFrame(() => {
      const rect = menuEl.getBoundingClientRect();
      if (rect.right > window.innerWidth) menuEl.style.left = `${x - rect.width}px`;
      if (rect.bottom > window.innerHeight) menuEl.style.top = `${y - rect.height}px`;
    });
  }

  function hide() {
    if (menuEl) menuEl.style.display = 'none';
  }

  return { show, hide };
})();

/* ====================================================================
   SIDEBAR
   ==================================================================== */
const Sidebar = (() => {
  function toggle() {
    document.getElementById('shell').classList.toggle('sidebar-hidden');
  }

  return { toggle };
})();

/* ====================================================================
   SEARCH — filter explorer, pause auto-refresh
   ==================================================================== */
const Search = (() => {
  let active = false;
  let selectedIdx = -1;

  function focus() {
    const inp = document.getElementById('search-input');
    if (inp) inp.focus();
  }

  function isActive() {
    return active;
  }

  function onInput(value) {
    active = value.length > 0;
    selectedIdx = -1;
    filter(value);
  }

  function filter(q) {
    const lower = q.toLowerCase();
    const items = document.querySelectorAll('#explorer .task-item');
    items.forEach((el) => {
      const name = el.querySelector('.name')?.textContent || '';
      const id = el.querySelector('.id-short')?.textContent || '';
      el.style.display =
        name.toLowerCase().includes(lower) || id.toLowerCase().includes(lower) ? '' : 'none';
    });
    updateSelection();
  }

  function clear() {
    const inp = document.getElementById('search-input');
    if (inp) {
      inp.value = '';
      onInput('');
      inp.blur();
    }
  }

  function navigate(dir) {
    const visible = [...document.querySelectorAll('#explorer .task-item')].filter(
      (e) => e.style.display !== 'none',
    );
    if (visible.length === 0) return;

    // remove prev highlight
    visible.forEach((e) => {
      e.classList.remove('kb-selected');
    });

    selectedIdx += dir;
    if (selectedIdx < 0) selectedIdx = visible.length - 1;
    if (selectedIdx >= visible.length) selectedIdx = 0;

    visible[selectedIdx].classList.add('kb-selected');
    visible[selectedIdx].scrollIntoView({ block: 'nearest' });
  }

  function confirm() {
    const sel = document.querySelector('#explorer .task-item.kb-selected');
    if (sel) {
      clear();
      window.location.href = sel.getAttribute('href');
    }
  }

  function updateSelection() {
    document.querySelectorAll('#explorer .task-item').forEach((e) => {
      e.classList.remove('kb-selected');
    });
    selectedIdx = -1;
  }

  return { focus, isActive, onInput, clear, navigate, confirm };
})();

/* ====================================================================
   PANEL — output log panel
   ==================================================================== */
const Panel = (() => {
  let open = false;
  let maximized = false;

  function isOpen() {
    return open;
  }

  function toggle() {
    open = !open;
    const p = document.getElementById('panel');
    if (!p) return;
    p.classList.toggle('hidden', !open);
    if (!open) maximized = false;
  }

  function maximize() {
    const p = document.getElementById('panel');
    if (!p) return;
    maximized = !maximized;
    p.style.height = maximized ? '80vh' : '220px';
  }

  function show(html) {
    const p = document.getElementById('panel');
    const body = document.getElementById('panel-body');
    if (!p || !body) return;
    body.innerHTML = html;
    p.classList.remove('hidden');
    open = true;
  }

  function initDrag() {
    const drag = document.getElementById('panel-drag');
    const panel = document.getElementById('panel');
    if (!drag || !panel) return;

    let startY, startH;
    drag.addEventListener('mousedown', (e) => {
      startY = e.clientY;
      startH = panel.offsetHeight;
      const onMove = (e2) => {
        panel.style.height = `${Math.max(80, startH + (startY - e2.clientY))}px`;
      };
      const onUp = () => {
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
      };
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    });
  }

  return { isOpen, toggle, maximize, show, initDrag };
})();

/* ====================================================================
   COMMAND PALETTE
   ==================================================================== */
const Palette = (() => {
  let filtered = [];

  const COMMANDS = [
    {
      name: 'New Task',
      keys: 'Alt+N',
      action: () => {
        window.location.href = '/tasks/new';
      },
    },
    { name: 'Toggle Output Panel', keys: `${MOD_LABEL}+J`, action: Panel.toggle },
    { name: 'Toggle Sidebar', keys: `${MOD_LABEL}+B`, action: Sidebar.toggle },
    { name: 'Toggle Theme', keys: `${MOD_LABEL}+Shift+L`, action: Theme.toggle },
    {
      name: 'Focus Search',
      keys: '/',
      action: () => {
        close();
        Search.focus();
      },
    },
    {
      name: 'Close Current Tab',
      keys: `${MOD_LABEL}+W`,
      action: () => Tabs.close(window.location.pathname),
    },
    {
      name: 'Go to Dashboard',
      keys: '',
      action: () => {
        window.location.href = '/';
      },
    },
  ];

  function open() {
    const o = document.getElementById('cmd-overlay');
    if (!o) return;
    o.classList.add('open');
    const inp = document.getElementById('cmd-input');
    if (inp) {
      inp.value = '';
      inp.focus();
    }
    filtered = COMMANDS;
    render();
  }

  function close() {
    const o = document.getElementById('cmd-overlay');
    if (o) o.classList.remove('open');
  }

  function isOpen() {
    const o = document.getElementById('cmd-overlay');
    return o?.classList.contains('open');
  }

  function filter(q) {
    const l = q.toLowerCase();
    filtered = COMMANDS.filter((c) => c.name.toLowerCase().includes(l));
    render();
  }

  function render() {
    const list = document.getElementById('cmd-list');
    if (!list) return;
    list.innerHTML = filtered
      .map(
        (c, i) =>
          `<div class="cmd-item${i === 0 ? ' sel' : ''}" data-i="${i}">` +
          `<span>${c.name}</span>` +
          `<span class="cmd-keys">${
            c.keys
              ? c.keys
                  .split('+')
                  .map((k) => `<span class=kbd>${k}</span>`)
                  .join('')
              : ''
          }</span>` +
          `</div>`,
      )
      .join('');

    list.querySelectorAll('.cmd-item').forEach((el) => {
      el.addEventListener('mouseenter', () => {
        list.querySelectorAll('.cmd-item').forEach((e) => {
          e.classList.remove('sel');
        });
        el.classList.add('sel');
      });
      el.addEventListener('click', () => {
        close();
        filtered[parseInt(el.dataset.i, 10)].action();
      });
    });
  }

  function navigate(dir) {
    const items = [...document.querySelectorAll('.cmd-item')];
    const cur = items.findIndex((i) => i.classList.contains('sel'));
    items.forEach((i) => {
      i.classList.remove('sel');
    });
    let n = cur + dir;
    if (n < 0) n = items.length - 1;
    if (n >= items.length) n = 0;
    items[n].classList.add('sel');
    items[n].scrollIntoView({ block: 'nearest' });
  }

  function confirm() {
    const s = document.querySelector('.cmd-item.sel');
    if (s) s.click();
  }

  return { open, close, isOpen, filter, navigate, confirm };
})();

/* ====================================================================
   HOTKEYS
   ==================================================================== */
const Hotkeys = (() => {
  function init() {
    document.addEventListener('keydown', handle);
  }

  function handle(e) {
    const mod = IS_MAC ? e.metaKey : e.ctrlKey;

    // ---- Escape ----
    if (e.key === 'Escape') {
      if (Palette.isOpen()) {
        Palette.close();
        e.preventDefault();
        return;
      }
      if (Search.isActive()) {
        Search.clear();
        e.preventDefault();
        return;
      }
      if (Panel.isOpen()) {
        Panel.toggle();
        e.preventDefault();
        return;
      }
      return;
    }

    // ---- Inside command palette ----
    if (Palette.isOpen()) {
      if (e.key === 'Enter') {
        Palette.confirm();
        e.preventDefault();
        return;
      }
      if (e.key === 'ArrowDown') {
        Palette.navigate(1);
        e.preventDefault();
        return;
      }
      if (e.key === 'ArrowUp') {
        Palette.navigate(-1);
        e.preventDefault();
        return;
      }
      return;
    }

    // ---- Inside search ----
    if (document.activeElement && document.activeElement.id === 'search-input') {
      if (e.key === 'ArrowDown') {
        Search.navigate(1);
        e.preventDefault();
        return;
      }
      if (e.key === 'ArrowUp') {
        Search.navigate(-1);
        e.preventDefault();
        return;
      }
      if (e.key === 'Enter') {
        Search.confirm();
        e.preventDefault();
        return;
      }
      return;
    }

    // ---- / → focus search (not in inputs) ----
    if (e.key === '/' && !mod && !e.shiftKey && !isInput(document.activeElement)) {
      Search.focus();
      e.preventDefault();
      return;
    }

    // ---- Alt+N → new task ----
    if (e.altKey && e.key.toLowerCase() === 'n') {
      window.location.href = '/tasks/new';
      e.preventDefault();
      return;
    }

    // ---- Mod shortcuts ----
    if (!mod) return;

    const key = e.key.toLowerCase();

    if (key === 'k' && !e.shiftKey) {
      Palette.open();
      e.preventDefault();
      return;
    }
    if (key === 'j' && !e.shiftKey) {
      Panel.toggle();
      e.preventDefault();
      return;
    }
    if (key === 'b' && !e.shiftKey) {
      Sidebar.toggle();
      e.preventDefault();
      return;
    }
    if (key === 'w' && !e.shiftKey) {
      Tabs.close(window.location.pathname);
      e.preventDefault();
      return;
    }

    // Shift combos
    if (e.shiftKey) {
      if (key === 'l') {
        Theme.toggle();
        e.preventDefault();
        return;
      }
      if (key === 'tab' || key === '[') {
        Tabs.cycle(-1);
        e.preventDefault();
        return;
      }
    }

    // Ctrl+Tab → next tab
    if (key === 'tab' && !e.shiftKey) {
      Tabs.cycle(1);
      e.preventDefault();
      return;
    }
  }

  function isInput(el) {
    if (!el) return false;
    return ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName) || el.isContentEditable;
  }

  return { init };
})();

/* ====================================================================
   LOGS — fetch executor logs into panel
   ==================================================================== */
const Logs = (() => {
  function showExec(taskId, idx) {
    // highlight card
    document.querySelectorAll('.exec-card').forEach((c) => {
      c.classList.remove('active');
    });
    const card = document.querySelector(`.exec-card[data-idx="${idx}"]`);
    if (card) card.classList.add('active');

    fetch(`/ga4gh/tes/v1/tasks/${taskId}?view=FULL`)
      .then((r) => r.json())
      .then((task) => Panel.show(formatLogs(task, idx)))
      .catch(() => Panel.show('<span class="log-info">Failed to fetch logs</span>'));
  }

  function formatLogs(task, idx) {
    if (!task.logs || task.logs.length === 0) {
      return '<span class="log-info">No logs available yet</span>';
    }

    const tl = task.logs[0];
    if (!tl.logs || !tl.logs[idx]) {
      return `<span class="log-info">No logs for executor #${idx} yet</span>`;
    }

    const lg = tl.logs[idx];
    let h = `<span class="log-info">Executor #${idx}`;
    if (lg.exit_code !== undefined && lg.exit_code !== null) {
      h +=
        lg.exit_code === 0
          ? ' <span style="color:var(--green)">exit 0</span>'
          : ` <span style="color:var(--red)">exit ${lg.exit_code}</span>`;
    }
    h += '</span>\n';

    if (lg.start_time) h += `<span class="log-info">started: ${lg.start_time}</span>\n`;
    if (lg.end_time) h += `<span class="log-info">ended:   ${lg.end_time}</span>\n`;
    h += '\n';

    if (lg.stdout?.trim()) {
      h +=
        '<span class="log-label">stdout</span>\n<span class="log-stdout">' +
        esc(lg.stdout) +
        '</span>\n';
    }
    if (lg.stderr?.trim()) {
      h +=
        '<span class="log-label">stderr</span>\n<span class="log-stderr">' +
        esc(lg.stderr) +
        '</span>\n';
    }
    if (!lg.stdout && !lg.stderr) {
      h += '<span class="log-info">(no output)</span>';
    }

    if (tl.system_logs && tl.system_logs.length > 0) {
      h +=
        '\n<span class="log-label">system</span>\n<span class="log-system">' +
        tl.system_logs.map(esc).join('\n') +
        '</span>';
    }

    return h;
  }

  function esc(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }

  return { showExec };
})();

/* ====================================================================
   TASKS — cancel action
   ==================================================================== */
function cancelTask(taskId) {
  if (!window.confirm('Cancel this task?')) return;
  fetch(`/ga4gh/tes/v1/tasks/${taskId}:cancel`, { method: 'POST' })
    .then((r) => {
      if (r.ok) {
        window.location.reload();
      } else {
        r.text().then((t) => {
          window.alert(`Cancel failed: ${t}`);
        });
      }
    })
    .catch(() => {
      window.alert('Cancel request failed');
    });
}

// eslint-disable-next-line no-unused-vars -- used by inline onclick in templates
window.cancelTask = cancelTask;

/* ====================================================================
   INIT
   ==================================================================== */
document.addEventListener('DOMContentLoaded', () => {
  Theme.init();
  Hotkeys.init();
  Panel.initDrag();

  // Register current page as a tab
  const path = window.location.pathname;
  const titleEl = document.querySelector('title');
  const title = titleEl
    ? titleEl.textContent.replace(' — poiesisD', '').replace(' - poiesisD', '')
    : 'Welcome';
  Tabs.ensureCurrent(path, title);

  // Close palette on overlay click
  const overlay = document.getElementById('cmd-overlay');
  if (overlay) {
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) Palette.close();
    });
  }
});

// Make modules available globally for inline onclick handlers
window.Tabs = Tabs;
window.Logs = Logs;
window.Panel = Panel;
window.Sidebar = Sidebar;
window.Search = Search;
window.Palette = Palette;
window.Theme = Theme;
