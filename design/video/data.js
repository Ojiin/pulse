// made-up content for the film: three shops and a blog, none of it real.
// Times are relative to BASE, the moment the film starts on the panel's clock.

export const BASE = new Date(2026, 10, 3, 14, 32, 0).getTime()
const S = 1000
const M = 60 * S
const H = 60 * M
const D = 24 * H
const at = (h, m) => new Date(2026, 10, 3, h, m, 0).getTime()

const subagent = (id, type, description, startedAt) => ({ id, type, description, startedAt, lastAt: BASE, isRunning: true, isBackground: false, depth: 1 })

const running = [
  {
    id: 'r1',
    title: 'Fix rounding in checkout totals',
    project: 'shop-api',
    kind: 'chat',
    since: BASE - 14 * M,
    startedAt: BASE - 14 * M,
    reason: '',
    subagents: [subagent('a1', 'Explore', 'find every price formatter', BASE - 100 * S), subagent('a2', 'general-purpose', 'regression tests for totals', BASE - 52 * S)],
  },
  { id: 'r2', title: 'Hourly comment review', project: 'shop-web', kind: 'recurring', since: BASE - 2 * M, startedAt: BASE - 2 * M, reason: '', subagents: [] },
  { id: 'r3', title: 'Tests for coupon codes', project: 'shop-api', kind: 'chat', parentId: 'r1', since: BASE - 6 * M, startedAt: BASE - 6 * M, reason: '', subagents: [] },
]
// what each running item does now, changing every few seconds
const TOOLS = {
  r1: ['Bash · npm test', 'Read · src/totals.ts', 'Edit · src/totals.ts', 'Bash · npm test'],
  r2: ['WebFetch · comments feed', 'thinking', 'Write · summary.md'],
  r3: ['Edit · coupons.test.ts', 'Bash · npx vitest coupons', 'thinking'],
}
const TURN = { r1: BASE - 192 * S, r2: BASE - 118 * S, r3: BASE - 47 * S }

const open = [
  { id: 'b1', title: 'Daily backup check', project: 'shop-web', kind: 'recurring', state: 'blocked', since: BASE - 2 * D - 3 * H, reason: 'Restore test failed again (day 2). The fix needs your approval.' },
  { id: 'w1', title: 'Feed import: 24-hour check', project: 'shop-api', kind: 'oneoff', state: 'waiting', since: BASE - 5 * H - 4 * M, reason: 'The plan for the broken supplier feeds waits for your go-ahead.' },
  { id: 'w2', title: 'Newsletter digest', project: 'travel-blog', kind: 'recurring', state: 'waiting', since: BASE - 38 * M, reason: 'Two reply drafts are ready for you to check.' },
  { id: 'v1', title: 'Morning link check', project: 'shop-web', kind: 'recurring', state: 'review', since: BASE - 6 * H, reason: '24 links checked, 2 broken, ping sent.' },
  { id: 'v2', title: 'Uptime report', project: 'shop-web', kind: 'recurring', state: 'review', since: BASE - 4 * H - 12 * M, reason: 'All 12 endpoints up, p95 latency 210 ms.' },
  { id: 'v3', title: 'Split the cart store', project: 'shop-web', kind: 'chat', state: 'review', since: BASE - 18 * M, reason: 'Cart store split into three modules, tests green.' },
]
// the routine that asks during the film
export const PRICE_WATCH = { id: 'w3', title: 'Price watch', project: 'shop-api', kind: 'recurring', state: 'waiting', reason: 'A new supplier is waiting for your decision.' }

const later = { id: 'f1', title: 'Plan the spring sale page', project: 'shop-web', kind: 'chat', state: 'review', stage: 'follow', since: BASE - 26 * H, reason: 'Three layouts drafted, one with a countdown.' }
const parked = { id: 'p1', title: 'Move to the new payments API', project: 'shop-api', said: 'Waiting for sandbox keys from the provider.', at: BASE - 3 * D }

// today's quiet runs, for the timeline
const quietRuns = [9, 10, 11, 12, 13].map(h => ({ id: `q${h}`, title: 'Hourly comment review', project: 'shop-web', kind: 'recurring', state: 'quiet', stage: 'done', since: at(h, 6), startedAt: at(h, 5), reason: 'No new comments.' }))
const quietPrice = { id: 'qp', title: 'Price watch', project: 'shop-api', kind: 'recurring', state: 'quiet', stage: 'done', since: at(11, 31), startedAt: at(11, 30), reason: 'No price changes.' }

const startOf = i => ({ b1: BASE - 2 * D - 3 * H - 2 * M, w1: at(9, 26), w2: at(13, 52), v1: at(8, 30), v2: at(10, 15), v3: at(14, 2), w3: at(14, 30) })[i.id] ?? i.since

export const CHECK = {
  key: 'c1',
  sessionId: 'v3',
  title: 'Split the cart store',
  project: 'shop-web',
  what: 'npm run dev',
  background: true,
  flag: 'forgotten',
  started: BASE - 3 * H - 10 * M,
  lastOutput: BASE - 2 * H - 55 * M,
  cpu: 0.01,
  memory: 182 * 1024 ** 2,
  ports: [5173],
  known: false,
}

