// Service Worker — 水仙10周年 wasm 版客户端缓存 (v3)
//
// 设计目标:
//  1. 全量资源预取(断点续传、跨多次访问累计完成)
//  2. 位图(IndexedDB)记录每个文件的预取状态 —— 状态持久,除非用户主动清理
//  3. 除非用户主动清理历史,缓存永不过期(activate 不清理旧缓存,缓存名固定)
//  4. 运行时请求(cache-first)永远优先于后台预取,保证剧情推进零卡顿
//
// 架构:
//  - assets-manifest.json(构建时生成,随 assets 部署):全量文件清单
//    [{path, size, priority}],path 字典序 —— 位图按此顺序索引
//  - 位图:Uint8Array,bit[i] = manifest.files[i] 是否已缓存;持久化于 IndexedDB
//    (store: bitmap, key: current),附带 fingerprint(路径列表哈希)用于
//    清单变更时自动失效重建
//  - 预取:4 通道并发消费队列,按 priority 降序(小文件/剧情高频优先);
//    每项先查缓存(位图可能因重建丢失,缓存才是权威)再决定下载
//  - 运行时请求:命中缓存秒回;未命中与预取共享 in-flight(不重复下载),
//    现场下载后写缓存并同步位图 —— "WASM 主动 fetch 时退至 SW 后台"
//  - 进度:完成项/字节数累计 + 节流广播 postMessage 给所有页面(浮动窗口消费)
//  - 消息协议:
//      START_PREFETCH -> 开始预取(页面空闲时发出)
//      CLEAR_ALL      -> 清空所有缓存 + 位图(用户主动清理)

const SW_VERSION = 'v3.0.0-20260812';

// 缓存名固定(不带版本号):只有用户主动清理(CLEAR_ALL)才会删除,
// 满足"除非主动清理否则缓存不失效"的持久性要求。
const CORE_CACHE = 'core-v1';
const ASSET_CACHE = 'assets-v1';

// 清单相对部署根目录的 URL(SW scope 内)
const MANIFEST_URL = './assets/assets-manifest.json';

// 预取并发通道数
const CONCURRENCY = 4;

// 启动核心资源(小体积,预缓存,秒开)
const CORE_ASSETS = [
  './',
  './index.html',
  './bevy-vn-example-c2dfd4e4200a6e9b.js',
  './manifest.webmanifest',
  './icon-192.png',
  './icon-512.png',
];

// --------------------------------------------------------------------------
// 网络抖动兜底
// --------------------------------------------------------------------------
function fetchWithRetry(req, attempts = 3) {
  return fetch(req).catch((err) => {
    if (attempts > 1) {
      return new Promise((resolve) => setTimeout(resolve, 500))
        .then(() => fetchWithRetry(req, attempts - 1));
    }
    throw err;
  });
}

// --------------------------------------------------------------------------
// 位图(IndexedDB 持久化)
// --------------------------------------------------------------------------
const IDB_NAME = 'prefetch-state';
const IDB_STORE = 'bitmap';
const IDB_KEY = 'current';

let db = null;

function openDB() {
  if (db) return Promise.resolve(db);
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, 1);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains(IDB_STORE)) {
        req.result.createObjectStore(IDB_STORE);
      }
    };
    req.onsuccess = () => { db = req.result; resolve(db); };
    req.onerror = () => reject(req.error);
  });
}

function idbGet() {
  return openDB().then(
    (d) =>
      new Promise((resolve) => {
        const tx = d.transaction(IDB_STORE, 'readonly');
        const r = tx.objectStore(IDB_STORE).get(IDB_KEY);
        r.onsuccess = () => resolve(r.result || null);
        r.onerror = () => resolve(null);
      })
  );
}

function idbPut(value) {
  return openDB().then(
    (d) =>
      new Promise((resolve) => {
        const tx = d.transaction(IDB_STORE, 'readwrite');
        tx.objectStore(IDB_STORE).put(value, IDB_KEY);
        tx.oncomplete = () => resolve(true);
        tx.onerror = () => resolve(false);
      })
  );
}

