import { useEffect, useLayoutEffect, useRef, useState } from 'react'

import { closePanel, openClaude, panelHeight, setPinned, useSnapshot } from './api'
import { clock } from './format'
import { Glyph, StateMark } from './glyphs'
import { RunCard, useNow } from './parts'
import { HistoryTab } from './history'
import { SettingsView } from './settings'
import { OpenTab, TimelineTab } from './tabs'

type Tab = 'open' | 'timeline' | 'history'
const TABS: Tab[] = ['open', 'timeline', 'history']

function useStored<T extends string>(key: string, initial: T): [T, (v: T) => void] {
  const [value, setValue] = useState<T>(() => {
    try {
      return (localStorage.getItem(key) as T | null) ?? initial
    } catch {
      return initial
    }
  })
  return [
    value,
    v => {
      setValue(v)
      try {
        localStorage.setItem(key, v)
      } catch {
        // without storage it is kept for this run only
      }
    },
  ]
}

export function App() {
  const { snapshot: snap, error } = useSnapshot()
  const now = useNow()
  const [stored, setTab] = useStored<Tab>('tab', 'open')
  // a tab kept from before that is gone opens Open
  const tab = TABS.includes(stored) ? stored : 'open'
  const [pinned, setPinnedState] = useState(false)
  const [isSettings, setIsSettings] = useState(false)
  const head = useRef<HTMLDivElement>(null)
  const body = useRef<HTMLDivElement>(null)
  const foot = useRef<HTMLDivElement>(null)

  // the window is as tall as its content, up to a limit; beyond it the list scrolls
  useLayoutEffect(() => {
    const measure = () => {
      const h = (head.current?.offsetHeight ?? 0) + (body.current?.scrollHeight ?? 0) + (foot.current?.offsetHeight ?? 0)
      void panelHeight(h)
    }
    const observer = new ResizeObserver(measure)
    for (const el of [head.current, body.current?.firstElementChild, foot.current]) if (el) observer.observe(el)
    measure()
    return () => observer.disconnect()
  }, [tab, snap, isSettings])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') void closePanel()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const c = snap?.counts
  const open = c ? c.blocked + c.waiting + c.review + c.commands : 0
  const togglePin = () => void setPinned(!pinned).then(setPinnedState)

  return (
    <div className={`panel ${pinned ? 'pinned' : ''}`}>
      <div ref={head} className="hdr">
        <div className={`counts ${c && open === 0 && c.running > 0 ? 'none' : ''}`}>
          {c && open + c.running === 0 && (
            <span className="cnt s-calm">
              <Glyph name="done" size={15} />
              All quiet
            </span>
          )}
          {c && c.blocked > 0 && (
            <span className="cnt s-blocked">
              <StateMark state="blocked" size={15} />
              {c.blocked} blocked
            </span>
          )}
          {c && c.waiting > 0 && (
            <span className="cnt s-waiting">
              <StateMark state="waiting" size={15} />
              {c.waiting} {c.waiting === 1 ? 'question' : 'questions'}
            </span>
          )}
          {c && c.commands > 0 && (
            <span className="cnt s-commands">
              <Glyph name="commands" size={15} />
              {c.commands} {c.commands === 1 ? 'command' : 'commands'}
            </span>
          )}
          {c && c.review > 0 && (
            <span className="cnt s-review">
              <StateMark state="review" size={15} />
              {c.review} to review
            </span>
          )}
        </div>
        <div className="tabs">
          {(
            [
              ['open', 'Open'],
              ['timeline', 'Timeline'],
              ['history', 'History'],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              className={tab === id && !isSettings ? 'on' : ''}
              onClick={() => {
                setTab(id)
                setIsSettings(false)
              }}
            >
              {label}
              {id === 'open' && open > 0 && <small>{open}</small>}
            </button>
          ))}
          <span className="tools">
            <button className={`tb ${isSettings ? 'on' : ''}`} title="Settings" onClick={() => setIsSettings(v => !v)}>
              <Glyph name="gear" size={15} />
            </button>
            <button className={`tb ${pinned ? 'on' : ''}`} title={pinned ? 'Unpin: closes on a click outside' : 'Pin: stays open'} onClick={togglePin}>
              <Glyph name={pinned ? 'pin' : 'pinOff'} size={15} />
            </button>
            <button className="tb" title="Open Claude" onClick={() => void openClaude()}>
              <Glyph name="external" size={15} />
            </button>
            {pinned && (
              <button className="tb" title="Close" onClick={() => void closePanel()}>
                <Glyph name="close" size={15} />
              </button>
            )}
          </span>
        </div>
      </div>
      <div ref={body} className="body">
        <div>
          {!snap && !error && <div className="empty">loading …</div>}
          {error && <div className="empty error">{error}</div>}
          {isSettings && <SettingsView onDone={() => setIsSettings(false)} />}
          {snap && !isSettings && tab === 'open' && <RunCard items={snap.items.filter(i => i.stage === 'running')} now={now} />}
          {snap && !isSettings && tab === 'open' && <OpenTab snap={snap} now={now} />}
          {snap && !isSettings && tab === 'timeline' && <TimelineTab snap={snap} now={now} />}
          {snap && !isSettings && tab === 'history' && <HistoryTab version={snap.now} now={now} />}
        </div>
      </div>
      <div ref={foot} className="foot">
        <span>{snap ? `Updated ${clock(snap.now)}` : ''}</span>
        {snap && snap.problems.length > 0 && <span className="problem" title={snap.problems.join('\n')}>⚠ {snap.problems[0]}</span>}
        {snap && c && c.follow > 0 && tab !== 'open' && <span>{c.follow} later</span>}
      </div>
    </div>
  )
}
