// The film: every frame is a function of its time t (seconds). render.mjs calls
// window.__frame(t) for each frame and takes a screenshot; opened in a browser,
// film.html?t=12.5 shows one frame and ?play plays it in real time.
//
// The panels are the real panel (desktop/ui) in iframes, fed by mock.js with
// the made-up data of data.js. Scenes:
//   1  0.0– 7.0  the problem: sessions and routines everywhere
//   2  7.0–13.0  the menu bar and its counts
//   3 13.0–23.0  the panel opens: running now, then what needs you, oldest first
//   4 23.0–30.0  a notification, a click, the session
//   5 30.0–37.0  commands to check
//   6 37.0–45.0  timeline and history
//   7 45.0–52.0  moved and made small
//   8 52.0–62.0  light to dark, the close

import { BASE, history, snapshot } from './data.js'

export const DURATION = 62
const TIMES = { priceWatchAt: 23.5, checkAt: 30.9 }
const SCALE = 1.45 // the panel on the stage
const MAX_H = 640 // the panel's height before its list scrolls

// easing
const clamp = (x, a = 0, b = 1) => Math.min(b, Math.max(a, x))
const prog = (t, a, b) => clamp((t - a) / (b - a))
const inOut = x => (x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2)
const out = x => 1 - (1 - x) ** 3
const outQuint = x => 1 - (1 - x) ** 5
const lerp = (a, b, x) => a + (b - a) * x
// up over [a, a+d], down over [b-d, b]
const env = (t, a, b, d = 0.6) => out(prog(t, a, a + d)) * (1 - inOut(prog(t, b - d, b)))

const $ = (tag, cls, parent, html) => {
  const el = document.createElement(tag)
  if (cls) el.className = cls
  if (html !== undefined) el.innerHTML = html
  parent?.append(el)
  return el
}
const show = (el, on) => {
  el.style.display = on ? '' : 'none'
  return on
}
const place = (el, x, y, s = 1, o = 1, extra = '') => {
  el.style.transform = `translate(${x}px, ${y}px) scale(${s})${extra}`
  el.style.opacity = String(o)
}

const stage = document.getElementById('stage')
const light = $('div', 'layer', stage)
$('div', 'bg', light)
const dark = $('div', 'layer dark', stage)
$('div', 'bg', dark)

let T = 0
window.__film = {
  now: () => BASE + Math.round(T * 1000),
  panel: frame => PANELS[frame.dataset.id],
}

// ---------------------------------------------------------------- panels

const PANELS = {}
function makePanel(id, parent, { dark: isDark = false, storage }) {
  const win = $('div', 'win', parent)
  const clip = $('div', 'clip', win)
  const frame = $('iframe', '', clip)
  frame.dataset.id = id
  const p = {
    id,
    win,
    clip,
    frame,
    dark: isDark,
    storage,
    placed: false,
    w: 420,
    h: 200,
    shownH: null,
    key: null,
    snapshot: () => snapshot(T, TIMES),
    history,
    size(h, w) {
      p.h = Math.min(Math.ceil(h), MAX_H)
      p.w = Math.ceil(w ?? 420)
      frame.style.width = `${p.w}px`
      frame.style.height = `${p.h}px`
    },
  }
  frame.style.width = '420px'
  frame.style.height = '200px'
  PANELS[id] = p
  frame.src = './panel.html'
  return p
}
const main = makePanel('main', light, { storage: { tab: 'open', size: 'big', 'history-filter': 'chat' } })
const small = makePanel('small', light, { storage: { size: 'small' } })
const smallDark = makePanel('smallDark', dark, { dark: true, storage: { size: 'small' } })

const doc = p => p.frame.contentDocument
const mock = p => p.frame.contentWindow.__mock
// the rectangle of an element in a panel, on the stage
function rectIn(p, el, x, y, s = SCALE) {
  const r = el.getBoundingClientRect()
  return { x: x + r.left * s, y: y + r.top * s, w: r.width * s, h: r.height * s, cx: x + (r.left + r.width / 2) * s, cy: y + (r.top + r.height / 2) * s }
}