function idbDelete() {
  return openDB().then(
    (d) =>
      new Promise((resolve) => {
        const tx = d.transaction(IDB_STORE, 'readwrite');
        tx.objectStore(IDB_STORE).delete(IDB_KEY);
        tx.oncomplete = () => resolve(true);
        tx.onerror = () => resolve(false);
      })
  );
}

// --------------------------------------------------------------------------
// 清单 / 位图 状态
// --------------------------------------------------------------------------
let manifest = null;      // { files:[{path,size,priority}], count, total_bytes }
let bitmap = null;        // Uint8Array,bit[i] = files[i] 已缓存
let doneCount = 0;        // 已缓存文件数(增量维护,持久化)
let bytesDone = 0;        // 已缓存字节数(增量维护,持久化)
let pathToIndex = null;   // Map<绝对URL, index>

// 简单字符串哈希(FNV-1a),用于清单指纹
function fingerprintOf(manifestObj) {
  let h = 0x811c9dc5;
  for (const f of manifestObj.files) {
    h ^= f.path.length; h = Math.imul(h, 0x01000193) >>> 0;
    for (let i = 0; i < f.path.length; i++) {
      h ^= f.path.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0;
    }
  }
  return h.toString(16);
}

function bitGet(i) { return (bitmap[i >> 3] >> (i & 7)) & 1; }
function bitSet(i) {
  if (bitGet(i)) return;
  bitmap[i >> 3] |= 1 << (i & 7);
  doneCount++;
}

function urlForPath(relPath) {
  return new URL('./assets/' + relPath, self.location.href).href;
}

// 从磁盘路径(如 /foo/assets/audio/x.ogg)反查清单索引
function indexForURL(urlStr) {
  if (!pathToIndex) return -1;
  const idx = pathToIndex.get(urlStr);
  return idx === undefined ? -1 : idx;
}

// 持久化位图(去抖,最多每 1s 写一次,退出前强制写)
let persistTimer = null;
let persistPending = false;
function persistBitmap() {
  if (!manifest || !bitmap) return;
  if (persistTimer) return; // 已有调度
  persistTimer = setTimeout(() => {
    persistTimer = null;
    idbPut({
      fingerprint: fingerprintOf(manifest),
      bitmap: bitmap.slice(),
      done: doneCount,
      bytesDone,
    });
  }, 1000);
}
// activate 后立即持久化一次初始状态(清空重测后)
function persistNow() {
  if (!manifest || !bitmap) return;
  return idbPut({
    fingerprint: fingerprintOf(manifest),
    bitmap: bitmap.slice(),
    done: doneCount,
    bytesDone,
  });
}

// --------------------------------------------------------------------------
// 进度广播(500ms 节流)
// --------------------------------------------------------------------------
let broadcastTimer = null;
let lastCurrent = '';
let lastBytes = 0;
let lastTime = 0;
let speed = 0;

function broadcastProgress() {
  if (!manifest) return;
  if (broadcastTimer) return;
  broadcastTimer = setTimeout(() => {
    broadcastTimer = null;
    const now = Date.now();
    if (lastTime > 0) {
      const dt = (now - lastTime) / 1000;
      if (dt > 0) speed = (bytesDone - lastBytes) / dt;
    }
    lastTime = now;
    lastBytes = bytesDone;
    const msg = {
      type: 'CACHE_PROGRESS',
      done: doneCount,
      total: manifest.count,
      bytesDone,
      bytesTotal: manifest.total_bytes,
      current: lastCurrent,
      speed,
    };
    self.clients.matchAll({ includeUncontrolled: true }).then((clients) => {
      for (const c of clients) c.postMessage(msg);
    });
  }, 500);
}

// --------------------------------------------------------------------------
// 预取调度:4 通道并发消费队列
// --------------------------------------------------------------------------
let queue = [];      // 待处理索引(已按 priority/path 排序)
let activeCount = 0; // 在途请求数
let pumping = false; // pump 是否在跑
let prefetchRunning = false;

