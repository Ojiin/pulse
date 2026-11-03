import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'

import type { Entry, Settings, Snapshot } from './types'

// The Rust side works everything out and sends a new snapshot whenever the
// files change; the panel asks once when it starts.
export function useSnapshot() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    invoke<Snapshot>('snapshot').then(setSnapshot, e => setError(String(e)))
    const stop = listen<Snapshot>('snapshot', e => {
      setSnapshot(e.payload)
      setError(null)
    })
    return () => void stop.then(f => f())
  }, [])
  return { snapshot, error }
}

export const mark = (ids: string[], kind: 'seen' | 'done' | 'open' | 'park' | 'resume' | 'forget') => invoke('mark', { ids, kind })
export const openChat = (id: string) => invoke('open_chat', { id })
export const openClaude = () => invoke('open_claude')
export const setPinned = (pinned: boolean) => invoke<boolean>('set_pinned', { pinned })
export const closePanel = () => invoke('close_panel')
export const panelHeight = (height: number) => invoke('panel_height', { height })
export const markCommand = (key: string, known: boolean) => invoke('mark_command', { key, known })
export const getHistory = () => invoke<Entry[]>('history')
export const getSettings = () => invoke<{ settings: Settings; problem?: string }>('get_settings')
export const saveSettings = (settings: Settings) => invoke<Settings>('set_settings', { settings })
