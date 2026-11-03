// The Rust side of one panel, played by the film that holds it in an iframe
// (film.js): it answers the panel's commands with made-up data, sends it
// snapshots and events, and gives it the film's clock instead of the real one,
// so that every frame can be drawn again exactly.

const film = window.parent.__film
const me = film.panel(window.frameElement)

// the clock: the film's time; the panel's one-second ticker runs on each frame
Date.now = () => film.now()
const ticks = new Map()
const realSet = window.setInterval
const realClear = window.clearInterval
let nextTick = -1
window.setInterval = (fn, ms, ...args) => {
  if (ms !== 1000) return realSet(fn, ms, ...args)
  const id = nextTick--
  ticks.set(id, fn)
  return id
}
window.clearInterval = id => (ticks.has(id) ? ticks.delete(id) : realClear(id))

// each panel its own storage: the tab, small or big, folded cards
const store = new Map(Object.entries(me.storage ?? {}))
Object.defineProperty(window, 'localStorage', {
  configurable: true,
  value: {
    getItem: k => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => void store.set(k, String(v)),
    removeItem: k => void store.delete(k),
  },
})

// the IPC of Tauri 2: commands, callbacks and the event plugin
const callbacks = new Map()
const listeners = new Map()
let nextId = 1
const settings = {
  blocked: { atOnce: true, remind: true },
  waiting: { atOnce: true, remind: true },
  review: { atOnce: false, remind: false },
  commands: { atOnce: true, remind: true },
  limits: { forgottenMin: 30, longMin: 120, stuckMin: 20, hotMin: 10 },
  remindHours: [2, 4, 12, 24],
  remindDaily: true,
  summary: true,
  summaryAt: 480,
  parkedWeekly: true,
  appearance: 'system',
}
const commands = {
  snapshot: () => me.snapshot(),
  history: () => me.history(),
  get_settings: () => ({ settings }),
  set_settings: a => a.settings,
  is_placed: () => me.placed ?? false,
  set_pinned: a => a.pinned,
  panel_size: a => me.size(a.height, a.width),
  'plugin:event|listen': a => {
    const id = nextId++
    if (!listeners.has(a.event)) listeners.set(a.event, new Map())
    listeners.get(a.event).set(id, a.handler)
    return id
  },
}
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
  transformCallback(callback) {
    const id = nextId++
    callbacks.set(id, callback)
    return id
  },
  unregisterCallback: id => void callbacks.delete(id),
  convertFileSrc: path => path,
  invoke: async (cmd, args = {}) => (commands[cmd] ? commands[cmd](args) : null),
}
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener: (event, id) => void listeners.get(event)?.delete(id),
}

// what the film does to the panel on each frame
const quiet = document.createElement('style')
quiet.textContent = '*, *::before, *::after { transition: none !important; caret-color: transparent; }'
document.head.append(quiet)

window.__mock = {
  emit(event, payload) {
    for (const [id, handler] of listeners.get(event) ?? []) callbacks.get(handler)?.({ event, id, payload })
  },
  // the second ticker, and every CSS animation at the film's time
  tick(ms) {
    for (const fn of ticks.values()) fn()
    for (const a of document.getAnimations()) {
      a.pause()
      a.currentTime = ms
    }
  },
  // light or dark whatever the page around prefers: the dark rules on or off
  theme(dark) {
    for (const sheet of document.styleSheets) {
      let rules
      try {
        rules = sheet.cssRules
      } catch {
        continue
      }
      for (const rule of rules) {
        if (!(rule instanceof CSSMediaRule)) continue
        rule.__scheme ??= rule.media.mediaText.includes('prefers-color-scheme: dark') ? 'dark' : rule.media.mediaText.includes('prefers-color-scheme: light') ? 'light' : 'other'
        if (rule.__scheme === 'dark') rule.media.mediaText = dark ? 'all' : 'not all'
        if (rule.__scheme === 'light') rule.media.mediaText = dark ? 'not all' : 'all'
      }
    }
  },
}