function buildQueue() {
  // 未缓存的索引,按 (priority desc, path asc) 排序
  const idxs = [];
  for (let i = 0; i < manifest.files.length; i++) {
    if (!bitGet(i)) idxs.push(i);
  }
  idxs.sort((a, b) => {
    const pa = manifest.files[a].priority;
    const pb = manifest.files[b].priority;
    if (pa !== pb) return pb - pa;
    return manifest.files[a].path < manifest.files[b].path ? -1 : 1;
  });
  queue = idxs;
}

async function prefetchOne(idx) {
  const item = manifest.files[idx];
  const url = urlForPath(item.path);
  lastCurrent = item.path;

  // 1. 位图可能因重建丢失,缓存才是权威:先查缓存
  const cached = await caches.match(url, { cacheName: ASSET_CACHE });
  if (cached && cached.ok) {
    bitSet(idx);
    bytesDone += item.size;
    persistBitmap();
    broadcastProgress();
    return;
  }

  // 2. 运行时可能正在下载同一文件:复用 in-flight
  const inFlight = self.__inFlight || (self.__inFlight = new Map());
  if (inFlight.has(url)) {
    await inFlight.get(url).catch(() => {});
    bitSet(idx);
    bytesDone += item.size;
    persistBitmap();
    broadcastProgress();
    return;
  }

  // 3. 下载 -> 写缓存(await 完成才返回,保证后续命中完整数据)
  const req = new Request(url, { method: 'GET' });
  const promise = fetchWithRetry(req).then(async (resp) => {
    if (resp && resp.ok) {
      const cache = await caches.open(ASSET_CACHE);
      await cache.put(req, resp.clone());
      bitSet(idx);
      bytesDone += item.size;
    }
    return resp;
  });
  inFlight.set(url, promise);
  try {
    await promise;
  } finally {
    inFlight.delete(url);
  }
  persistBitmap();
  broadcastProgress();
}

function pump() {
  if (pumping) return;
  pumping = true;
  const step = () => {
    while (activeCount < CONCURRENCY && queue.length > 0) {
      const idx = queue.shift();
      if (bitGet(idx)) continue; // 已被运行时请求顺带完成
      activeCount++;
      prefetchOne(idx)
        .catch(() => {}) // 单项失败不中断队列
        .finally(() => {
          activeCount--;
          step();
        });
    }
    if (activeCount === 0 && queue.length === 0) {
      pumping = false;
      prefetchRunning = false;
      lastCurrent = '完成';
      broadcastProgress();
    }
  };
  step();
}

function startPrefetch() {
  if (!manifest || prefetchRunning) return;
  prefetchRunning = true;
  buildQueue();
  console.log(`[sw] 开始预取: ${queue.length} 个文件待下载`);
  pump();
}

// --------------------------------------------------------------------------
// 生命周期
// --------------------------------------------------------------------------
self.addEventListener('install', (event) => {
  event.waitUntil(
    (async () => {
      // 预缓存核心(逐项容忍失败,缺失由运行时 fetch 兜底)
      const cache = await caches.open(CORE_CACHE);
      await Promise.allSettled(CORE_ASSETS.map((u) => cache.add(u)));
      // 拉取清单(失败不阻塞激活:仅失去预取能力,运行时缓存照常)
      try {
        const resp = await fetchWithRetry(new Request(MANIFEST_URL));
        if (resp && resp.ok) {
          manifest = await resp.json();
        }
      } catch (e) {
        console.warn('[sw] manifest fetch failed:', e);
      }
      await self.skipWaiting();
    })()
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      // 申请持久化存储:降低浏览器自动驱逐缓存/位图的风险
      if (navigator.storage && navigator.storage.persist) {
        navigator.storage.persist().catch(() => {});
      }

      // 若 install 阶段未拿到清单,这里重试一次
      if (!manifest) {
        try {
          const resp = await fetchWithRetry(new Request(MANIFEST_URL));
          if (resp && resp.ok) manifest = await resp.json();
        } catch (e) {
          console.warn('[sw] manifest fetch failed (activate):', e);
        }
      }

      // 位图恢复 / 重建
      if (manifest) {
        const stored = await idbGet();
        const fp = fingerprintOf(manifest);
        if (stored && stored.fingerprint === fp && stored.bitmap &&
            stored.bitmap.length === Math.ceil(manifest.count / 8)) {
          bitmap = new Uint8Array(stored.bitmap);
          doneCount = stored.done || 0;
          bytesDone = stored.bytesDone || 0;
          console.log(`[sw] 位图恢复: ${doneCount}/${manifest.count} 已缓存`);
        } else {
          bitmap = new Uint8Array(Math.ceil(manifest.count / 8));
          doneCount = 0;
          bytesDone = 0;
          console.log(`[sw] 位图重建(fingerprint 变更或首次): ${manifest.count} 文件`);
        }
        // 构建 URL->index 映射
        pathToIndex = new Map();
        manifest.files.forEach((f, i) => {
          pathToIndex.set(urlForPath(f.path), i);
        });
        await persistNow();
      }

      // 注意:不删除任何旧缓存 —— 除非用户主动清理,缓存永久有效
      await self.clients.claim();
    })()
  );
});

