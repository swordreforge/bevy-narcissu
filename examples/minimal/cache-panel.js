// cache-panel.js — 资源预取进度浮动窗口
//
// 特性:
//  - 默认右上角小胶囊(仅显示百分比),点击展开完整进度条
//  - 可拖动(按住窗口标题区拖动,记忆位置到 localStorage)
//  - 可最小化(收起为小胶囊),可关闭
//  - pointer-events:none 容器 + auto 子元素:不拦截 canvas 游戏输入
//  - 消费 SW 广播的 CACHE_PROGRESS 消息
//
// 注意:index.html 的触摸转换脚本已在捕获阶段排除 #cache-panel 区域,
// 本窗口内的触摸/拖动事件不会被转换成 canvas 鼠标事件。

(function () {
  'use strict';

  // ---- 样式注入 ---------------------------------------------------------
  var CSS = `
#cache-panel {
  position: fixed;
  right: 12px;
  bottom: 12px;
  z-index: 9999;
  pointer-events: none;   /* 容器不挡 canvas 输入 */
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  user-select: none;
  -webkit-user-select: none;
}
#cache-panel * { pointer-events: auto; }  /* 子元素可交互 */
#cache-panel .cp-btn {
  display: flex; align-items: center; gap: 6px;
  background: rgba(20, 20, 26, 0.85);
  border: 1px solid rgba(255,255,255,0.15);
  border-radius: 999px;
  padding: 6px 12px;
  color: #fff;
  font-size: 12px;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(0,0,0,0.4);
}
#cache-panel .cp-btn:hover { background: rgba(40, 40, 50, 0.9); }
#cache-panel .cp-btn.done { background: rgba(20, 60, 30, 0.85); border-color: rgba(80, 200, 120, 0.4); }
#cache-panel .cp-dot {
  width: 10px; height: 10px; border-radius: 50%;
  background: #4da3ff; flex: none;
}
#cache-panel .cp-dot.loading { animation: cp-pulse 1s ease-in-out infinite; }
#cache-panel .cp-dot.done { background: #4ade80; }
#cache-panel .cp-dot.error { background: #f87171; }
@keyframes cp-pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.35; } }

#cache-panel .cp-window {
  display: none;
  width: 260px;
  background: rgba(18, 18, 24, 0.92);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 10px;
  color: #fff;
  font-size: 12px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.5);
  overflow: hidden;
}
#cache-panel.expanded .cp-window { display: block; }
#cache-panel.expanded .cp-btn { display: none; }
#cache-panel .cp-head {
  display: flex; align-items: center; gap: 8px;
  padding: 8px 10px;
  background: rgba(255,255,255,0.05);
  cursor: move;
}
#cache-panel .cp-title { flex: 1; font-weight: 600; font-size: 12px; }
#cache-panel .cp-head button {
  background: none; border: none; color: #aaa;
  font-size: 14px; line-height: 1; cursor: pointer; padding: 2px 4px;
}
#cache-panel .cp-head button:hover { color: #fff; }
#cache-panel .cp-body { padding: 10px 12px; }
#cache-panel .cp-bar {
  height: 6px; border-radius: 3px;
  background: rgba(255,255,255,0.12);
  overflow: hidden; margin: 6px 0 8px;
}
#cache-panel .cp-bar-inner {
  height: 100%; width: 0%;
  background: linear-gradient(90deg, #4da3ff, #7c5cff);
  border-radius: 3px;
  transition: width 0.4s ease;
}
#cache-panel .cp-stats { display: flex; justify-content: space-between; color: #bbb; margin-bottom: 4px; }
#cache-panel .cp-sub { color: #888; font-size: 11px; line-height: 1.5; word-break: break-all; }
`;

  function injectCSS() {
    var style = document.createElement('style');
    style.textContent = CSS;
    document.head.appendChild(style);
  }

  // ---- 工具 -------------------------------------------------------------
  function fmtBytes(n) {
    if (!n) return '0 B';
    var units = ['B', 'KB', 'MB', 'GB'];
    var i = 0;
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
    return (i === 0 ? n : n.toFixed(1)) + ' ' + units[i];
  }

  function fmtSpeed(bps) {
    if (!bps || bps <= 0) return '';
    return fmtBytes(bps) + '/s';
  }

  // ---- DOM 构建 ----------------------------------------------------------
  var state = { done: 0, total: 0, bytesDone: 0, bytesTotal: 0, current: '', speed: 0, started: false, doneFlag: false };

  function build() {
    injectCSS();

    var panel = document.createElement('div');
    panel.id = 'cache-panel';
    panel.innerHTML = `
      <div class="cp-btn" role="button" tabindex="0">
        <span class="cp-dot loading"></span>
        <span class="cp-label">缓存 0%</span>
      </div>
      <div class="cp-window">
        <div class="cp-head">
          <span class="cp-title">资源缓存</span>
          <button class="cp-min" title="最小化">—</button>
          <button class="cp-close" title="关闭">✕</button>
        </div>
        <div class="cp-body">
          <div class="cp-stats">
            <span class="cp-count">0 / 0</span>
            <span class="cp-bytes">0 B / 0 B</span>
          </div>
          <div class="cp-bar"><div class="cp-bar-inner"></div></div>
          <div class="cp-stats">
            <span class="cp-speed"></span>
            <span class="cp-pct">0%</span>
          </div>
          <div class="cp-sub"></div>
        </div>
      </div>
    `;
    document.body.appendChild(panel);
    return {
      panel: panel,
      btn: panel.querySelector('.cp-btn'),
      label: panel.querySelector('.cp-label'),
      dot: panel.querySelector('.cp-dot'),
      win: panel.querySelector('.cp-window'),
      head: panel.querySelector('.cp-head'),
      count: panel.querySelector('.cp-count'),
      bytes: panel.querySelector('.cp-bytes'),
      bar: panel.querySelector('.cp-bar-inner'),
      speed: panel.querySelector('.cp-speed'),
      pct: panel.querySelector('.cp-pct'),
      sub: panel.querySelector('.cp-sub'),
      minBtn: panel.querySelector('.cp-min'),
      closeBtn: panel.querySelector('.cp-close'),
    };
  }

  // ---- 渲染 --------------------------------------------------------------
  function render(ui, s) {
    var pct = s.total > 0 ? Math.floor((s.done / s.total) * 100) : 0;
    var allDone = s.total > 0 && s.done >= s.total;

    ui.label.textContent = allDone ? '缓存完成' : '缓存 ' + pct + '%';
    ui.count.textContent = s.done + ' / ' + s.total;
    ui.bytes.textContent = fmtBytes(s.bytesDone) + ' / ' + fmtBytes(s.bytesTotal);
    ui.bar.style.width = pct + '%';
    ui.pct.textContent = pct + '%';
    ui.speed.textContent = fmtSpeed(s.speed);
    ui.sub.textContent = allDone ? '' : (s.current ? '正在: ' + s.current : '等待空闲…');

    if (allDone && !s.doneFlag) {
      s.doneFlag = true;
      ui.btn.classList.add('done');
      ui.dot.classList.remove('loading');
      ui.dot.classList.add('done');
    }
  }

  // ---- 拖动 --------------------------------------------------------------
  function makeDraggable(panel, handle) {
    var dragging = false, startX = 0, startY = 0, origLeft = 0, origTop = 0;
    handle.addEventListener('pointerdown', function (e) {
      if (e.button !== 0) return;
      // 按钮(最小化/关闭)按下时不启动拖动:setPointerCapture 会把
      // pointerup/click 重定向到捕获元素,导致按钮 click 永远不触发。
      if (e.target && e.target.closest && e.target.closest('button')) return;
      dragging = true;
      startX = e.clientX; startY = e.clientY;
      var rect = panel.getBoundingClientRect();
      origLeft = rect.left; origTop = rect.top;
      panel.style.left = origLeft + 'px';
      panel.style.top = origTop + 'px';
      panel.style.right = 'auto';
      panel.style.bottom = 'auto';
      handle.setPointerCapture(e.pointerId);
      e.preventDefault();
    });
    handle.addEventListener('pointermove', function (e) {
      if (!dragging) return;
      panel.style.left = (origLeft + e.clientX - startX) + 'px';
      panel.style.top = (origTop + e.clientY - startY) + 'px';
    });
    handle.addEventListener('pointerup', function () {
      if (!dragging) return;
      dragging = false;
      try {
        localStorage.setItem('cache-panel-pos', JSON.stringify({
          left: panel.style.left, top: panel.style.top,
        }));
      } catch (e) {}
    });
    // 拖动结束后释放捕获,避免吞掉后续点击
    handle.addEventListener('pointercancel', function () { dragging = false; });
  }

  // ---- 消息消费 ----------------------------------------------------------
  function onMessage(ui, e) {
    var d = e.data;
    if (!d || d.type !== 'CACHE_PROGRESS') return;
    state.done = d.done;
    state.total = d.total;
    state.bytesDone = d.bytesDone;
    state.bytesTotal = d.bytesTotal;
    state.current = d.current || '';
    state.speed = d.speed || 0;
    state.started = true;
    render(ui, state);
  }

  // ---- 启动预取 / 查询状态 --------------------------------------------------
  function requestStart() {
    if (navigator.serviceWorker && navigator.serviceWorker.controller) {
      navigator.serviceWorker.controller.postMessage({ type: 'START_PREFETCH' });
    }
  }
  // 主动查询当前进度:刷新/重开页面时,预取可能早已完成,
  // SW 不会再广播 CACHE_PROGRESS,panel 需要主动要一次状态。
  function queryStatus() {
    if (navigator.serviceWorker && navigator.serviceWorker.controller) {
      navigator.serviceWorker.controller.postMessage({ type: 'QUERY_STATUS' });
    }
  }

  // ---- 初始化 ------------------------------------------------------------
  function init() {
    var ui = build();

    // 最小化/展开切换
    ui.btn.addEventListener('click', function () {
      ui.panel.classList.add('expanded');
    });
    ui.minBtn.addEventListener('click', function () {
      ui.panel.classList.remove('expanded');
    });
    // 关闭 = 隐藏面板(不停止预取)
    ui.closeBtn.addEventListener('click', function () {
      ui.panel.style.display = 'none';
    });

    // 拖动
    makeDraggable(ui.panel, ui.head);

    // 恢复位置
    try {
      var pos = JSON.parse(localStorage.getItem('cache-panel-pos') || 'null');
      if (pos && pos.left !== 'auto') {
        ui.panel.style.left = pos.left;
        ui.panel.style.top = pos.top;
        ui.panel.style.right = 'auto';
        ui.panel.style.bottom = 'auto';
      }
    } catch (e) {}

    // 监听 SW 进度
    if (navigator.serviceWorker) {
      navigator.serviceWorker.addEventListener('message', function (e) {
        onMessage(ui, e);
      });
    }

    // 空闲后启动预取:首次空闲或用户首次交互后
    var started = false;
    function tryStart() {
      if (started) return;
      started = true;
      // 等 SW ready(controller 可能稍后才接管)
      setTimeout(requestStart, 1500);
    }
    if ('requestIdleCallback' in window) {
      window.requestIdleCallback(tryStart, { timeout: 30000 });
    } else {
      window.addEventListener('load', function () { setTimeout(tryStart, 3000); });
    }
    // 用户交互兜底(空闲回调可能因持续渲染永不触发)
    ['pointerdown', 'keydown'].forEach(function (ev) {
      window.addEventListener(ev, tryStart, { once: true });
    });

    // SW ready 后:查询当前进度 + 确保 controller 已接管
    if (navigator.serviceWorker) {
      navigator.serviceWorker.ready.then(function () {
        queryStatus();
        tryStart();
      });
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