const planned = [
  { title: 'Hourly comment review', project: 'shop-web', kind: 'recurring', at: at(15, 5), count: 8, last: at(22, 5) },
  { title: 'Clean up the database', project: 'shop-api', kind: 'oneoff', at: at(20, 45), count: 1, last: at(20, 45) },
  { title: 'Morning link check', project: 'shop-web', kind: 'recurring', at: at(8, 30) + D, count: 1, last: at(8, 30) + D },
  { title: 'Renew the TLS certificate', project: 'shop-web', kind: 'oneoff', at: at(10, 0) + D, count: 1, last: at(10, 0) + D },
  { title: 'Daily backup check', project: 'shop-web', kind: 'recurring', at: at(10, 15) + D, count: 1, last: at(10, 15) + D },
]
const missed = [{ title: 'Weekly image run', project: 'shop-web', at: at(11, 0) - D }]

// the snapshot at film time t: what came by then, the clock at its last refresh
export function snapshot(t, { priceWatchAt, checkAt }) {
  const now = BASE + Math.floor(t / 5) * 5 * S
  const step = Math.floor(t / 3.2)
  const items = [
    ...running.map((r, n) => ({
      ...r,
      state: 'running',
      stage: 'running',
      live: { turnStarted: TURN[r.id], tool: TOOLS[r.id][(step + n) % TOOLS[r.id].length], tools: 20 + step, errors: 0 },
    })),
    ...open.map(i => ({ ...i, stage: 'open', startedAt: startOf(i), subagents: [] })),
    ...(t >= priceWatchAt ? [{ ...PRICE_WATCH, stage: 'open', since: BASE + priceWatchAt * S, startedAt: startOf(PRICE_WATCH), subagents: [] }] : []),
    { ...later, startedAt: later.since - 20 * M, subagents: [] },
    ...quietRuns.map(q => ({ ...q, subagents: [] })),
    { ...quietPrice, subagents: [] },
  ]
  const checks = t >= checkAt ? [CHECK] : []
  const waiting = items.filter(i => i.stage === 'open' && i.state === 'waiting').length
  return {
    now,
    tzOffsetMin: -new Date(BASE).getTimezoneOffset(),
    items,
    planned,
    missed,
    counts: { parked: 1, blocked: 1, waiting, review: 3, running: running.length, follow: 1, commands: checks.length },
    checks,
    parked: [{ ...parked, id: parked.id, gone: false }],
    problems: [],
    key: `${now}:${step}:${t >= priceWatchAt}:${checks.length}`,
  }
}

const entry = (id, title, project, kind, at, state, stage, said, extra = {}) => ({ id, title, project, kind, at, state, stage, said, archived: false, ...extra })

// every session, by its last activity
export const history = () => [
  entry('r1', 'Fix rounding in checkout totals', 'shop-api', 'chat', BASE - 30 * S, 'running', 'running', ''),
  entry('r3', 'Tests for coupon codes', 'shop-api', 'chat', BASE - 40 * S, 'running', 'running', '', { parentId: 'r1' }),
  entry('v3', 'Split the cart store', 'shop-web', 'chat', BASE - 18 * M, 'review', 'open', 'Cart store split into three modules, tests green.'),
  entry('h1', 'Checkout copy for the sale', 'shop-web', 'chat', at(12, 40), 'quiet', 'done', 'New button labels and the delivery note are live.'),
  entry('h2', 'Travel blog: autumn post', 'travel-blog', 'chat', at(11, 12), 'quiet', 'done', 'Post scheduled for Thursday 9:00.'),
  entry('h3', 'Checkout: one-page flow', 'shop-web', 'chat', at(9, 48), 'quiet', 'done', 'Address and payment on one page, tests green.'),
  entry('h4', 'Speed up product images', 'shop-web', 'chat', at(8, 55), 'quiet', 'done', 'Images now WebP, LCP down to 1.6 s.'),
  entry('f1', 'Plan the spring sale page', 'shop-web', 'chat', BASE - 26 * H, 'review', 'follow', 'Three layouts drafted, one with a countdown.'),
  entry('h5', 'Checkout: guest orders', 'shop-api', 'chat', at(16, 20) - D, 'quiet', 'done', 'Guest orders keep their cart for 7 days.', { archived: true }),
  entry('h6', 'Map widget for the blog', 'travel-blog', 'chat', at(14, 5) - D, 'quiet', 'done', 'Map loads only on scroll now.'),
  entry('h7', 'Checkout totals: tax per country', 'shop-api', 'chat', at(10, 30) - D, 'quiet', 'done', 'Tax table for 9 countries, tests green.'),
  entry('p1', 'Move to the new payments API', 'shop-api', 'chat', BASE - 3 * D, 'waiting', 'parked', 'Waiting for sandbox keys from the provider.'),
]
