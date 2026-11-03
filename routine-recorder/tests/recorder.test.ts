import { expect, mock, test } from 'claude-code/testing'
import type { On } from 'claude-code'

import type { RunRecord } from '../hooks/register'

const FILE = '/home/t/.claude/routine-recorder/runs/s1.json'

function world(on: On, answer: string) {
  const files = new Map<string, string>()
  mock.env(on, { HOME: '/home/t' })
  mock.clock(on, { now: 1000 })
  on('session.id', () => ({ value: 's1' }))
  on('session.cwd', () => ({ value: '/work' }))
  on('fs.exists', ($, e) => ({ value: files.has(e.path) }))
  on('fs.read', ($, e) => ({ value: files.get(e.path) ?? '' }))
  on('fs.write', ($, e) => {
    files.set(e.path, e.text)
    return { value: undefined }
  })
  const contexts: (readonly string[] | undefined)[] = []
  on('prompt.submit', ($, e) => {
    contexts.push(e.context)
    return { text: e.text }
  })
  on('turn.start', ($, e) => ({ turnId: e.turnId }))
  on('tool.check', () => ({ decision: 'ask' as const, reason: 'Bash needs approval' }))
  on('tool.call', () => ({ deny: "The user doesn't want to proceed with this tool use." }))
  on('turn.complete', () => ({ text: answer }))
  return { files, contexts }
}

const read = (files: Map<string, string>) => JSON.parse(files.get(FILE) ?? 'null') as RunRecord | null

test('a routine run is recorded with its denial, ping and STATUS line', async ($, on) => {
  const { files, contexts } = world(on, 'Konnte nicht.\nPING: Discord an das Team\nSTATUS: blocked | SSH-Tunnel abgelehnt.')

  await $.prompt.submit({
    text: '<scheduled-task name="weekly-report" file="/x/SKILL.md">\nRun it.',
    wait: false,
    origin: { kind: 'scheduled-trigger' },
  })
  await $.turn.start({ text: '', turnId: 't1' })
  await $.tool.check({ tool: 'Bash', input: { command: 'ssh host' }, tool_use_id: 'u1' })
  await $.tool.call({ tool: 'Bash', command: 'ssh host', tool_use_id: 'u1' })
  await $.turn.complete({ answer: '', durationMs: 5, isAborted: false, turnId: 't1', reason: 'answer' })

  const rec = read(files)
  expect(rec?.taskId).toBe('weekly-report')
  expect(rec?.activeTurnId).toBe(null)
  const turn = rec?.turns[0]
  expect(rec?.source).toBe('live')
  expect(turn?.by).toBe('routine')
  expect(turn?.trigger).toBe('scheduled')
  expect(turn?.answerTail).toContain('STATUS: blocked')
  expect(turn?.status).toBe('blocked')
  expect(turn?.statusText).toBe('SSH-Tunnel abgelehnt.')
  expect(turn?.tools['Bash']).toBe(1)
  expect(turn?.permissions[0]?.decision).toBe('ask')
  expect(turn?.permissions[0]?.outcome).toBe('denied')
  expect(turn?.pings[0]?.detail).toBe('Discord an das Team')

  expect(contexts[0]?.at(-1)).toContain('\nSTATUS: <idle|worked|waiting|blocked|parked>')
  expect(contexts[0]?.at(-1)).toContain('scheduled routines')
})

test('the person continuing in the run is recorded as a user turn', async ($, on) => {
  const { files, contexts } = world(on, 'Erledigt.\n\n[//]: # (PING: Push an mich)\n[//]: # (STATUS: waiting | Zwei Punkte noch offen.)')

  await $.prompt.submit({ text: '<scheduled-task name="ops">\nGo', wait: false, origin: { kind: 'scheduled-trigger' } })
  await $.turn.start({ text: '', turnId: 't1' })
  await $.turn.complete({ answer: '', durationMs: 5, isAborted: false, turnId: 't1', reason: 'answer' })
  await $.prompt.submit({ text: 'Ja, lade es.', wait: false, origin: { kind: 'composer' } })
  await $.turn.start({ text: 'Ja, lade es.', turnId: 't2' })
  await $.turn.complete({ answer: '', durationMs: 5, isAborted: false, turnId: 't2', reason: 'answer' })

  const rec = read(files)
  expect(rec?.turns.map(t => t.by)).toEqual(['routine', 'user'])
  expect(rec?.turns[1]?.trigger).toBe(undefined)
  expect(rec?.turns[1]?.status).toBe('waiting')
  expect(rec?.turns[1]?.statusText).toBe('Zwei Punkte noch offen.')
  expect(rec?.turns[1]?.pings[0]?.detail).toBe('Push an mich')
  expect(contexts[1]?.at(-1)).toContain('STATUS:')
})

test('an ordinary chat is not recorded but asked for its STATUS', async ($, on) => {
  const { files, contexts } = world(on, 'Hallo.')

  await $.prompt.submit({ text: 'Hallo', wait: false, origin: { kind: 'composer' } })
  await $.turn.start({ text: 'Hallo', turnId: 't1' })
  await $.turn.complete({ answer: '', durationMs: 5, isAborted: false, turnId: 't1', reason: 'answer' })

  expect(files.size).toBe(0)
  expect(contexts[0]?.at(-1)).toContain('\nSTATUS: <idle|worked|waiting|blocked|parked>')
  expect(contexts[0]?.at(-1)).not.toContain('scheduled routines')
  expect(contexts[0]?.at(-1)).not.toContain('PING')
})

test('a command the person runs from the chat is a round of theirs', async ($, on) => {
  const { files } = world(on, 'Bitte führ den Befehl selbst aus.\nSTATUS: blocked | Prüfung abgelehnt.')

  await $.prompt.submit({ text: '<scheduled-task name="ops">\nGo', wait: false, origin: { kind: 'scheduled-trigger' } })
  await $.turn.start({ text: '', turnId: 't1' })
  await $.tool.check({ tool: 'Bash', input: { command: 'check' }, tool_use_id: 'u1' })
  await $.tool.call({ tool: 'Bash', command: 'check', tool_use_id: 'u1' })
  await $.turn.complete({ answer: '', durationMs: 5, isAborted: false, turnId: 't1', reason: 'answer' })
  await $.prompt.submit({ text: '<bash-input>check</bash-input><bash-stdout>OK</bash-stdout>', wait: false, origin: { kind: 'composer' } })

  const rec = read(files)
  expect(rec?.turns.map(t => t.by)).toEqual(['routine', 'user'])
  expect(rec?.turns.at(-1)?.permissions).toEqual([])
})

test('a parked topic is recorded with what comes next', async ($, on) => {
  const { files } = world(on, 'Stand: Analyse halb fertig.\n\nSTATUS: parked | RAM-Messung steht, nächster Schritt: Profil des Workers.')

  await $.prompt.submit({ text: '<scheduled-task name="ops">\nGo', wait: false, origin: { kind: 'scheduled-trigger' } })
  await $.turn.start({ text: '', turnId: 't1' })
  await $.turn.complete({ answer: '', durationMs: 5, isAborted: false, turnId: 't1', reason: 'answer' })

  const turn = read(files)?.turns[0]
  expect(turn?.status).toBe('parked')
  expect(turn?.statusText).toBe('RAM-Messung steht, nächster Schritt: Profil des Workers.')
})