// ---------------------------------------------------------------- the menu bar

const WIFI = `<svg class="sys" viewBox="0 0 26 22"><path d="M13 19.5l3.2-3.6a4.6 4.6 0 00-6.4 0z" fill="currentColor"/><path d="M5.6 11.6a10.6 10.6 0 0114.8 0M2 7.8a15.8 15.8 0 0122 0" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/><path d="M8.6 14.6a6.6 6.6 0 018.8 0" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/></svg>`
const BATTERY = `<svg class="sys" viewBox="0 0 34 22" style="width:36px"><rect x="1" y="4" width="28" height="14" rx="4" fill="none" stroke="currentColor" stroke-opacity=".45" stroke-width="1.6"/><rect x="3.4" y="6.4" width="19" height="9.2" rx="2.2" fill="currentColor"/><path d="M31 9v4" stroke="currentColor" stroke-opacity=".45" stroke-width="2" stroke-linecap="round"/></svg>`
const TRAY_COLOR = { blocked: '#ff4d55', waiting: '#f5a524', review: '#a46bf0', commands: '#1fb8a5' }
const TRAY_ORDER = ['blocked', 'waiting', 'review', 'commands']
// when each count changes, and to what
const TICKS = {
  blocked: [[7.9, 1]],
  waiting: [[8.15, 1], [8.4, 2], [TIMES.priceWatchAt + 0.1, 3]],
  review: [[8.65, 1], [8.9, 2], [9.15, 3]],
  commands: [[TIMES.checkAt + 0.1, 1]],
}
const count = (state, t) => {
  let v = 0
  let prev = 0
  let at = -1
  let first = -1
  for (const [time, n] of TICKS[state]) {
    if (time > t) break
    prev = v
    v = n
    at = time
    if (first < 0) first = time
  }
  return { v, prev, at, first }
}

function makeMenuBar(parent) {
  const bar = $('div', 'mb', parent)
  $('div', 'mb-left', bar, '<b>Studio</b><span>File</span><span>Edit</span><span>View</span><span>Window</span><span>Help</span>')
  const right = $('div', 'mb-right', bar)
  const pulse = $('div', 'pulse', right)
  const tray = $('div', 'tray', pulse)
  const chips = {}
  for (const s of TRAY_ORDER) {
    const chip = $('div', 'tray-in', tray)
    chip.style.color = TRAY_COLOR[s]
    chip.innerHTML = G[s]
    const digit = $('span', 'digit', chip)
    chips[s] = { chip, a: $('span', '', digit), b: $('span', '', digit) }
  }
  $('span', '', right, WIFI)
  $('span', '', right, BATTERY)
  const clock = $('span', '', right)
  return { bar, pulse, tray, chips, clock }
}
const barLight = makeMenuBar(light)
const barDark = makeMenuBar(dark)

