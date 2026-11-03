#!/usr/bin/env node
// Renders the film (film.html) frame by frame with the installed Chrome and
// puts the frames together with ffmpeg; nothing to install beyond Node 20+,
// Chrome, ffmpeg and the panel's packages (npm install in desktop/ui).
//
//   node design/video/render.mjs                    the whole film → out/pulse.mp4
//   node design/video/render.mjs --from 13 --to 23  a part of it → out/part.mp4
//   node design/video/render.mjs --still 17.5 21    single frames → out/still-17.5.png …
//   options: --fps 60, --scale 2 (draws at 3840 × 2160 and scales down), --out file

import { spawn } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repo = join(here, '..', '..')
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const PORT = 5288
const DEBUG_PORT = 9339

const args = process.argv.slice(2)
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`)
  return i < 0 ? fallback : args[i + 1]
}
const stills = args.includes('--still') ? args.slice(args.indexOf('--still') + 1).filter(a => !a.startsWith('--')).map(Number) : null
const fps = Number(opt('fps', 60))
const scale = Number(opt('scale', 1))
const from = Number(opt('from', 0))
const outDir = join(here, 'out')
mkdirSync(outDir, { recursive: true })

const sleep = ms => new Promise(r => setTimeout(r, ms))
const children = []
const stop = () => children.forEach(c => c.kill())
process.on('exit', stop)
process.on('SIGINT', () => process.exit(130))

async function until(check, what, ms = 30000) {
  const end = Date.now() + ms
  for (;;) {
    try {
      const v = await check()
      if (v) return v
    } catch {
      // not yet
    }
    if (Date.now() > end) throw new Error(`timed out waiting for ${what}`)
    await sleep(200)
  }
}

// the dev server, with the panel's own vite
const vite = spawn(join(repo, 'desktop/ui/node_modules/.bin/vite'), ['--config', join(here, 'vite.config.js')], { cwd: repo, stdio: ['ignore', 'inherit', 'inherit'] })
children.push(vite)
const filmUrl = `http://localhost:${PORT}/design/video/film.html`
await until(async () => (await fetch(filmUrl)).ok, 'vite')

// Chrome without a window, driven over the DevTools protocol
const profile = mkdtempSync(join(tmpdir(), 'pulse-film-'))
process.on('exit', () => {
  try {
    rmSync(profile, { recursive: true, force: true, maxRetries: 3 })
  } catch {
    // Chrome may still be writing it; the system clears its temp folder
  }
})
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${DEBUG_PORT}`, `--user-data-dir=${profile}`, '--hide-scrollbars', '--force-color-profile=srgb', '--no-first-run', '--mute-audio', 'about:blank'], { stdio: 'ignore' })
children.push(chrome)
const target = await until(async () => (await (await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/list`)).json()).find(t => t.type === 'page'), 'Chrome')

const ws = new WebSocket(target.webSocketDebuggerUrl)
await new Promise((resolve, reject) => {
  ws.onopen = resolve
  ws.onerror = reject
})
let nextId = 1
const pending = new Map()
ws.onmessage = e => {
  const msg = JSON.parse(e.data)
  const p = pending.get(msg.id)
  if (!p) return
  pending.delete(msg.id)
  if (msg.error) p.reject(new Error(msg.error.message))
  else p.resolve(msg.result)
}
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const id = nextId++
    pending.set(id, { resolve, reject })
    ws.send(JSON.stringify({ id, method, params }))
  })
const evaluate = async expression => {
  const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text)
  return r.result.value
}

await send('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: scale, mobile: false })
await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'light' }] })
await send('Page.enable')
// the first load lets vite prepare its packages, which may reload the page once
await send('Page.navigate', { url: filmUrl })
await sleep(3000)
await send('Page.navigate', { url: filmUrl })
await until(() => evaluate('window.__ready'), 'the film', 60000)
const duration = await evaluate('import("./film.js").then(m => m.DURATION)')
const to = Number(opt('to', duration))

const shot = async () => Buffer.from((await send('Page.captureScreenshot', { format: 'png', optimizeForSpeed: true })).data, 'base64')

if (stills) {
  for (const t of stills) {
    // run up to it, so what smooths over frames is where it would be
    for (let s = Math.max(0, t - 1); s < t; s += 1 / fps) await evaluate(`window.__frame(${s})`)
    await evaluate(`window.__frame(${t})`)
    const file = join(outDir, `still-${t}.png`)
    writeFileSync(file, await shot())
    console.log(file)
  }
  process.exit(0)
}

const out = opt('out', join(outDir, from === 0 && to === duration ? 'pulse.mp4' : 'part.mp4'))
const ffmpeg = spawn(
  'ffmpeg',
  ['-y', '-loglevel', 'error', '-f', 'image2pipe', '-framerate', String(fps), '-i', '-', ...(scale !== 1 ? ['-vf', 'scale=1920:1080:flags=lanczos'] : []), '-c:v', 'libx264', '-preset', 'slow', '-crf', '15', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', out],
  { stdio: ['pipe', 'inherit', 'inherit'] },
)
const frames = Math.round((to - from) * fps)
const started = Date.now()
for (let n = 0; n < frames; n++) {
  await evaluate(`window.__frame(${from + n / fps})`)
  const png = await shot()
  if (!ffmpeg.stdin.write(png)) await new Promise(r => ffmpeg.stdin.once('drain', r))
  if (n % fps === 0) process.stdout.write(`\r${(from + n / fps).toFixed(0)} s of ${to} s, ${((Date.now() - started) / 1000).toFixed(0)} s so far`)
}
ffmpeg.stdin.end()
await new Promise(r => ffmpeg.on('close', r))
console.log(`\n${out}`)
process.exit(0)