// --------------------------------------------------------------------------
// 消息协议
// --------------------------------------------------------------------------
self.addEventListener('message', (event) => {
  const data = event.data;
  if (!data || !data.type) return;
  switch (data.type) {
    case 'START_PREFETCH':
      startPrefetch();
      break;
    case 'QUERY_STATUS':
      // 立即回复当前进度(不走节流):刷新页面时预取可能已完成,
      // panel 需要主动拿到真实状态而非停在初始 0%。
      if (manifest && event.source && event.source.postMessage) {
        event.source.postMessage({
          type: 'CACHE_PROGRESS',
          done: doneCount,
          total: manifest.count,
          bytesDone,
          bytesTotal: manifest.total_bytes,
          current: lastCurrent,
          speed: 0,
        });
      }
      break;
    case 'CLEAR_ALL':
      event.waitUntil(
        (async () => {
          const keys = await caches.keys();
          await Promise.all(keys.map((k) => caches.delete(k)));
          await idbDelete();
          bitmap = null;
          manifest = null;
          doneCount = 0;
          bytesDone = 0;
          console.log('[sw] 已清空全部缓存与预取状态');
        })()
      );
      break;
    default:
      break;
  }
});

// --------------------------------------------------------------------------
// fetch 拦截:cache-first + 与预取共享 in-flight
// --------------------------------------------------------------------------
self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;
  if (req.mode === 'navigate') return; // 页面本身放行(网络)

  const isAsset = url.pathname.includes('/assets/');

  const inFlight = self.__inFlight || (self.__inFlight = new Map());

  // 同一 URL 已有在途请求(预取或运行时):直接复用,避免重复下载
  if (inFlight.has(req.url)) {
    event.respondWith(inFlight.get(req.url));
    return;
  }

  const promise = (async () => {
    const cacheName = isAsset ? ASSET_CACHE : CORE_CACHE;

    // 1. 缓存优先
    const cached = await caches.match(req, { cacheName });
    if (cached && cached.ok) {
      // 同步位图(运行时提前命中后台预取尚未处理的文件)
      const idx = indexForURL(req.url);
      if (idx >= 0 && !bitGet(idx) && manifest) {
        bitSet(idx);
        bytesDone += manifest.files[idx].size;
        persistBitmap();
      }
      return cached;
    }

    // 2. 未命中 -> 网络下载 -> await 写缓存 -> 返回
    const resp = await fetchWithRetry(req);
    if (resp && resp.ok) {
      const cache = await caches.open(cacheName);
      await cache.put(req, resp.clone());
      const idx = indexForURL(req.url);
      if (idx >= 0 && manifest) {
        bitSet(idx);
        bytesDone += manifest.files[idx].size;
        persistBitmap();
        broadcastProgress();
      }
    }
    return resp;
  })();

  inFlight.set(req.url, promise);
  promise.finally(() => {
    if (inFlight.get(req.url) === promise) inFlight.delete(req.url);
  }).catch(() => {});
  event.respondWith(promise);
});