function drawMenuBar(m, t) {
  for (const s of TRAY_ORDER) {
    const { v, prev, at, first } = count(s, t)
    const { chip, a, b } = m.chips[s]
    const grow = first < 0 ? 0 : outQuint(prog(t, first, first + 0.45))
    chip.style.maxWidth = `${grow * 70}px`
    chip.style.opacity = String(clamp(grow * 1.4 - 0.2))
    chip.style.paddingRight = `${grow * 16}px`
    const roll = at < 0 ? 1 : outQuint(prog(t, at, at + 0.35))
    a.textContent = String(prev)
    b.textContent = String(v)
    a.style.transform = `translateY(${-roll * 100}%)`
    b.style.transform = `translateY(${(1 - roll) * 100}%)`
    a.style.display = at === first ? 'none' : ''
  }
  const d = new Date(BASE + T * 1000)
  const pad = n => String(n).padStart(2, '0')
  m.clock.textContent = `Tue 3 Nov  ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

// the pivot of the zoom: the right edge of the Pulse item, in the bar
const pivot = m => ({ x: m.pulse.offsetLeft + m.pulse.offsetWidth, y: 22 })
function barPlace(m, t) {
  const p = pivot(m)
  // big in the middle while its counts come, then up to its place
  const zoom = inOut(prog(t, 11.7, 13.0))
  const s = lerp(lerp(2.6, 2.4, out(prog(t, 6.6, 7.8))), 1, zoom)
  const tx = lerp(1140, p.x, zoom)
  const ty = lerp(430, p.y, zoom)
  m.bar.style.transform = `translate(${tx - s * p.x}px, ${ty - s * p.y}px) scale(${s})`
  m.bar.style.opacity = String(out(prog(t, 6.6, 7.4)))
}
// the centre of the Pulse item on the stage, once the bar is in its place
const pulseCenter = m => ({ x: m.pulse.offsetLeft + m.pulse.offsetWidth / 2, y: 22 })

// ---------------------------------------------------------------- claims

function claim(parent, lines, { x = 150, y = 540, center = false, size } = {}) {
  const el = $('div', `claim ${center ? 'center' : ''}`, parent)
  if (size) el.style.fontSize = `${size}px`
  const spans = lines.map(([text, cls]) => $('span', `line ${cls ?? ''}`, el, text))
  return { el, spans, x: center ? 0 : x, y }
}
function drawClaim(c, t, a, b, lineAt = []) {
  if (!show(c.el, t >= a && t < b)) return
  c.el.style.top = `${c.y - c.el.offsetHeight / 2}px`
  c.spans.forEach((span, n) => {
    const start = lineAt[n] ?? a + n * 0.18
    const p = out(prog(t, start, start + 0.9))
    const gone = inOut(prog(t, b - 0.6, b))
    span.style.opacity = String(p * (1 - gone))
    span.style.transform = `translate(${c.x}px, ${(1 - p) * 30 - gone * 14}px)`
    span.style.filter = p < 0.98 ? `blur(${(1 - p) * 10}px)` : ''
  })
}

const C = {
  many: claim(light, [['So many sessions.'], ['So many routines.', 'soft']], { center: true }),
  which: claim(light, [['Which one needs you?']], { center: true }),
  bar: claim(light, [['What needs you.'], ['Right in the menu bar.', 'soft']], { center: true, y: 690, size: 84 }),
  oldest: claim(light, [['Oldest first.'], ['Nothing slips through.', 'soft']], { y: 540 }),
  click: claim(light, [['One click.'], ['Straight to the session.', 'soft']], { y: 860, size: 76 }),
  forgot: claim(light, [['The dev server'], ['you forgot?'], ['Pulse didn’t.', 'teal']], { y: 560 }),
  day: claim(light, [['Your whole day.'], ['Every session.', 'soft']], { y: 560 }),
  away: claim(light, [['Out of your way.'], ['Whenever you like.', 'soft']], { x: 960, y: 560 }),
}

// ---------------------------------------------------------------- scene 1: everything at once

const CHIPS = [
  ['Daily backup check', 'shop-web', 'blocked'],
  ['Price watch', 'shop-api', 'waiting'],
  ['Morning link check', 'shop-web', 'review'],
  ['Fix rounding in checkout totals', 'shop-api', 'running'],
  ['Newsletter digest', 'travel-blog', 'waiting'],
  ['Hourly comment review', 'shop-web', 'running'],
  ['Uptime report', 'shop-web', 'review'],
  ['Split the cart store', 'shop-web', 'review'],
  ['Tests for coupon codes', 'shop-api', 'running'],
  ['Feed import: 24-hour check', 'shop-api', 'waiting'],
  ['Travel blog: autumn post', 'travel-blog', 'done'],
  ['Renew the TLS certificate', 'shop-web', 'planned'],
  ['Clean up the database', 'shop-api', 'planned'],
  ['Speed up product images', 'shop-web', 'done'],
  ['Weekly image run', 'shop-web', 'missed'],
  ['Map widget for the blog', 'travel-blog', 'done'],
  ['Checkout: one-page flow', 'shop-web', 'done'],
  ['Plan the spring sale page', 'shop-web', 'follow'],
  ['Move to the new payments API', 'shop-api', 'parked'],
  ['Checkout copy for the sale', 'shop-web', 'done'],
  ['Comment spam filter', 'travel-blog', 'running'],
  ['Review the refund flow', 'shop-api', 'waiting'],
  ['Inventory sync', 'shop-api', 'running'],
  ['Dependency updates', 'shop-web', 'review'],
]
const DOT = { blocked: '#e5484d', waiting: '#e2900a', review: '#8e4ec6', running: '#3e7bfa', done: '#30a46c', planned: '#aeaeb2', missed: '#aeaeb2', follow: '#8e8e93', parked: '#6b7a99' }
let seed = 7
const rand = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646
const chipsLayer = $('div', 'layer', light)
const chips = CHIPS.map(([title, project, state], n) => {
  const el = $('div', 'chip', chipsLayer, `<span class="dot" style="background:${DOT[state]}">${(G[state] ?? G.done).replace(/var\(--knock\)/g, DOT[state])}</span><span><b>${title}</b><small>${project}</small></span>`)
  el.querySelector('svg').style.color = '#fff'
  const z = 0.55 + rand() * 0.75
  const col = n % 6
  const row = Math.floor(n / 6)
  return { el, z, x: -120 + col * 345 + rand() * 140, y: 60 + row * 245 + rand() * 110, vx: (rand() < 0.5 ? -1 : 1) * (10 + rand() * 18), phase: rand() * 6.28, delay: rand() * 0.8 }
})
chips.sort((a, b) => a.z - b.z).forEach(c => chipsLayer.append(c.el))
const veil = $('div', 'veil', light)

function sceneChips(t) {
  if (!show(chipsLayer, t < 7.8)) return show(veil, false)
  show(veil, true)
  veil.style.opacity = String(env(t, 0.4, 6.4, 0.8))
  for (const c of chips) {
    const w = c.el.offsetWidth
    const h = c.el.offsetHeight
    const appear = out(prog(t, c.delay * 0.6, c.delay * 0.6 + 1.2))
    let x = c.x + c.vx * t
    let y = c.y + Math.sin(c.phase + t * 0.5) * 10
    let s = c.z * lerp(0.92, 1, appear)
    let o = appear * lerp(0.45, 1, (c.z - 0.55) / 0.75)
    // then they all go to the menu bar item
    const d = Math.hypot(x - 1140, y - 430) / 1600
    const go = inOut(prog(t, 5.9 + d * 0.5, 6.9 + d * 0.5))
    x = lerp(x, 1060 - w / 2, go)
    y = lerp(y, 430 - h / 2, go)
    s = lerp(s, 0.12, go)
    o *= 1 - prog(go, 0.55, 1)
    place(c.el, x, y, s, o)
    const blur = (1.3 - c.z) * 4
    c.el.style.filter = blur > 0.6 ? `blur(${blur.toFixed(1)}px)` : ''
  }
}

// ---------------------------------------------------------------- the notification and the session

const notif = $('div', 'notif', light, `<img src="/desktop/assets/app-icon.svg" alt=""><span class="app">Pulse</span><span class="tm">now</span><span class="ttl">Needs you: Price watch</span><p>shop-api · A new supplier is waiting for your decision.</p>`)
const session = $('div', 'session', light, `
  <div class="tbar"><i></i><i></i><i></i><span>Price watch <em>· shop-api</em></span></div>
  <div class="msgs">
    <div class="meta">Routine · today 14:30</div>
    <div class="you">Check the supplier prices for the cable range and flag anything new.</div>
    <div>Checked 214 prices, 3 changed within the limits. One new supplier, Northwire Supply, offers the HDMI cables 8 % below the current price.</div>
    <div class="ask">Add Northwire Supply to the watch list?</div>
  </div>`)

// ---------------------------------------------------------------- the pointer

const cursor = $('div', 'cursor', light, `<svg viewBox="0 0 34 50" width="34" height="50"><path d="M3 3v38l9.2-8.6 6 14.2 6.4-2.7-6-14h12.6z" fill="#111" stroke="#fff" stroke-width="2.6" stroke-linejoin="round"/></svg>`)

// ---------------------------------------------------------------- rings

const rings = {}
function ring(key, r, t, a, b, color) {
  const el = (rings[key] ??= $('div', 'ring', light))
  if (!show(el, !!r && t >= a && t < b)) return
  el.style.setProperty('--c', color)
  const pad = 9
  const p = out(prog(t, a, a + 0.5))
  const s = lerp(1.25, 1, p)
  el.style.width = `${r.w + pad * 2}px`
  el.style.height = `${r.h + pad * 2}px`
  el.style.transformOrigin = '50% 50%'
  place(el, r.x - pad, r.y - pad, s, env(t, a, b, 0.45))
}

// ---------------------------------------------------------------- the close

const closing = $('div', 'closing', dark, `<img src="/desktop/assets/app-icon.svg" alt=""><div class="name">Pulse</div><div class="for">For Claude Code on macOS</div><div class="url">github.com/naphti88/claude-dashboard</div>`)
const [icon, name, forLine, url] = closing.children
const edge = $('div', 'edge', stage)
const black = $('div', 'black', stage)

// ---------------------------------------------------------------- each frame

// what the panels should show at t: their data, the tab, the search, placed or not
function applyPanels(t) {
  for (const p of Object.values(PANELS)) {
    const m = mock(p)
    if (!m) continue
    const snap = snapshot(t, TIMES)
    if (snap.key !== p.key) {
      p.key = snap.key
      m.emit('snapshot', snap)
    }
    m.theme(p.dark)
    m.tick(t * 1000)
  }
  const d = doc(main)
  const tab = t < 37.6 ? 'Open' : t < 41.05 ? 'Timeline' : 'History'
  const on = d.querySelector('.tabs > button.on')
  if (on && !on.textContent.startsWith(tab)) [...d.querySelectorAll('.tabs > button')].find(b => b.textContent.startsWith(tab))?.click()
  const query = 'checkout'.slice(0, Math.max(0, Math.floor((t - 41.9) / 0.11) + 1))
  const input = d.querySelector('.hist-bar input')
  if (input && input.value !== query) {
    Object.getOwnPropertyDescriptor(main.frame.contentWindow.HTMLInputElement.prototype, 'value').set.call(input, query)
    input.dispatchEvent(new main.frame.contentWindow.Event('input', { bubbles: true }))
  }
  const placed = t >= 47.5
  if (placed !== main.placed) {
    main.placed = placed
    mock(main).emit('placed', placed)
  }
}

// a part of a panel that comes in: it grows to its height and fades in
function grow(el, p) {
  if (!el) return
  if (el.__mb === undefined) el.__mb = parseFloat(getComputedStyle(el).marginBottom) || 0
  if (p >= 1) {
    el.style.height = el.style.marginBottom = el.style.opacity = el.style.overflow = ''
    return
  }
  el.style.overflow = 'hidden'
  el.style.height = ''
  const natural = el.scrollHeight
  const e = inOut(p)
  el.style.height = `${natural * e}px`
  el.style.marginBottom = `${el.__mb * e}px`
  el.style.opacity = String(clamp(p * 1.7 - 0.5))
}

let lastT = null
function smooth(p, jump) {
  p.shownH = p.shownH === null || jump ? p.h : p.shownH + (p.h - p.shownH) * 0.2
  if (Math.abs(p.shownH - p.h) < 0.4) p.shownH = p.h
  return p.shownH
}

function layout(t) {
  const jump = lastT === null || Math.abs(t - lastT) > 0.1
  lastT = t
  sceneChips(t)

  // the menu bar, light; the dark one is the same bar
  show(barLight.bar, t >= 6.6)
  drawMenuBar(barLight, t)
  barPlace(barLight, t)
  drawMenuBar(barDark, t)
  barPlace(barDark, t)
  barLight.pulse.classList.toggle('on', t >= 13.1 && t < 48.6)

  drawClaim(C.many, t, 0.7, 3.7)
  drawClaim(C.which, t, 3.9, 6.5)
  drawClaim(C.bar, t, 8.4, 12.0)
  drawClaim(C.oldest, t, 14.8, 22.7, [14.8, 16.2])
  drawClaim(C.click, t, 26.3, 29.7, [26.3, 26.9])
  drawClaim(C.forgot, t, 31.3, 36.6, [31.3, 31.5, 32.7])
  drawClaim(C.day, t, 38.0, 44.6, [38.0, 41.6])
  drawClaim(C.away, t, 48.9, 51.8, [48.9, 49.6])

  // the main panel, below the item; moved by hand in scene 7
  const d = doc(main)
  const c = pulseCenter(barLight)
  const x0 = Math.min(c.x - (420 * SCALE) / 2, 1920 - 28 - 420 * SCALE)
  const y0 = 56
  const drag = inOut(prog(t, 46.2, 47.5))
  const mx = lerp(x0, 150, drag)
  const my = lerp(y0, 150, drag)
  const mainOn = show(main.win, t >= 13.2 && t < 49.1)
  if (mainOn) {
    const opening = out(prog(t, 13.2, 13.75))
    const h = smooth(main, jump)
    const shrink = inOut(prog(t, 48.45, 49.0))
    const sw = lerp(420, small.w, shrink)
    const sh = lerp(h, small.h, shrink)
    main.clip.style.width = `${sw}px`
    main.clip.style.height = `${sh}px`
    const lift = Math.sin(Math.PI * drag) * 0.012
    const s = SCALE * lerp(0.94, 1, opening) * (1 + lift)
    const ox = mx + (420 * SCALE * (1 - s / SCALE)) / 2
    place(main.win, ox, my - (1 - opening) * 14, s, opening * (1 - prog(t, 48.55, 48.95)))

    // the cards settle one after another as it opens
    const parts = [...(d.querySelector('.body > div')?.children ?? [])]
    parts.forEach((el, n) => {
      if (t < 16.5) {
        const p = out(prog(t, 13.45 + n * 0.09, 14.15 + n * 0.09))
        el.style.transform = `translateY(${(1 - p) * 16}px)`
        el.style.opacity = String(p)
        el.dataset.settled = '1'
      } else if (el.dataset.settled) {
        el.style.transform = el.style.opacity = ''
        delete el.dataset.settled
      }
    })
    // Price watch comes in, then the commands card
    const pw = [...d.querySelectorAll('.m-waiting .row')].find(r => r.querySelector('.title')?.textContent === 'Price watch')
    grow(pw, prog(t, TIMES.priceWatchAt, TIMES.priceWatchAt + 0.6))
    grow(d.querySelector('#open-checks, .card.m-commands:not(.aside)'), prog(t, TIMES.checkAt, TIMES.checkAt + 0.75))
    const pill = d.querySelector('.hdr .cnt.s-commands')
    if (pill) {
      const p = outQuint(prog(t, TIMES.checkAt, TIMES.checkAt + 0.5))
      pill.style.transform = p < 1 ? `scale(${lerp(0.5, 1, p)})` : ''
      pill.style.opacity = p < 1 ? String(p) : ''
    }

    // the list scrolls: down to the results, up for the commands, down the timeline
    const body = d.querySelector('.body')
    if (body) {
      const max = body.scrollHeight - body.clientHeight
      const review = d.querySelector('.card.m-review')
      const down = review ? clamp(review.offsetTop + review.offsetHeight - body.clientHeight + 10, 0, max) : 0
      const nowLine = d.querySelector('.tl .now')
      const tl = nowLine ? clamp(nowLine.offsetTop - body.clientHeight * 0.4, 0, max) : 0
      let top = 0
      if (t < 30.8) top = down * inOut(prog(t, 19.3, 20.9)) * (1 - inOut(prog(t, 30.0, 30.8)))
      else if (t >= 37.6 && t < 41.05) top = tl * inOut(prog(t, 38.6, 40.4))
      body.scrollTop = top
      // the tab changes with a short dip
      const dip = a => (Math.abs(t - a) > 0.5 ? 1 : t < a ? 1 - inOut(prog(t, a - 0.15, a)) : out(prog(t, a, a + 0.3)))
      body.style.opacity = String(Math.min(dip(37.6), dip(41.05)))
    }
  }

  // rings: the red age, the forgotten dev server
  const age = mainOn && d.querySelector('.m-blocked .age.alarm')
  ring('age', age && rectIn(main, age, mx, y0), t, 17.4, 19.1, '#e5484d')
  const flag = mainOn && d.querySelector('.flag.forgotten')
  ring('flag', flag && rectIn(main, flag, mx, y0), t, 33.0, 35.6, '#12a594')

  // the notification and the session it opens
  if (show(notif, t >= TIMES.priceWatchAt && t < 26.1)) {
    const inn = outQuint(prog(t, TIMES.priceWatchAt + 0.05, TIMES.priceWatchAt + 0.7))
    const press = Math.sin(Math.PI * prog(t, 25.35, 25.6))
    const away = inOut(prog(t, 25.6, 26.05))
    place(notif, lerp(1960, 1920 - 28 - 540, inn) + away * 40, 60, 1 - press * 0.025 - away * 0.04, 1 - away)
  }
  if (show(session, t >= 25.75 && t < 30.1)) {
    const p = outQuint(prog(t, 25.75, 26.6))
    const gone = inOut(prog(t, 29.4, 30.05))
    place(session, 150, 170 + (1 - p) * 40 + gone * 20, lerp(0.95, 1, p) - gone * 0.02, p * (1 - gone))
  }

  // the pointer
  const tabAt = n => {
    const b = d.querySelectorAll('.tabs > button')[n]
    return b ? rectIn(main, b, mx, y0) : { cx: 1500, cy: 200 }
  }
  let cur = null
  if (t >= 24.4 && t < 26.4) {
    const p = inOut(prog(t, 24.4, 25.3))
    cur = { x: lerp(1250, 1920 - 28 - 270, p), y: lerp(620, 112, p), o: env(t, 24.4, 26.4, 0.35), press: prog(t, 25.35, 25.6) }
  } else if (t >= 36.9 && t < 41.9) {
    const a = tabAt(1)
    const b = tabAt(2)
    const p1 = inOut(prog(t, 36.9, 37.45))
    const p2 = inOut(prog(t, 40.35, 40.9))
    const p3 = inOut(prog(t, 41.15, 41.85))
    cur = { x: lerp(lerp(lerp(1560, a.cx, p1), b.cx, p2), b.cx + 90, p3), y: lerp(lerp(lerp(760, a.cy, p1), b.cy, p2), b.cy + 260, p3), o: env(t, 36.9, 41.9, 0.35), press: Math.max(prog(t, 37.45, 37.65), prog(t, 40.9, 41.1)) }
  } else if (t >= 45.3 && t < 49.3) {
    const grab = { x: 285 * SCALE, y: 58 * SCALE } // the free space between the tabs and the tools
    const btn = d.querySelector('.tb[title^="Make small"]')
    const bp = btn ? rectIn(main, btn, mx, my) : { cx: 0, cy: 0 }
    const p1 = inOut(prog(t, 45.3, 46.0))
    const p2 = inOut(prog(t, 47.7, 48.3))
    const gx = mx + grab.x
    const gy = my + grab.y
    cur = { x: lerp(lerp(1560, gx, p1), bp.cx, p2), y: lerp(lerp(800, gy, p1), bp.cy, p2), o: env(t, 45.3, 49.3, 0.35), press: Math.max(t >= 46.05 && t < 47.6 ? 1 : 0, prog(t, 48.3, 48.5)) }
  }
  if (show(cursor, !!cur && cur.o > 0.01)) {
    const pr = cur.press > 0 && cur.press < 1 ? Math.sin(Math.PI * cur.press) : cur.press >= 1 ? 1 : 0
    place(cursor, cur.x - 3, cur.y - 3, 1 - 0.12 * pr, cur.o)
  }

  // made small: the bar where the panel was, then it tucks itself away
  const tuck = inOut(prog(t, 49.7, 50.8))
  const sx = lerp(150, 40, tuck)
  const sy = lerp(150, 66, tuck)
  for (const p of [small, smallDark]) {
    p.clip.style.width = `${p.w}px`
    p.clip.style.height = `${p.h}px`
  }
  if (show(small.win, t >= 48.7 && t < 54)) {
    const p = out(prog(t, 48.7, 49.15))
    place(small.win, sx, sy, SCALE * lerp(1.03, 1, p), p)
  }

  // light to dark, then the close
  const wipe = inOut(prog(t, 52.0, 53.5))
  if (show(dark, t >= 52)) {
    dark.style.clipPath = wipe < 1 ? `inset(0 ${(1 - wipe) * 1920}px 0 0)` : ''
    const fade = 1 - inOut(prog(t, 53.9, 54.7))
    barDark.bar.style.opacity = String(fade)
    if (show(smallDark.win, fade > 0)) place(smallDark.win, sx, sy, SCALE * lerp(1, 0.92, 1 - fade), fade)
    const ip = outQuint(prog(t, 54.3, 55.6))
    place(icon, 0, (1 - ip) * 40, lerp(0.86, 1, ip), ip)
    icon.style.transformOrigin = '50% 50%'
    for (const [el, a] of [
      [name, 55.0],
      [forLine, 55.7],
      [url, 56.3],
    ]) {
      const p = out(prog(t, a, a + 0.9))
      place(el, 0, (1 - p) * 24, 1, p)
      el.style.filter = p < 0.98 ? `blur(${(1 - p) * 8}px)` : ''
    }
  }
  if (show(edge, wipe > 0 && wipe < 1)) {
    edge.style.left = `${wipe * 1920 - 1}px`
    edge.style.opacity = String(Math.sin(Math.PI * wipe))
  }
  black.style.opacity = String(inOut(prog(t, 61.0, 62.0)))
}

const nextFrame = () => new Promise(r => requestAnimationFrame(() => r()))
const sleep = ms => new Promise(r => setTimeout(r, ms))

window.__frame = async t => {
  T = t
  applyPanels(t)
  await sleep(0)
  await nextFrame()
  await sleep(0)
  layout(t)
  await nextFrame()
}

// ready when every panel has drawn and the fonts and icons are in
window.__ready = (async () => {
  await document.fonts.ready
  await Promise.all([...document.images].map(i => i.decode().catch(() => undefined)))
  for (;;) {
    const ok = Object.values(PANELS).every(p => doc(p)?.querySelector('.panel') && mock(p))
    if (ok) break
    await sleep(50)
  }
  await window.__frame(0)
  await sleep(200)
  return true
})()

// in a browser: film.html?t=12.5 for one frame, ?play to play it
const params = new URLSearchParams(location.search)
window.__ready.then(async () => {
  if (params.has('t')) await window.__frame(Number(params.get('t')))
  if (params.has('play')) {
    const start = performance.now() - Number(params.get('play') || 0) * 1000
    for (;;) {
      const t = (performance.now() - start) / 1000
      if (t > DURATION) break
      await window.__frame(t)
    }
  }
})
