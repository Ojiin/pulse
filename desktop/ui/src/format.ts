// Times as the panel writes them, in local time

const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR
const pad = (n: number) => String(n).padStart(2, '0')

export const clock = (t: number) => {
  const d = new Date(t)
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`
}

// the local day of a time, to compare days
export const dayOf = (t: number) => {
  const d = new Date(t)
  return d.getFullYear() * 400 + d.getMonth() * 32 + d.getDate()
}

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
export const dayName = (t: number, now: number) => {
  const d = new Date(t)
  if (dayOf(t) === dayOf(now)) return 'Today'
  if (dayOf(t) === dayOf(now + DAY)) return `Tomorrow, ${WEEKDAYS[d.getDay()]} ${d.getDate()}`
  if (dayOf(t) === dayOf(now - DAY)) return 'Yesterday'
  return `${WEEKDAYS[d.getDay()]}, ${MONTHS[d.getMonth()]} ${d.getDate()}`
}

// "for 5 h", "for 2 days"
export const since = (t: number, now: number) => {
  const age = Math.max(0, now - t)
  if (age < HOUR) return `for ${Math.max(1, Math.floor(age / MINUTE))} min`
  if (age < DAY) return `for ${Math.floor(age / HOUR)} h`
  if (age < 2 * DAY) return 'for 1 day'
  return `for ${Math.floor(age / DAY)} days`
}

// "5 h ago"
export const ago = (t: number, now: number) => `${since(t, now).slice(4)} ago`

// 2 h and more lies a while, 24 h and more too long
export const ageLevel = (t: number, now: number) => (now - t >= DAY ? 'alarm' : now - t >= 2 * HOUR ? 'warn' : '')

// a running time: "0:37" or "1:12:05"
export const elapsed = (from: number, now: number) => {
  const s = Math.max(0, Math.floor((now - from) / 1000))
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  return h > 0 ? `${h}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`
}

// "in 23 min", "in 5 h"
export const until = (t: number, now: number) => {
  const d = t - now
  if (d < HOUR) return `in ${Math.max(1, Math.round(d / MINUTE))} min`
  if (d < DAY) return `in ${Math.round(d / HOUR)} h`
  return ''
}

export { DAY, HOUR, MINUTE }
